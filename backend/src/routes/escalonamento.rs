use crate::error::AppError;
use crate::middleware::rbac::{extrair_ip_cliente, AuthContext};
use crate::models::{StatusEscalonamento, TipoEscopo};
use crate::services::session::{AuditoriaEvento, SessionManager};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ListarEscalonamentosQuery {
    pub status: Option<StatusEscalonamento>,
}

#[derive(Debug, Deserialize)]
pub struct AvaliarEscalonamentoRequest {
    pub aprovado: bool,
    pub parecer_revisao: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EscalonamentoItem {
    pub id: Uuid,
    pub noticia_id: Uuid,
    pub noticia_titulo: String,
    pub noticia_slug: String,
    pub noticia_ibge_codigo: Option<String>,
    pub solicitante_nome: String,
    pub escopo_origem: TipoEscopo,
    pub escopo_destino: TipoEscopo,
    pub status: StatusEscalonamento,
    pub justificativa: String,
    pub parecer_revisao: Option<String>,
    pub avaliado_por_nome: Option<String>,
    pub criado_em: String,
    pub avaliado_em: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListarEscalonamentosResponse {
    pub sucesso: bool,
    pub total_itens: usize,
    pub solicitacoes: Vec<EscalonamentoItem>,
}

#[derive(Debug, Serialize)]
pub struct AvaliarEscalonamentoResponse {
    pub sucesso: bool,
    pub solicitacao_id: Uuid,
    pub noticia_id: Uuid,
    pub status: StatusEscalonamento,
    pub novo_escopo: TipoEscopo,
    pub novo_ibge_codigo: Option<String>,
    pub mensagem: String,
}

#[derive(Debug, sqlx::FromRow)]
struct EscalonamentoRow {
    id: Uuid,
    noticia_id: Uuid,
    noticia_titulo: String,
    noticia_slug: String,
    noticia_ibge_codigo: Option<String>,
    solicitante_nome: String,
    escopo_origem: TipoEscopo,
    escopo_destino: TipoEscopo,
    status: StatusEscalonamento,
    justificativa: String,
    parecer_revisao: Option<String>,
    avaliado_por_nome: Option<String>,
    criado_em: DateTime<Utc>,
    avaliado_em: Option<DateTime<Utc>>,
}

pub async fn listar_escalonamentos(
    auth: AuthContext,
    State(pool): State<PgPool>,
    Query(query): Query<ListarEscalonamentosQuery>,
) -> Result<Json<ListarEscalonamentosResponse>, AppError> {
    let is_nacional = auth.escopo == TipoEscopo::Nacional;
    let prefixo_uf = match auth.escopo {
        TipoEscopo::Estadual => Some(format!(
            "{}%",
            auth.ibge_codigo.as_deref().unwrap_or("__")
        )),
        _ => None,
    };
    let ibge_municipal = match auth.escopo {
        TipoEscopo::Municipal => auth.ibge_codigo.clone(),
        _ => None,
    };

    let rows = sqlx::query_as::<_, EscalonamentoRow>(
        r#"
        SELECT se.id,
               se.noticia_id,
               n.titulo AS noticia_titulo,
               n.slug AS noticia_slug,
               n.ibge_codigo AS noticia_ibge_codigo,
               sol.nome AS solicitante_nome,
               se.escopo_origem,
               se.escopo_destino,
               se.status,
               se.justificativa,
               se.parecer_revisao,
               av.nome AS avaliado_por_nome,
               se.criado_em,
               se.avaliado_em
        FROM solicitacoes_escalonamento se
        JOIN noticias n ON se.noticia_id = n.id
        JOIN usuarios sol ON se.solicitante_id = sol.id
        LEFT JOIN usuarios av ON se.avaliado_por = av.id
        WHERE ($1::status_escalonamento IS NULL OR se.status = $1)
          AND (
              $2::bool = TRUE
              OR ($3::text IS NOT NULL AND n.ibge_codigo LIKE $3)
              OR ($4::text IS NOT NULL AND (n.ibge_codigo = $4 OR se.solicitante_id = $5))
          )
        ORDER BY
            CASE WHEN se.status = 'PENDENTE' THEN 0 ELSE 1 END,
            se.criado_em DESC
        LIMIT 50
        "#,
    )
    .bind(query.status)
    .bind(is_nacional)
    .bind(prefixo_uf.as_deref())
    .bind(ibge_municipal.as_deref())
    .bind(auth.usuario_id)
    .fetch_all(&pool)
    .await?;

    let solicitacoes: Vec<EscalonamentoItem> = rows
        .into_iter()
        .map(|r| EscalonamentoItem {
            id: r.id,
            noticia_id: r.noticia_id,
            noticia_titulo: r.noticia_titulo,
            noticia_slug: r.noticia_slug,
            noticia_ibge_codigo: r.noticia_ibge_codigo,
            solicitante_nome: r.solicitante_nome,
            escopo_origem: r.escopo_origem,
            escopo_destino: r.escopo_destino,
            status: r.status,
            justificativa: r.justificativa,
            parecer_revisao: r.parecer_revisao,
            avaliado_por_nome: r.avaliado_por_nome,
            criado_em: r.criado_em.to_rfc3339(),
            avaliado_em: r.avaliado_em.map(|d| d.to_rfc3339()),
        })
        .collect();

    Ok(Json(ListarEscalonamentosResponse {
        sucesso: true,
        total_itens: solicitacoes.len(),
        solicitacoes,
    }))
}

pub async fn avaliar_escalonamento(
    auth: AuthContext,
    headers: HeaderMap,
    State(pool): State<PgPool>,
    Path(solicitacao_id): Path<Uuid>,
    Json(payload): Json<AvaliarEscalonamentoRequest>,
) -> Result<Json<AvaliarEscalonamentoResponse>, AppError> {
    if !auth.cargo.pode_aprovar_escalonamento {
        return Err(AppError::Forbidden(
            "Cargo atual não possui permissão para aprovar ou rejeitar escalonamentos editoriais."
                .to_string(),
        ));
    }

    let parecer = payload
        .parecer_revisao
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let mut tx = pool.begin().await?;

    let row = sqlx::query(
        r#"
        SELECT se.noticia_id,
               se.escopo_origem,
               se.escopo_destino,
               se.status,
               n.escopo AS noticia_escopo_atual,
               n.ibge_codigo AS noticia_ibge
        FROM solicitacoes_escalonamento se
        JOIN noticias n ON se.noticia_id = n.id
        WHERE se.id = $1
        FOR UPDATE OF se, n
        "#,
    )
    .bind(solicitacao_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        let _ = tx.rollback().await;
        return Err(AppError::NotFound(
            "Solicitação de escalonamento não encontrada.".to_string(),
        ));
    };

    let noticia_id: Uuid = row.try_get("noticia_id")?;
    let escopo_origem: TipoEscopo = row.try_get("escopo_origem")?;
    let escopo_destino: TipoEscopo = row.try_get("escopo_destino")?;
    let status_atual: StatusEscalonamento = row.try_get("status")?;
    let noticia_escopo_atual: TipoEscopo = row.try_get("noticia_escopo_atual")?;
    let noticia_ibge: Option<String> = row.try_get("noticia_ibge")?;

    if status_atual != StatusEscalonamento::Pendente {
        let _ = tx.rollback().await;
        return Err(AppError::BadRequest(
            "Esta solicitação de escalonamento já foi avaliada anteriormente.".to_string(),
        ));
    }

    // Valida alçada territorial do avaliador sobre o escopo de destino (ADR D4)
    match escopo_destino {
        TipoEscopo::Nacional => {
            if auth.escopo != TipoEscopo::Nacional {
                let _ = tx.rollback().await;
                return Err(AppError::Forbidden(
                    "Apenas instâncias NACIONAIS podem aprovar promoção para o Feed Nacional."
                        .to_string(),
                ));
            }
        }
        TipoEscopo::Estadual => {
            let uf_destino = noticia_ibge.as_deref().and_then(|c| c.get(..2));
            if let Err(err) = auth.validar_jurisdicao_alvo(&TipoEscopo::Estadual, uf_destino) {
                let _ = tx.rollback().await;
                return Err(err);
            }
        }
        TipoEscopo::Municipal => {
            let _ = tx.rollback().await;
            return Err(AppError::BadRequest(
                "Escopo de destino inválido para escalonamento.".to_string(),
            ));
        }
    }

    let novo_status = if payload.aprovado {
        StatusEscalonamento::Aprovado
    } else {
        StatusEscalonamento::Rejeitado
    };

    sqlx::query(
        r#"
        UPDATE solicitacoes_escalonamento
        SET status = $1,
            avaliado_por = $2,
            parecer_revisao = $3,
            avaliado_em = NOW()
        WHERE id = $4
        "#,
    )
    .bind(&novo_status)
    .bind(auth.usuario_id)
    .bind(parecer.as_deref())
    .bind(solicitacao_id)
    .execute(&mut *tx)
    .await?;

    let (escopo_final, ibge_final) = if payload.aprovado {
        let ibge_promovido = match escopo_destino {
            TipoEscopo::Nacional => None,
            TipoEscopo::Estadual => noticia_ibge
                .as_deref()
                .and_then(|c| c.get(..2))
                .map(str::to_string),
            TipoEscopo::Municipal => noticia_ibge.clone(),
        };

        sqlx::query(
            r#"
            UPDATE noticias
            SET escopo = $1,
                ibge_codigo = $2,
                status = 'PUBLICADO',
                publicado_em = COALESCE(publicado_em, NOW())
            WHERE id = $3
            "#,
        )
        .bind(&escopo_destino)
        .bind(ibge_promovido.as_deref())
        .bind(noticia_id)
        .execute(&mut *tx)
        .await?;

        (escopo_destino.clone(), ibge_promovido)
    } else {
        (noticia_escopo_atual, noticia_ibge)
    };

    tx.commit().await?;

    if payload.aprovado {
        crate::services::redis_cache::RedisService::global()
            .invalidar_cache_feed()
            .await;
    }

    let ip = extrair_ip_cliente(&headers);
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok());
    let acao = if payload.aprovado {
        "ESCALONAMENTO_APROVADO"
    } else {
        "ESCALONAMENTO_REJEITADO"
    };
    let sid_str = solicitacao_id.to_string();
    SessionManager::registrar_auditoria(
        &pool,
        AuditoriaEvento {
            ator_id: Some(auth.usuario_id),
            acao,
            entidade: "solicitacoes_escalonamento",
            entidade_id: Some(&sid_str),
            dados_novos: json!({
                "noticia_id": noticia_id,
                "escopo_origem": escopo_origem,
                "escopo_destino": escopo_destino,
                "aprovado": payload.aprovado,
                "novo_ibge_codigo": ibge_final,
                "parecer_revisao": parecer
            }),
            ip_origem: ip,
            user_agent,
        },
    )
    .await;

    Ok(Json(AvaliarEscalonamentoResponse {
        sucesso: true,
        solicitacao_id,
        noticia_id,
        status: novo_status,
        novo_escopo: escopo_final,
        novo_ibge_codigo: ibge_final,
        mensagem: if payload.aprovado {
            "Escalonamento aprovado e escopo da matéria promovido com sucesso!".to_string()
        } else {
            "Solicitação de escalonamento rejeitada e registrada em auditoria.".to_string()
        },
    }))
}
