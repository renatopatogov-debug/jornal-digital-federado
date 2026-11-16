use crate::services::crypto::CryptoService;
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::net::IpAddr;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct SessaoAtivaInfo {
    pub ip_origem: String,
    pub user_agent: String,
    pub criado_em: String,
    pub expira_em: String,
    pub ultimo_acesso: String,
    pub sessao_atual: bool,
}

pub struct AuditoriaEvento<'a> {
    pub ator_id: Option<Uuid>,
    pub acao: &'a str,
    pub entidade: &'a str,
    pub entidade_id: Option<&'a str>,
    pub dados_novos: Value,
    pub ip_origem: IpAddr,
    pub user_agent: Option<&'a str>,
}

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

        sqlx::query(
            r#"
            INSERT INTO sessoes (token_hash, usuario_id, ip_origem, user_agent, expira_em)
            VALUES ($1, $2, $3::text::inet, $4, $5)
            "#,
        )
        .bind(token_hash)
        .bind(usuario_id)
        .bind(ip_origem.to_string())
        .bind(user_agent)
        .bind(expira_em)
        .execute(pool)
        .await?;

        Ok(token)
    }

    /// Registra evento de auditoria com IP e User-Agent na tabela auditoria_logs
    pub async fn registrar_auditoria(pool: &PgPool, evento: AuditoriaEvento<'_>) {
        let _ = sqlx::query(
            r#"
            INSERT INTO auditoria_logs (
                ator_id, acao, entidade, entidade_id, dados_novos, ip_origem, user_agent
            )
            VALUES ($1, $2, $3, $4, $5, $6::text::inet, $7)
            "#,
        )
        .bind(evento.ator_id)
        .bind(evento.acao)
        .bind(evento.entidade)
        .bind(evento.entidade_id)
        .bind(evento.dados_novos)
        .bind(evento.ip_origem.to_string())
        .bind(evento.user_agent)
        .execute(pool)
        .await;
    }

    /// Lista sessões ativas de um usuário sem expor token_hash
    pub async fn listar_sessoes_usuario(
        pool: &PgPool,
        usuario_id: Uuid,
        token_atual: Option<&str>,
    ) -> Result<Vec<SessaoAtivaInfo>, sqlx::Error> {
        let token_atual_hash = token_atual.map(CryptoService::hash_token_sha256);

        let rows = sqlx::query(
            r#"
            SELECT token_hash,
                   host(ip_origem) AS ip_origem,
                   user_agent,
                   criado_em,
                   expira_em,
                   ultimo_acesso
            FROM sessoes
            WHERE usuario_id = $1
              AND expira_em > NOW()
            ORDER BY ultimo_acesso DESC
            "#,
        )
        .bind(usuario_id)
        .fetch_all(pool)
        .await?;

        let mut list = Vec::with_capacity(rows.len());
        for row in rows {
            let hash_row: String = row.try_get("token_hash")?;
            let ip_origem: String = row.try_get("ip_origem")?;
            let user_agent: String = row.try_get("user_agent")?;
            let criado_em: DateTime<Utc> = row.try_get("criado_em")?;
            let expira_em: DateTime<Utc> = row.try_get("expira_em")?;
            let ultimo_acesso: DateTime<Utc> = row.try_get("ultimo_acesso")?;

            let sessao_atual = token_atual_hash
                .as_deref()
                .map(|h| h == hash_row)
                .unwrap_or(false);

            list.push(SessaoAtivaInfo {
                ip_origem,
                user_agent,
                criado_em: criado_em.to_rfc3339(),
                expira_em: expira_em.to_rfc3339(),
                ultimo_acesso: ultimo_acesso.to_rfc3339(),
                sessao_atual,
            });
        }

        Ok(list)
    }

    /// Invalida todas as sessões de um usuário (opcionalmente preservando a sessão atual)
    pub async fn revogar_todas_sessoes(
        pool: &PgPool,
        usuario_id: Uuid,
        manter_token_atual: Option<&str>,
    ) -> Result<u64, sqlx::Error> {
        let res = if let Some(tok) = manter_token_atual {
            let hash = CryptoService::hash_token_sha256(tok);
            sqlx::query(r#"DELETE FROM sessoes WHERE usuario_id = $1 AND token_hash <> $2"#)
                .bind(usuario_id)
                .bind(hash)
                .execute(pool)
                .await?
        } else {
            sqlx::query(r#"DELETE FROM sessoes WHERE usuario_id = $1"#)
                .bind(usuario_id)
                .execute(pool)
                .await?
        };

        Ok(res.rows_affected())
    }

    /// Remove sessões expiradas da base (higiene periódica no worker)
    pub async fn limpar_sessoes_expiradas(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let res = sqlx::query(r#"DELETE FROM sessoes WHERE expira_em <= NOW()"#)
            .execute(pool)
            .await?;
        Ok(res.rows_affected())
    }

    /// Invalida e remove a sessão (logout)
    pub async fn revogar_sessao(pool: &PgPool, token: &str) -> Result<Option<Uuid>, sqlx::Error> {
        let token_hash = CryptoService::hash_token_sha256(token);
        let usuario_id: Option<Uuid> = sqlx::query_scalar(
            r#"DELETE FROM sessoes WHERE token_hash = $1 RETURNING usuario_id"#,
        )
        .bind(token_hash)
        .fetch_optional(pool)
        .await?;

        Ok(usuario_id)
    }
}

// Módulo auxiliar para codificação hexadecimal sem dependência externa adicional
mod hex {
    pub fn encode(data: [u8; 32]) -> String {
        data.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
