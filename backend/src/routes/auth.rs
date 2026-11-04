use crate::error::AppError;
use crate::services::session::SessionManager;
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::net::{IpAddr, Ipv4Addr};
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

pub async fn login(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    if payload.senha.trim().is_empty() {
        return Err(AppError::BadRequest("A senha não pode estar vazia.".to_string()));
    }

    // Busca usuário ativo no PostgreSQL 16
    let usuario = sqlx::query!(
        r#"
        SELECT id, senha_hash, totp_habilitado, status as "status: String"
        FROM usuarios
        WHERE email = $1
        "#,
        payload.email.trim()
    )
    .fetch_optional(&pool)
    .await?;

    let usuario = match usuario {
        Some(u) => u,
        None => return Err(AppError::Unauthorized("Credenciais inválidas.".to_string())),
    };

    if usuario.status != "ATIVO" {
        return Err(AppError::Forbidden(
            "Usuário suspenso ou pendente de ativação pelo líder diretorial.".to_string(),
        ));
    }

    // Se 2FA estiver ativo, requer etapa subsequente de desafio TOTP ou Passkey
    if usuario.totp_habilitado {
        return Ok((
            HeaderMap::new(),
            Json(LoginResponse {
                sucesso: true,
                requer_2fa: true,
                usuario_id: Some(usuario.id),
                mensagem: "Credencial validada. Confirme o segundo fator (Google Authenticator ou Passkey).".to_string(),
            }),
        ));
    }

    // Emissão de sessão HTTP-Only (ADR D3)
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("JornalClient/2.1");
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let token = SessionManager::criar_sessao(&pool, usuario.id, ip, user_agent).await?;

    let mut response_headers = HeaderMap::new();
    if let Ok(cookie_val) = HeaderValue::from_str(&format!(
        "__Host-jornal_session={}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=604800",
        token
    )) {
        response_headers.insert(header::SET_COOKIE, cookie_val);
    }

    Ok((
        response_headers,
        Json(LoginResponse {
            sucesso: true,
            requer_2fa: false,
            usuario_id: Some(usuario.id),
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
    if codigo.len() != 6 || !codigo.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::BadRequest(
            "O código TOTP deve conter exatamente 6 dígitos numéricos.".to_string(),
        ));
    }

    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("JornalClient/2.1");
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let token = SessionManager::criar_sessao(&pool, payload.usuario_id, ip, user_agent).await?;

    let mut response_headers = HeaderMap::new();
    if let Ok(cookie_val) = HeaderValue::from_str(&format!(
        "__Host-jornal_session={}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=604800",
        token
    )) {
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
    if let Some(cookie_val) = headers.get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_val.to_str() {
            for part in cookie_str.split(';') {
                let part = part.trim();
                if let Some(token) = part.strip_prefix("__Host-jornal_session=") {
                    let _ = SessionManager::revogar_sessao(&pool, token).await;
                }
            }
        }
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "__Host-jornal_session=; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=0",
        ),
    );

    Ok((
        response_headers,
        Json(json!({ "sucesso": true, "mensagem": "Sessão encerrada com sucesso." })),
    ))
}
