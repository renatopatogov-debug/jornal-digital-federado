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
    pub rate_limit_global_per_minute: u32,
    pub rate_limit_auth_per_minute: u32,
    pub feed_cache_ttl_seconds: u64,
    pub cors_allowed_origins: Vec<String>,
    pub cookie_secure: bool,
    pub cookie_same_site: String,
    pub trust_proxy: bool,
    pub aes_previous_master_key: Option<String>,
}

impl AppConfig {
    pub fn cookie_secure_from_env() -> bool {
        env::var("COOKIE_SECURE")
            .map(|v| !matches!(v.trim().to_ascii_lowercase().as_str(), "false" | "0" | "no"))
            .unwrap_or(true)
    }

    pub fn trust_proxy_from_env() -> bool {
        env::var("TRUST_PROXY")
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(false)
    }

    pub fn feed_cache_ttl_from_env() -> u64 {
        env::var("FEED_CACHE_TTL_SECONDS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(20)
    }

    pub fn cookie_same_site_from_env() -> String {
        match env::var("COOKIE_SAME_SITE")
            .unwrap_or_else(|_| "Strict".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "lax" => "Lax".to_string(),
            "none" => "None".to_string(),
            _ => "Strict".to_string(),
        }
    }

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

        let rate_limit_global_per_minute = env::var("RATE_LIMIT_REQUESTS_PER_MINUTE")
            .unwrap_or_else(|_| "120".to_string())
            .parse::<u32>()
            .unwrap_or(120)
            .max(1);

        let rate_limit_auth_per_minute = env::var("RATE_LIMIT_AUTH_POR_MINUTO")
            .unwrap_or_else(|_| "15".to_string())
            .parse::<u32>()
            .unwrap_or(15)
            .max(1);

        let feed_cache_ttl_seconds = Self::feed_cache_ttl_from_env();

        let cors_allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000,http://127.0.0.1:3000".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let cookie_secure = Self::cookie_secure_from_env();
        let cookie_same_site = Self::cookie_same_site_from_env();
        let trust_proxy = Self::trust_proxy_from_env();
        let aes_previous_master_key = env::var("AES_256_GCM_PREVIOUS_MASTER_KEY")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        Ok(Self {
            host,
            port,
            database_url,
            redis_url,
            database_max_connections,
            modo_eleitoral_default,
            max_ativacao_semanal_lider_municipal,
            rate_limit_votos_por_minuto,
            rate_limit_global_per_minute,
            rate_limit_auth_per_minute,
            feed_cache_ttl_seconds,
            cors_allowed_origins,
            cookie_secure,
            cookie_same_site,
            trust_proxy,
            aes_previous_master_key,
        })
    }
}
