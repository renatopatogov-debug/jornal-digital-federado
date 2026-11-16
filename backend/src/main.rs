use jornal_backend::{config::AppConfig, routes, worker};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
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

    // 4. Iniciar Worker em Segundo Plano do Hype Score e Higiene de Sessões (Ciclo de 5 min)
    worker::hype_worker::HypeWorker::spawn(pool.clone());

    // 5. Configurar Middlewares Globais (CORS com Credenciais, Headers, CSRF, Rate Limit, Limite de Buffer)
    let app = routes::build_app_router(pool, &app_config)
        .layer(axum::extract::DefaultBodyLimit::max(5 * 1024 * 1024));

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
