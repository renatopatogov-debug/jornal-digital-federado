use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct PasskeyInfo {
    pub id: Uuid,
    pub nome_dispositivo: String,
    pub criado_em: String,
    pub ultimo_uso: Option<String>,
}

#[allow(dead_code)]
pub struct WebAuthnService;

#[allow(dead_code)]
impl WebAuthnService {
    /// Lista as Passkeys registradas para o usuário
    pub async fn listar_passkeys(
        pool: &PgPool,
        usuario_id: Uuid,
    ) -> Result<Vec<PasskeyInfo>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, nome_dispositivo, criado_em, ultimo_uso
            FROM webauthn_passkeys
            WHERE usuario_id = $1
            ORDER BY criado_em DESC
            "#,
        )
        .bind(usuario_id)
        .fetch_all(pool)
        .await?;

        let mut list = Vec::with_capacity(rows.len());
        for r in rows {
            let id: Uuid = r.try_get("id")?;
            let nome_dispositivo: String = r.try_get("nome_dispositivo")?;
            let criado_em: DateTime<Utc> = r.try_get("criado_em")?;
            let ultimo_uso: Option<DateTime<Utc>> = r.try_get("ultimo_uso")?;

            list.push(PasskeyInfo {
                id,
                nome_dispositivo,
                criado_em: criado_em.to_rfc3339(),
                ultimo_uso: ultimo_uso.map(|u| u.to_rfc3339()),
            });
        }

        Ok(list)
    }

    /// Atualiza o contador de assinatura e data de último uso após autenticação bem-sucedida
    pub async fn registrar_uso(
        pool: &PgPool,
        credencial_id: &[u8],
        novo_contador: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE webauthn_passkeys
            SET contador_assinatura = $1, ultimo_uso = NOW()
            WHERE credencial_id = $2
            "#,
        )
        .bind(novo_contador)
        .bind(credencial_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
