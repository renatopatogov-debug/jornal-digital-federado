pub mod auth;
pub mod feed;
pub mod health;
pub mod infra;
pub mod metrics;
pub mod noticias;
pub mod openapi;
pub mod usuarios;
pub mod voto;

use crate::config::AppConfig;
use crate::middleware::{
    rate_limit::{rate_limit_middleware, RateLimiter},
    security::{security_headers_middleware, SecurityConfig},
};
use axum::{
    http::{header, HeaderValue, Method},
    middleware,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn build_api_router(pool: PgPool) -> Router {
    Router::new()
        // Health, Status, Observabilidade Prometheus & OpenAPI 3.0.3
        .route("/health", get(health::health_check))
        .route("/metrics", get(metrics::prometheus_metrics))
        .route("/api/openapi.json", get(openapi::openapi_json))
        .route("/api/docs", get(openapi::openapi_docs))
        // Feed & Editorial Público
        .route("/api/v1/feed", get(feed::get_feed))
        // Autenticação, 2FA, Auditoria & Sessões HTTP-Only
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/2fa/verify", post(auth::verify_2fa))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/me", get(auth::me))
        .route("/api/v1/auth/sessoes", get(auth::listar_sessoes))
        .route(
            "/api/v1/auth/sessoes/revogar-todas",
            post(auth::revogar_todas_sessoes),
        )
        // Governança RBAC & Ativações
        .route("/api/v1/usuarios/ativar", post(usuarios::ativar_filiado))
        .route("/api/v1/usuarios/revogar-lote", post(usuarios::revogar_lote))
        // Infraestrutura & Rotação de Chaves AES-256-GCM
        .route(
            "/api/v1/infra/rotacionar-chaves-totp",
            post(infra::rotacionar_chaves_totp),
        )
        // Redação, Sanitização Ammonia & Escalonamento Editorial
        .route("/api/v1/noticias", post(noticias::criar_noticia))
        .route(
            "/api/v1/noticias/preview-sanitizado",
            post(noticias::preview_sanitizado),
        )
        .route(
            "/api/v1/noticias/:id",
            get(noticias::obter_noticia_por_slug),
        )
        .route(
            "/api/v1/noticias/:id/escalonar",
            post(noticias::solicitar_escalonamento),
        )
        // Ledger de Votação (ADRs D1 e D2)
        .route("/api/v1/noticias/:id/voto", post(voto::submeter_voto))
        .with_state(pool)
}

pub fn build_app_router(pool: PgPool, config: &AppConfig) -> Router {
    let allowed_origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
        .allow_credentials(true);

    let rate_limiter = RateLimiter::new(
        config.rate_limit_global_per_minute,
        config.rate_limit_auth_per_minute,
        config.trust_proxy,
    );
    let security_cfg = SecurityConfig::new(config.cors_allowed_origins.clone());

    build_api_router(pool)
        .layer(middleware::from_fn_with_state(
            rate_limiter,
            rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            security_cfg,
            security_headers_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

#[cfg(test)]
mod tests {
    use super::build_api_router;
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    async fn router_construido_sem_conflito_de_parametros() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/jornal_test")
            .expect("pool lazy válido");
        let _router = build_api_router(pool);
    }
}
