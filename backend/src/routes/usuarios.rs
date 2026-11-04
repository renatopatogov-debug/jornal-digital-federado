use crate::error::AppError;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AtivarFiliadoRequest {
    pub lider_id: Uuid,
    pub filiado_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RevogarLoteRequest {
    pub autor_id: Uuid,
    pub lider_dissidente_id: Uuid,
    pub motivo: String,
}

#[derive(Debug, Serialize)]
pub struct AcaoUsuarioResponse {
    pub sucesso: bool,
    pub mensagem: String,
    pub registros_afetados: u64,
}

pub async fn ativar_filiado(
    State(pool): State<PgPool>,
    Json(payload): Json<AtivarFiliadoRequest>,
) -> Result<Json<AcaoUsuarioResponse>, AppError> {
    // 1. Obter informações do líder ativador
    let lider = sqlx::query!(
        r#"
        SELECT u.id, u.cargo_id, c.slug, c.limite_ativacao_semanal
        FROM usuarios u
        JOIN cargos c ON u.cargo_id = c.id
        WHERE u.id = $1 AND u.status = 'ATIVO'
        "#,
        payload.lider_id
    )
    .fetch_optional(&pool)
    .await?;

    let lider = match lider {
        Some(l) => l,
        None => return Err(AppError::Unauthorized("Líder não encontrado ou inativo.".to_string())),
    };

    // 2. Teto operacional de 50 ativações para Líder Municipal (Seção 2.2 / Marco 0.2)
    if let Some(limite) = lider.limite_ativacao_semanal {
        let total_semana = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) FROM usuarios
            WHERE ativado_por = $1
              AND ativado_em >= NOW() - INTERVAL '7 days'
            "#,
            payload.lider_id
        )
        .fetch_one(&pool)
        .await?
        .unwrap_or(0);

        if total_semana >= limite as i64 {
            return Err(AppError::RateLimited(format!(
                "Cota semanal de {} ativações atingida para este diretório municipal.",
                limite
            )));
        }
    }

    // 3. Ativar filiado
    let res = sqlx::query!(
        r#"
        UPDATE usuarios
        SET status = 'ATIVO',
            filiado_verificado = TRUE,
            ativado_por = $1,
            ativado_em = NOW()
        WHERE id = $2
        "#,
        payload.lider_id,
        payload.filiado_id
    )
    .execute(&pool)
    .await?;

    Ok(Json(AcaoUsuarioResponse {
        sucesso: true,
        mensagem: "Filiado ativado e habilitado para votação no ranking Hype.".to_string(),
        registros_afetados: res.rows_affected(),
    }))
}

pub async fn revogar_lote(
    State(pool): State<PgPool>,
    Json(payload): Json<RevogarLoteRequest>,
) -> Result<Json<AcaoUsuarioResponse>, AppError> {
    // Kill-Switch de Dissidência Partidária (Seção 2.2 do Plano Diretor)
    let res = sqlx::query!(
        r#"
        UPDATE usuarios
        SET status = 'SUSPENSO'
        WHERE ativado_por = $1
        "#,
        payload.lider_dissidente_id
    )
    .execute(&pool)
    .await?;

    let afetados = res.rows_affected();

    // Invalidação em cascata das sessões ativas
    let _ = sqlx::query!(
        r#"
        DELETE FROM sessoes
        WHERE usuario_id IN (
            SELECT id FROM usuarios WHERE ativado_por = $1
        )
        "#,
        payload.lider_dissidente_id
    )
    .execute(&pool)
    .await;

    // Registro na trilha de auditoria
    let _ = sqlx::query!(
        r#"
        INSERT INTO auditoria_logs (ator_id, acao, entidade, entidade_id, dados_novos, ip_origem)
        VALUES ($1, 'REVOGACAO_LOTE_DISSIDENCIA', 'usuarios', $2, $3, '127.0.0.1'::inet)
        "#,
        payload.autor_id,
        payload.lider_dissidente_id.to_string(),
        json!({
            "motivo": payload.motivo,
            "usuarios_suspensos": afetados
        })
    )
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
