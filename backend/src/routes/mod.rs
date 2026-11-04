pub mod auth;
pub mod feed;
pub mod health;
pub mod metrics;
pub mod noticias;
pub mod usuarios;
pub mod voto;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

pub fn build_api_router(pool: PgPool) -> Router {
    Router::new()
        // Health, Status & Observabilidade Prometheus
        .route("/health", get(health::health_check))
        .route("/metrics", get(metrics::prometheus_metrics))
        // Feed & Editorial Público
        .route("/api/v1/feed", get(feed::get_feed))
        // Autenticação, 2FA & Sessões HTTP-Only
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/2fa/verify", post(auth::verify_2fa))
        .route("/api/v1/auth/logout", post(auth::logout))
        // Governança RBAC & Ativações
        .route("/api/v1/usuarios/ativar", post(usuarios::ativar_filiado))
        .route("/api/v1/usuarios/revogar-lote", post(usuarios::revogar_lote))
        // Redação, Sanitização Ammonia & Escalonamento Editorial
        .route("/api/v1/noticias", post(noticias::criar_noticia))
        .route(
            "/api/v1/noticias/preview-sanitizado",
            post(noticias::preview_sanitizado),
        )
        .route(
            "/api/v1/noticias/:id/escalonar",
            post(noticias::solicitar_escalonamento),
        )
        // Ledger de Votação (ADRs D1 e D2)
        .route("/api/v1/noticias/:id/voto", post(voto::submeter_voto))
        .with_state(pool)
}
