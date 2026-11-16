use crate::error::AppError;
use crate::middleware::rbac::{extrair_ip_cliente, AuthContext};
use crate::services::crypto::CryptoService;
use crate::services::session::{AuditoriaEvento, SessionManager};
use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

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
