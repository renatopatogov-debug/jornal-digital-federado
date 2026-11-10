use std::env;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub database_max_connections: u32,
    pub modo_eleitoral_default: bool,
    pub max_ativacao_semanal_lider_municipal: i32,
    pub rate_limit_votos_por_minuto: i32,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|e| format!("PORT inválida: {}", e))?;

        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/jornal_federado".to_string()
        });

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let database_max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "20".to_string())
            .parse::<u32>()
            .unwrap_or(20);

        let modo_eleitoral_default = env::var("MODO_ELEITORAL_DEFAULT")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        let max_ativacao_semanal_lider_municipal = env::var("MAX_ATIVACAO_SEMANAL_LIDER_MUNICIPAL")
            .unwrap_or_else(|_| "50".to_string())
            .parse::<i32>()
            .unwrap_or(50);

        let rate_limit_votos_por_minuto = env::var("RATE_LIMIT_VOTOS_POR_MINUTO")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<i32>()
            .unwrap_or(10);

        Ok(Self {
            host,
            port,
            database_url,
            redis_url,
            database_max_connections,
            modo_eleitoral_default,
            max_ativacao_semanal_lider_municipal,
            rate_limit_votos_por_minuto,
        })
    }
}
