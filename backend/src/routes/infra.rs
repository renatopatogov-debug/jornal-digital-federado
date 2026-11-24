use crate::error::AppError;
use crate::middleware::rbac::{extrair_ip_cliente, AuthContext};
use crate::models::{ModoEleitoralConfig, TipoEscopo};
use crate::services::crypto::CryptoService;
use crate::services::session::{AuditoriaEvento, SessionManager};
use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const BANNER_ELEITORAL_PADRAO: &str = "MODO ELEITORAL ATIVO (Lei nº 9.504/97): Ordenação algorítmica Hype suspensa. Exibição estritamente cronológica.";

#[derive(Debug, Deserialize)]
pub struct AtualizarModoEleitoralRequest {
    pub ativo: bool,
    pub motivo: String,
    pub banner_texto: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ModoEleitoralResponse {
    pub sucesso: bool,
    pub config: ModoEleitoralConfig,
    pub atualizado_em: Option<String>,
    pub mensagem: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditoriaLogItem {
    pub id: i64,
    pub ator_id: Option<Uuid>,
    pub ator_nome: Option<String>,
    pub acao: String,
    pub entidade: String,
    pub entidade_id: Option<String>,
    pub dados_novos: Option<Value>,
    pub ip_origem: String,
    pub criado_em: String,
}

#[derive(Debug, Serialize)]
pub struct ListarAuditoriaResponse {
    pub sucesso: bool,
    pub visao_global: bool,
    pub total_itens: usize,
    pub logs: Vec<AuditoriaLogItem>,
}

#[derive(Debug, sqlx::FromRow)]
struct AuditoriaLogRow {
    id: i64,
    ator_id: Option<Uuid>,
    ator_nome: Option<String>,
    acao: String,
    entidade: String,
    entidade_id: Option<String>,
    dados_novos: Option<Value>,
    ip_origem: String,
    criado_em: DateTime<Utc>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RotacionarChavesRequest {
    pub chave_anterior: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RotacionarChavesResponse {
    pub sucesso: bool,
    pub registros_rotacionados: u64,
    pub registros_ja_atualizados: u64,
    pub mensagem: String,
}

/// Rotaciona os envelopes `totp_secret_cifrado` de `AES_256_GCM_PREVIOUS_MASTER_KEY`
/// para a chave ativa `AES_256_GCM_MASTER_KEY` em transação atômica.
pub async fn rotacionar_chaves_totp(
    auth: AuthContext,
    headers: HeaderMap,
    State(pool): State<PgPool>,
    body: Bytes,
) -> Result<Json<RotacionarChavesResponse>, AppError> {
    if !auth.cargo.pode_gerenciar_infra {
        return Err(AppError::Forbidden(
            "Apenas custódia de infraestrutura (SuperAdmin) pode rotacionar chaves AES-256-GCM."
                .to_string(),
        ));
    }

    let payload: RotacionarChavesRequest = if body.is_empty() {
        RotacionarChavesRequest::default()
    } else {
        serde_json::from_slice(&body).map_err(|_| {
            AppError::BadRequest("Corpo JSON inválido para rotação de chaves.".to_string())
        })?
    };

    let crypto_atual = CryptoService::from_env();
    let crypto_anterior = payload
        .chave_anterior
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(CryptoService::from_raw_secret)
        .or_else(CryptoService::previous_from_env);

    let mut tx = pool.begin().await?;
    let rows = sqlx::query(
        r#"
        SELECT id, totp_secret_cifrado
        FROM usuarios
        WHERE totp_secret_cifrado IS NOT NULL
        FOR UPDATE
        "#,
    )
    .fetch_all(&mut *tx)
    .await?;

    let mut rotacionados: u64 = 0;
    let mut ja_atualizados: u64 = 0;

    for row in rows {
        let uid: Uuid = row.try_get("id")?;
        let envelope: Vec<u8> = row.try_get("totp_secret_cifrado")?;

        let (plain, usou_anterior) = crypto_atual
            .decifrar_com_fallback(&envelope, crypto_anterior.as_ref())
            .map_err(|e| {
                AppError::BadRequest(format!(
                    "Falha ao decifrar envelope TOTP do usuário {}: {}",
                    uid, e
                ))
            })?;

        if usou_anterior {
            let novo_envelope = crypto_atual
                .cifrar_envelope_aleatorio(&plain)
                .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
            sqlx::query("UPDATE usuarios SET totp_secret_cifrado = $1 WHERE id = $2")
                .bind(novo_envelope)
                .bind(uid)
                .execute(&mut *tx)
                .await?;
            rotacionados += 1;
        } else {
            ja_atualizados += 1;
        }
    }

    tx.commit().await?;

    let ip = extrair_ip_cliente(&headers);
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok());
    SessionManager::registrar_auditoria(
        &pool,
        AuditoriaEvento {
            ator_id: Some(auth.usuario_id),
            acao: "ROTACAO_CHAVE_AES_TOTP",
            entidade: "usuarios",
            entidade_id: None,
            dados_novos: json!({
                "registros_rotacionados": rotacionados,
                "registros_ja_atualizados": ja_atualizados
            }),
            ip_origem: ip,
            user_agent,
        },
    )
    .await;

    Ok(Json(RotacionarChavesResponse {
        sucesso: true,
        registros_rotacionados: rotacionados,
        registros_ja_atualizados: ja_atualizados,
        mensagem: format!(
            "Rotação concluída: {} envelopes re-cifrados, {} já estavam na chave ativa.",
            rotacionados, ja_atualizados
        ),
    }))
}

/// Retorna a configuração vigente da feature flag `modo_eleitoral` (ADR D6)
pub async fn obter_modo_eleitoral(
    _auth: AuthContext,
    State(pool): State<PgPool>,
) -> Result<Json<ModoEleitoralResponse>, AppError> {
    let row = sqlx::query(
        "SELECT valor, atualizado_em FROM configuracoes WHERE chave = 'modo_eleitoral'",
    )
    .fetch_optional(&pool)
    .await?;

    let (config, atualizado_em) = if let Some(r) = row {
        let val: Value = r.try_get("valor")?;
        let ts: DateTime<Utc> = r.try_get("atualizado_em")?;
        let parsed = serde_json::from_value::<ModoEleitoralConfig>(val).unwrap_or(
            ModoEleitoralConfig {
                ativo: false,
                banner_obrigatorio: true,
                banner_texto: BANNER_ELEITORAL_PADRAO.to_string(),
                freeze_hype: false,
                motivo: "Operação ordinária.".to_string(),
            },
        );
        (parsed, Some(ts.to_rfc3339()))
    } else {
        (
            ModoEleitoralConfig {
                ativo: false,
                banner_obrigatorio: true,
                banner_texto: BANNER_ELEITORAL_PADRAO.to_string(),
                freeze_hype: false,
                motivo: "Operação ordinária.".to_string(),
            },
            None,
        )
    };

    Ok(Json(ModoEleitoralResponse {
        sucesso: true,
        config,
        atualizado_em,
        mensagem: None,
    }))
}

/// Ativa ou desativa o Modo Eleitoral (Lei nº 9.504/97 — ADR D6) com registro imutável em auditoria
pub async fn atualizar_modo_eleitoral(
    auth: AuthContext,
    headers: HeaderMap,
    State(pool): State<PgPool>,
    Json(payload): Json<AtualizarModoEleitoralRequest>,
) -> Result<Json<ModoEleitoralResponse>, AppError> {
    let autorizado = auth.cargo.pode_gerenciar_infra
        || (auth.cargo.pode_alterar_cargos && auth.escopo == TipoEscopo::Nacional);
    if !autorizado {
        return Err(AppError::Forbidden(
            "Apenas custódia de infraestrutura ou presidência nacional pode alternar o Modo Eleitoral."
                .to_string(),
        ));
    }

    let motivo = payload.motivo.trim();
    if motivo.len() < 5 {
        return Err(AppError::BadRequest(
            "Informe um motivo jurídico/operacional com pelo menos 5 caracteres.".to_string(),
        ));
    }

    let banner_texto = payload
        .banner_texto
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(BANNER_ELEITORAL_PADRAO)
        .to_string();

    let nova_config = ModoEleitoralConfig {
        ativo: payload.ativo,
        banner_obrigatorio: true,
        banner_texto,
        freeze_hype: payload.ativo,
        motivo: motivo.to_string(),
    };

    let valor_json = serde_json::to_value(&nova_config)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let atualizado_em: DateTime<Utc> = sqlx::query_scalar(
        r#"
        INSERT INTO configuracoes (chave, valor, descricao, modificado_por, atualizado_em)
        VALUES (
            'modo_eleitoral',
            $1,
            'Compliance Lei nº 9.504/97 — Congelamento de algoritmo de Hype e ordenação estritamente cronológica durante calendário eleitoral.',
            $2,
            NOW()
        )
        ON CONFLICT (chave) DO UPDATE
        SET valor = EXCLUDED.valor,
            modificado_por = EXCLUDED.modificado_por,
            atualizado_em = NOW()
        RETURNING atualizado_em
        "#,
    )
    .bind(&valor_json)
    .bind(auth.usuario_id)
    .fetch_one(&pool)
    .await?;

    let ip = extrair_ip_cliente(&headers);
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok());
    SessionManager::registrar_auditoria(
        &pool,
        AuditoriaEvento {
            ator_id: Some(auth.usuario_id),
            acao: "MODO_ELEITORAL_ALTERADO",
            entidade: "configuracoes",
            entidade_id: Some("modo_eleitoral"),
            dados_novos: valor_json,
            ip_origem: ip,
            user_agent,
        },
    )
    .await;

    crate::services::redis_cache::RedisService::global()
        .invalidar_cache_feed()
        .await;

    let status_txt = if nova_config.ativo {
        "ATIVADO (Freeze de Hype em vigor)"
    } else {
        "DESATIVADO (Ordenação gravitacional Hype restaurada)"
    };

    Ok(Json(ModoEleitoralResponse {
        sucesso: true,
        config: nova_config,
        atualizado_em: Some(atualizado_em.to_rfc3339()),
        mensagem: Some(format!("Modo Eleitoral {} com sucesso.", status_txt)),
    }))
}

/// Lista os eventos recentes de `auditoria_logs` (visão global para custódia nacional ou escopo do próprio ator)
pub async fn listar_auditoria(
    auth: AuthContext,
    State(pool): State<PgPool>,
) -> Result<Json<ListarAuditoriaResponse>, AppError> {
    let visao_global = auth.cargo.pode_gerenciar_infra
        || (auth.escopo == TipoEscopo::Nacional && auth.cargo.pode_revogar_lote);

    let rows = sqlx::query_as::<_, AuditoriaLogRow>(
        r#"
        SELECT a.id,
               a.ator_id,
               u.nome AS ator_nome,
               a.acao,
               a.entidade,
               a.entidade_id,
               a.dados_novos,
               host(a.ip_origem) AS ip_origem,
               a.criado_em
        FROM auditoria_logs a
        LEFT JOIN usuarios u ON a.ator_id = u.id
        WHERE ($1::bool = TRUE OR a.ator_id = $2)
        ORDER BY a.criado_em DESC
        LIMIT 50
        "#,
    )
    .bind(visao_global)
    .bind(auth.usuario_id)
    .fetch_all(&pool)
    .await?;

    let logs: Vec<AuditoriaLogItem> = rows
        .into_iter()
        .map(|r| AuditoriaLogItem {
            id: r.id,
            ator_id: r.ator_id,
            ator_nome: r.ator_nome,
            acao: r.acao,
            entidade: r.entidade,
            entidade_id: r.entidade_id,
            dados_novos: r.dados_novos,
            ip_origem: r.ip_origem,
            criado_em: r.criado_em.to_rfc3339(),
        })
        .collect();

    Ok(Json(ListarAuditoriaResponse {
        sucesso: true,
        visao_global,
        total_itens: logs.len(),
        logs,
    }))
}
