use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
};
use sqlx::PgPool;

/// Exportador de Métricas Prometheus da Core API do Jornal Digital Federado
/// Rota: GET /metrics
pub async fn prometheus_metrics(State(pool): State<PgPool>) -> impl IntoResponse {
    // 1. Contagem de notícias ativas e votos computados
    let total_noticias = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM noticias WHERE status = 'PUBLICADO'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);

    let total_votos = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM votos_noticias"#
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);

    // 2. Status do Modo Eleitoral (ADR D6)
    let modo_eleitoral_row = sqlx::query!(
        r#"SELECT valor FROM configuracoes WHERE chave = 'modo_eleitoral'"#
    )
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let mut modo_eleitoral_gauge = 0;
    if let Some(row) = modo_eleitoral_row {
        if let Some(ativo) = row.valor.get("ativo").and_then(|v| v.as_bool()) {
            if ativo {
                modo_eleitoral_gauge = 1;
            }
        }
    }

    // 3. Montagem do payload de métricas no formato padrão Prometheus (OpenMetrics)
    let metrics_payload = format!(
        "# HELP jornal_noticias_publicadas_total Total de materias publicadas no portal\n\
         # TYPE jornal_noticias_publicadas_total gauge\n\
         jornal_noticias_publicadas_total {}\n\n\
         # HELP jornal_ledger_votos_total Total de votos computados de forma imutavel no ledger\n\
         # TYPE jornal_ledger_votos_total counter\n\
         jornal_ledger_votos_total {}\n\n\
         # HELP jornal_modo_eleitoral_status Status do interruptor eleitoral (1 = Ativo / Lei 9.504/97, 0 = Inativo)\n\
         # TYPE jornal_modo_eleitoral_status gauge\n\
         jornal_modo_eleitoral_status {}\n\n\
         # HELP jornal_backend_uptime_status Status de saude do backend (1 = Saudavel)\n\
         # TYPE jornal_backend_uptime_status gauge\n\
         jornal_backend_uptime_status 1\n",
        total_noticias, total_votos, modo_eleitoral_gauge
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        metrics_payload,
    )
}
