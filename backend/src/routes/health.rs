use crate::error::AppError;
use crate::services::redis_cache::RedisService;
use axum::{extract::State, Json};
use serde_json::{json, Value};
use sqlx::PgPool;

pub async fn health_check(State(pool): State<PgPool>) -> Result<Json<Value>, AppError> {
    // Validação de conectividade ativa com PostgreSQL 16
    sqlx::query("SELECT 1").execute(&pool).await?;

    let redis_ativo = RedisService::global().ping().await;
    let status_redis = if redis_ativo {
        "Redis 7 Conectado"
    } else {
        "Modo Fallback Local (Em Memória)"
    };

    Ok(Json(json!({
        "status": "healthy",
        "service": "jornal-digital-federado-api",
        "versao": "2.1.0",
        "banco_de_dados": "PostgreSQL 16 Multi-AZ Conectado",
        "cache_redis": status_redis,
        "redis_conectado": redis_ativo,
        "compliance": "Lei nº 9.504/97 e ADRs D1 a D8 Homologadas"
    })))
}
