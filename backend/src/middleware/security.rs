use crate::middleware::rbac::extrair_token_cookie;
use axum::{
    extract::{Request, State},
    http::{header, HeaderName, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Clone, Debug)]
pub struct SecurityConfig {
    pub allowed_origins: Vec<String>,
}

impl SecurityConfig {
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self { allowed_origins }
    }

    fn is_origin_allowed(&self, origin: &str) -> bool {
        let trimmed = origin.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
            return false;
        }
        self.allowed_origins
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(trimmed))
    }
}

pub async fn security_headers_middleware(
    State(cfg): State<SecurityConfig>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let is_sensitive_path = path.starts_with("/api/v1/auth")
        || path.starts_with("/api/v1/usuarios")
        || path.starts_with("/api/v1/infra");

    // Proteção CSRF para requisições mutáveis autenticadas via cookie HTTP-Only
    let is_state_changing = !matches!(
        *req.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let origin_invalida = req
        .headers()
        .get(header::ORIGIN)
        .map(|h| !h.to_str().map(|o| cfg.is_origin_allowed(o)).unwrap_or(false))
        .unwrap_or(false);
    if is_state_changing && extrair_token_cookie(req.headers()).is_some() && origin_invalida {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "erro": true,
                "mensagem": "Bloqueio CSRF: origem não autorizada para requisição autenticada por cookie.",
                "status_code": 403
            })),
        )
            .into_response();
    }

    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    if is_sensitive_path {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
        );
        headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    }

    response
}
