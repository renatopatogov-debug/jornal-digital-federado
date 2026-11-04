use crate::services::crypto::CryptoService;
use chrono::{Duration, Utc};
use rand::RngCore;
use sqlx::PgPool;
use std::net::IpAddr;
use uuid::Uuid;

pub struct SessionManager;

impl SessionManager {
    /// Gera um token de sessão de 256 bits com alta entropia
    pub fn gerar_token_sessao() -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    }

    /// Persiste a nova sessão no PostgreSQL 16 (tabela sessoes)
    pub async fn criar_sessao(
        pool: &PgPool,
        usuario_id: Uuid,
        ip_origem: IpAddr,
        user_agent: &str,
    ) -> Result<String, sqlx::Error> {
        let token = Self::gerar_token_sessao();
        let token_hash = CryptoService::hash_token_sha256(&token);
        let expira_em = Utc::now() + Duration::days(7);

        sqlx::query!(
            r#"
            INSERT INTO sessoes (token_hash, usuario_id, ip_origem, user_agent, expira_em)
            VALUES ($1, $2, $3::text::inet, $4, $5)
            "#,
            token_hash,
            usuario_id,
            ip_origem.to_string(),
            user_agent,
            expira_em
        )
        .execute(pool)
        .await?;

        Ok(token)
    }

    /// Invalida e remove a sessão (logout)
    pub async fn revogar_sessao(pool: &PgPool, token: &str) -> Result<(), sqlx::Error> {
        let token_hash = CryptoService::hash_token_sha256(token);
        sqlx::query!(
            r#"DELETE FROM sessoes WHERE token_hash = $1"#,
            token_hash
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

// Módulo auxiliar para codificação hexadecimal sem dependência externa adicional
mod hex {
    pub fn encode(data: [u8; 32]) -> String {
        data.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
