mod config;
mod error;
mod middleware;
mod models;
mod routes;
mod services;
mod worker;

use config::AppConfig;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inicializar Logging Estruturado
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,jornal_backend=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Carregar Configurações de Ambiente
    let app_config = AppConfig::from_env().expect("Falha ao carregar configurações do .env");
    tracing::info!("Inicializando Jornal Digital Federado — API Backend v2.1");

    // 3. Pool de Conexões com PostgreSQL 16
    let pool = PgPoolOptions::new()
        .max_connections(app_config.database_max_connections)
        .connect(&app_config.database_url)
        .await
        .unwrap_or_else(|err| {
            tracing::warn!(
                "Aviso: Não foi possível conectar ao PostgreSQL em {}: {}. Aguardando banco...",
                app_config.database_url,
                err
            );
            panic!("PostgreSQL 16 obrigatório para inicialização da API.");
        });

    tracing::info!("Conexão com PostgreSQL 16 estabelecida com sucesso.");

    // 4. Iniciar Worker em Segundo Plano do Hype Score (Ciclo de 5 min)
    worker::hype_worker::HypeWorker::spawn(pool.clone());

    // 5. Configurar Middlewares Globais (CORS, Trace, Limite Estrito de Buffer)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = routes::build_api_router(pool)
        .layer(axum::extract::DefaultBodyLimit::max(5 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    // 6. Iniciar Servidor HTTP Axum
    let addr = SocketAddr::new(
        app_config.host.parse().expect("Host inválido"),
        app_config.port,
    );

    tracing::info!("Servidor escutando em http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
