use crate::config::AppConfig;
use crate::error::AppError;
use crate::middleware::rbac::{extrair_ip_cliente, extrair_token_cookie};
use crate::services::crypto::CryptoService;
use crate::services::session::SessionManager;
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub senha: String,
}

#[derive(Debug, Deserialize)]
pub struct Verify2faRequest {
    pub usuario_id: Uuid,
    pub codigo_totp: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub sucesso: bool,
    pub requer_2fa: bool,
    pub usuario_id: Option<Uuid>,
    pub mensagem: String,
}

fn construir_cookie_sessao(token: &str, max_age_secs: i64) -> Option<HeaderValue> {
    let cookie = if AppConfig::cookie_secure_from_env() {
        format!(
            "__Host-jornal_session={}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age={}",
            token, max_age_secs
        )
    } else {
        format!(
            "jornal_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
            token, max_age_secs
        )
    };
    HeaderValue::from_str(&cookie).ok()
}

pub async fn login(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    if payload.email.trim().is_empty() || payload.senha.trim().is_empty() {
        return Err(AppError::BadRequest(
            "E-mail e senha são obrigatórios.".to_string(),
        ));
    }

    // Busca usuário e valida senha_hash em tempo constante
    let row = sqlx::query(
        r#"
        SELECT id, senha_hash, totp_habilitado, status::text as status
        FROM usuarios
        WHERE LOWER(email) = LOWER($1)
        "#,
    )
    .bind(payload.email.trim())
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Credenciais inválidas.".to_string()))?;

    let usuario_id: Uuid = row.try_get("id")?;
    let senha_hash: String = row.try_get("senha_hash")?;
    let totp_habilitado: bool = row.try_get("totp_habilitado")?;
    let status: String = row.try_get("status")?;

    if !CryptoService::verificar_senha_hash(&payload.senha, &senha_hash) {
        return Err(AppError::Unauthorized("Credenciais inválidas.".to_string()));
    }

    if status != "ATIVO" {
        return Err(AppError::Forbidden(
            "Usuário suspenso ou pendente de ativação pelo líder diretorial.".to_string(),
        ));
    }

    // Se 2FA estiver ativo, requer etapa subsequente de desafio TOTP ou código de recuperação
    if totp_habilitado {
        return Ok((
            HeaderMap::new(),
            Json(LoginResponse {
                sucesso: true,
                requer_2fa: true,
                usuario_id: Some(usuario_id),
                mensagem:
                    "Credencial validada. Confirme o segundo fator (TOTP ou código de recuperação)."
                        .to_string(),
            }),
        ));
    }

    // Emissão de sessão HTTP-Only (ADR D3)
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("JornalClient/2.1");
    let ip = extrair_ip_cliente(&headers);
    let token = SessionManager::criar_sessao(&pool, usuario_id, ip, user_agent).await?;

    let mut response_headers = HeaderMap::new();
    if let Some(cookie_val) = construir_cookie_sessao(&token, 604_800) {
        response_headers.insert(header::SET_COOKIE, cookie_val);
    }

    Ok((
        response_headers,
        Json(LoginResponse {
            sucesso: true,
            requer_2fa: false,
            usuario_id: Some(usuario_id),
            mensagem: "Login realizado e sessão HTTP-Only emitida com sucesso.".to_string(),
        }),
    ))
}

pub async fn verify_2fa(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<Verify2faRequest>,
) -> Result<impl IntoResponse, AppError> {
    let codigo = payload.codigo_totp.trim();
    if codigo.is_empty() {
        return Err(AppError::BadRequest(
            "Informe o código TOTP de 6 dígitos ou código de recuperação.".to_string(),
        ));
    }

    let row = sqlx::query(
        r#"
        SELECT totp_habilitado,
               totp_secret_cifrado,
               COALESCE(recovery_codes_hashes, '{}') as recovery_codes_hashes,
               status::text as status
        FROM usuarios
        WHERE id = $1
        "#,
    )
    .bind(payload.usuario_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Usuário não encontrado.".to_string()))?;

    let status: String = row.try_get("status")?;
    let totp_habilitado: bool = row.try_get("totp_habilitado")?;
    let totp_secret_cifrado: Option<Vec<u8>> = row.try_get("totp_secret_cifrado")?;
    let recovery_codes_hashes: Vec<String> = row.try_get("recovery_codes_hashes")?;

    if status != "ATIVO" {
        return Err(AppError::Forbidden(
            "Usuário suspenso ou pendente de ativação.".to_string(),
        ));
    }
    if !totp_habilitado {
        return Err(AppError::BadRequest(
            "Segundo fator 2FA não está habilitado para esta conta.".to_string(),
        ));
    }

    let mut verificado = false;

    if let Some(envelope) = totp_secret_cifrado {
        if codigo.len() == 6 && codigo.chars().all(|c| c.is_ascii_digit()) {
            let crypto = CryptoService::from_env();
            let secret = crypto
                .decifrar_envelope(&envelope)
                .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
            let agora = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
                .as_secs();
            if CryptoService::verificar_totp_sha256(&secret, codigo, agora) {
                verificado = true;
            }
        }
    }

    if !verificado && !recovery_codes_hashes.is_empty() {
        let codigo_hash = CryptoService::hash_token_sha256(codigo);
        if recovery_codes_hashes.iter().any(|h| h == &codigo_hash) {
            sqlx::query(
                r#"
                UPDATE usuarios
                SET recovery_codes_hashes = array_remove(recovery_codes_hashes, $1)
                WHERE id = $2
                "#,
            )
            .bind(&codigo_hash)
            .bind(payload.usuario_id)
            .execute(&pool)
            .await?;
            verificado = true;
        }
    }

    if !verificado {
        return Err(AppError::Unauthorized(
            "Código TOTP ou código de recuperação inválido.".to_string(),
        ));
    }

    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("JornalClient/2.1");
    let ip = extrair_ip_cliente(&headers);
    let token = SessionManager::criar_sessao(&pool, payload.usuario_id, ip, user_agent).await?;

    let mut response_headers = HeaderMap::new();
    if let Some(cookie_val) = construir_cookie_sessao(&token, 604_800) {
        response_headers.insert(header::SET_COOKIE, cookie_val);
    }

    Ok((
        response_headers,
        Json(json!({
            "sucesso": true,
            "usuario_id": payload.usuario_id,
            "mensagem": "Segundo fator autenticado e cookie HTTP-Only emitido."
        })),
    ))
}

pub async fn logout(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    if let Some(token) = extrair_token_cookie(&headers) {
        let _ = SessionManager::revogar_sessao(&pool, &token).await;
    }

    let mut response_headers = HeaderMap::new();
    if let Some(cookie_val) = construir_cookie_sessao("", 0) {
        response_headers.insert(header::SET_COOKIE, cookie_val);
    }

    Ok((
        response_headers,
        Json(json!({ "sucesso": true, "mensagem": "Sessão encerrada com sucesso." })),
    ))
}
