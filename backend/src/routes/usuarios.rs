use crate::error::AppError;
use crate::middleware::rbac::{extrair_ip_cliente, AuthContext};
use crate::models::TipoEscopo;
use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AtivarFiliadoRequest {
    pub filiado_id: Option<Uuid>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RevogarLoteRequest {
    pub lider_dissidente_id: Option<Uuid>,
    pub lider_dissidente_email: Option<String>,
    pub motivo: String,
}

#[derive(Debug, Serialize)]
pub struct AcaoUsuarioResponse {
    pub sucesso: bool,
    pub mensagem: String,
    pub registros_afetados: u64,
}

#[derive(sqlx::FromRow)]
struct UsuarioAlvoRow {
    id: Uuid,
    escopo: TipoEscopo,
    ibge_codigo: Option<String>,
}

async fn resolver_usuario_alvo(
    pool: &PgPool,
    id: Option<Uuid>,
    email: Option<&str>,
    mensagem_ausente: &str,
) -> Result<UsuarioAlvoRow, AppError> {
    if let Some(uid) = id {
        sqlx::query_as::<_, UsuarioAlvoRow>(
            "SELECT id, escopo, ibge_codigo FROM usuarios WHERE id = $1",
        )
        .bind(uid)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(mensagem_ausente.to_string()))
    } else if let Some(mail) = email.map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query_as::<_, UsuarioAlvoRow>(
            "SELECT id, escopo, ibge_codigo FROM usuarios WHERE LOWER(email) = LOWER($1)",
        )
        .bind(mail)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(mensagem_ausente.to_string()))
    } else {
        Err(AppError::BadRequest(
            "Informe o UUID ou o e-mail do usuário alvo.".to_string(),
        ))
    }
}

pub async fn ativar_filiado(
    auth: AuthContext,
    headers: HeaderMap,
    State(pool): State<PgPool>,
    Json(payload): Json<AtivarFiliadoRequest>,
) -> Result<Json<AcaoUsuarioResponse>, AppError> {
    if !auth.cargo.pode_ativar_filiados {
        return Err(AppError::Forbidden(
            "Cargo atual não possui permissão para ativar filiados.".to_string(),
        ));
    }

    // 1. Teto operacional de ativações semanais para liderança municipal (Marco 0.2)
    if let Some(limite) = auth.cargo.limite_ativacao_semanal {
        let total_semana: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM usuarios
            WHERE ativado_por = $1
              AND ativado_em >= NOW() - INTERVAL '7 days'
            "#,
        )
        .bind(auth.usuario_id)
        .fetch_one(&pool)
        .await?;

        if total_semana >= i64::from(limite) {
            return Err(AppError::RateLimited(format!(
                "Cota semanal de {} ativações atingida para este diretório.",
                limite
            )));
        }
    }

    // 2. Resolver alvo e validar isolamento territorial (ADR D4)
    let alvo = resolver_usuario_alvo(
        &pool,
        payload.filiado_id,
        payload.email.as_deref(),
        "Nenhum filiado encontrado para o identificador informado.",
    )
    .await?;

    auth.validar_jurisdicao_alvo(&alvo.escopo, alvo.ibge_codigo.as_deref())?;

    // 3. Ativar filiado
    let res = sqlx::query(
        r#"
        UPDATE usuarios
        SET status = 'ATIVO',
            filiado_verificado = TRUE,
            ativado_por = $1,
            ativado_em = NOW()
        WHERE id = $2
        "#,
    )
    .bind(auth.usuario_id)
    .bind(alvo.id)
    .execute(&pool)
    .await?;

    let ip_origem = extrair_ip_cliente(&headers).to_string();
    let _ = sqlx::query(
        r#"
        INSERT INTO auditoria_logs (ator_id, acao, entidade, entidade_id, dados_novos, ip_origem)
        VALUES ($1, 'ATIVACAO_FILIADO', 'usuarios', $2, $3, $4::inet)
        "#,
    )
    .bind(auth.usuario_id)
    .bind(alvo.id.to_string())
    .bind(json!({ "escopo": alvo.escopo, "ibge_codigo": alvo.ibge_codigo }))
    .bind(&ip_origem)
    .execute(&pool)
    .await;

    Ok(Json(AcaoUsuarioResponse {
        sucesso: true,
        mensagem: "Filiado ativado e habilitado para votação no ranking Hype.".to_string(),
        registros_afetados: res.rows_affected(),
    }))
}

pub async fn revogar_lote(
    auth: AuthContext,
    headers: HeaderMap,
    State(pool): State<PgPool>,
    Json(payload): Json<RevogarLoteRequest>,
) -> Result<Json<AcaoUsuarioResponse>, AppError> {
    if !auth.cargo.pode_revogar_lote {
        return Err(AppError::Forbidden(
            "Cargo atual não possui permissão para executar Kill-Switch em lote.".to_string(),
        ));
    }

    if payload.motivo.trim().len() < 5 {
        return Err(AppError::BadRequest(
            "Informe uma justificativa formal com ao menos 5 caracteres.".to_string(),
        ));
    }

    let alvo = resolver_usuario_alvo(
        &pool,
        payload.lider_dissidente_id,
        payload.lider_dissidente_email.as_deref(),
        "Liderança dissidente não encontrada para o identificador informado.",
    )
    .await?;

    if alvo.id == auth.usuario_id {
        return Err(AppError::BadRequest(
            "Operação bloqueada: não é permitido revogar a própria conta autenticada."
                .to_string(),
        ));
    }

    auth.validar_jurisdicao_alvo(&alvo.escopo, alvo.ibge_codigo.as_deref())?;

    // Kill-Switch de Dissidência Partidária (Seção 2.2 do Plano Diretor)
    let res = sqlx::query(
        r#"
        UPDATE usuarios
        SET status = 'SUSPENSO'
        WHERE id = $1 OR ativado_por = $1
        "#,
    )
    .bind(alvo.id)
    .execute(&pool)
    .await?;

    let afetados = res.rows_affected();

    // Invalidação em cascata das sessões ativas
    let _ = sqlx::query(
        r#"
        DELETE FROM sessoes
        WHERE usuario_id = $1
           OR usuario_id IN (SELECT id FROM usuarios WHERE ativado_por = $1)
        "#,
    )
    .bind(alvo.id)
    .execute(&pool)
    .await;

    let ip_origem = extrair_ip_cliente(&headers).to_string();
    let _ = sqlx::query(
        r#"
        INSERT INTO auditoria_logs (ator_id, acao, entidade, entidade_id, dados_novos, ip_origem)
        VALUES ($1, 'REVOGACAO_LOTE_DISSIDENCIA', 'usuarios', $2, $3, $4::inet)
        "#,
    )
    .bind(auth.usuario_id)
    .bind(alvo.id.to_string())
    .bind(json!({
        "motivo": payload.motivo.trim(),
        "usuarios_suspensos": afetados
    }))
    .bind(&ip_origem)
    .execute(&pool)
    .await;

    Ok(Json(AcaoUsuarioResponse {
        sucesso: true,
        mensagem: format!(
            "Kill-Switch executado: {} usuários suspensos e sessões expurgadas com sucesso.",
            afetados
        ),
        registros_afetados: afetados,
    }))
}
