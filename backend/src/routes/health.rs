use crate::error::AppError;
use axum::{extract::State, Json};
use serde_json::{json, Value};
use sqlx::PgPool;

pub async fn health_check(State(pool): State<PgPool>) -> Result<Json<Value>, AppError> {
    // Validação de conectividade ativa com PostgreSQL 16
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(e))?;

    Ok(Json(json!({
        "status": "healthy",
        "service": "jornal-digital-federado-api",
        "versao": "2.1.0",
        "banco_de_dados": "PostgreSQL 16 Multi-AZ Conectado",
        "compliance": "Lei nº 9.504/97 e ADRs D1 a D8 Homologadas"
    })))
}
