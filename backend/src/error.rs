use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Não autorizado: {0}")]
    Unauthorized(String),

    #[error("Acesso proibido (RBAC Territorial): {0}")]
    Forbidden(String),

    #[error("Recurso não encontrado: {0}")]
    NotFound(String),

    #[error("Requisição inválida: {0}")]
    BadRequest(String),

    #[error("Limite de taxa excedido: {0}")]
    RateLimited(String),

    #[error("Erro interno do servidor")]
    Internal(#[from] anyhow::Error),

    #[error("Erro de banco de dados: {0}")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, mensagem) = match &self {
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::RateLimited(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.clone()),
            AppError::Database(err) => {
                tracing::error!("Erro de banco de dados: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Falha ao processar requisição no banco de dados.".to_string(),
                )
            }
            AppError::Internal(err) => {
                tracing::error!("Erro interno: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Erro interno no servidor.".to_string(),
                )
            }
        };

        let body = Json(json!({
            "erro": true,
            "mensagem": mensagem,
            "status_code": status.as_u16()
        }));

        (status, body).into_response()
    }
}
