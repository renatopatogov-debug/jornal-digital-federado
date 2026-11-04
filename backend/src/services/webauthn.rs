use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct PasskeyInfo {
    pub id: Uuid,
    pub nome_dispositivo: String,
    pub criado_em: String,
    pub ultimo_uso: Option<String>,
}

pub struct WebAuthnService;

impl WebAuthnService {
    /// Lista as Passkeys registradas para o usuário
    pub async fn listar_passkeys(pool: &PgPool, usuario_id: Uuid) -> Result<Vec<PasskeyInfo>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"
            SELECT id, nome_dispositivo, criado_em, ultimo_uso
            FROM webauthn_passkeys
            WHERE usuario_id = $1
            ORDER BY criado_em DESC
            "#,
            usuario_id
        )
        .fetch_all(pool)
        .await?;

        let list = rows
            .into_iter()
            .map(|r| PasskeyInfo {
                id: r.id,
                nome_dispositivo: r.nome_dispositivo,
                criado_em: r.criado_em.to_rfc3339(),
                ultimo_uso: r.ultimo_uso.map(|u| u.to_rfc3339()),
            })
            .collect();

        Ok(list)
    }

    /// Atualiza o contador de assinatura e data de último uso após autenticação bem-sucedida
    pub async fn registrar_uso(
        pool: &PgPool,
        credencial_id: &[u8],
        novo_contador: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE webauthn_passkeys
            SET contador_assinatura = $1, ultimo_uso = NOW()
            WHERE credencial_id = $2
            "#,
            novo_contador,
            credencial_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
