use crate::models::{ModoEleitoralConfig, TipoEscopo};
use crate::services::hype::HypeEngine;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

pub struct HypeWorker;

impl HypeWorker {
    /// Inicia o worker em segundo plano com execução a cada 5 minutos (300 segundos)
    pub fn spawn(pool: PgPool) {
        tokio::spawn(async move {
            tracing::info!("[HypeWorker] Inicializado com ciclo de 300 segundos (5 minutos).");

            loop {
                if let Err(err) = Self::executar_ciclo(&pool).await {
                    tracing::error!("[HypeWorker] Falha durante o ciclo de recálculo: {:?}", err);
                }

                // Aguarda 5 minutos até a próxima rodada
                sleep(Duration::from_secs(300)).await;
            }
        });
    }

    async fn executar_ciclo(pool: &PgPool) -> Result<(), sqlx::Error> {
        let agora = Utc::now();

        // 1. Inspecionar status do Modo Eleitoral (ADR D6)
        let config_val: Option<serde_json::Value> = sqlx::query_scalar(
            r#"SELECT valor FROM configuracoes WHERE chave = 'modo_eleitoral'"#,
        )
        .fetch_optional(pool)
        .await?;

        let mut modo_eleitoral_ativo = false;
        if let Some(valor) = config_val {
            if let Ok(config) = serde_json::from_value::<ModoEleitoralConfig>(valor) {
                modo_eleitoral_ativo = config.ativo;
            }
        }

        // 2. Buscar notícias ativas dos últimos 7 dias
        let rows = sqlx::query(
            r#"
            SELECT id, escopo, votos_positivos, votos_negativos, publicado_em
            FROM noticias
            WHERE status = 'PUBLICADO'
              AND publicado_em >= NOW() - INTERVAL '7 days'
            "#,
        )
        .fetch_all(pool)
        .await?;

        if rows.is_empty() {
            return Ok(());
        }

        // 3. Recalcular e atualizar em lote
        let mut atualizadas = 0;
        for row in rows {
            let id: Uuid = row.try_get("id")?;
            let escopo: TipoEscopo = row.try_get("escopo")?;
            let votos_positivos: i32 = row.try_get("votos_positivos")?;
            let votos_negativos: i32 = row.try_get("votos_negativos")?;
            let publicado_em: Option<DateTime<Utc>> = row.try_get("publicado_em")?;

            let novo_score = HypeEngine::calcular_score(
                &escopo,
                votos_positivos,
                votos_negativos,
                publicado_em,
                agora,
            );

            sqlx::query(
                r#"
                UPDATE noticias
                SET hype_score = $1::float8::numeric
                WHERE id = $2
                "#,
            )
            .bind(novo_score)
            .bind(id)
            .execute(pool)
            .await?;

            atualizadas += 1;
        }

        // 4. Registro de auditoria caso o recálculo tenha ocorrido em modo silencioso
        if modo_eleitoral_ativo {
            let _ = sqlx::query(
                r#"
                INSERT INTO auditoria_logs (acao, entidade, dados_novos, ip_origem)
                VALUES ('RECALCULO_HYPE_SILENCIOSO', 'noticias', $1, '127.0.0.1'::inet)
                "#,
            )
            .bind(serde_json::json!({
                "modo_eleitoral": true,
                "materias_recalculadas": atualizadas,
                "motivo": "Manutencao da integridade matematica sem exposicao publica"
            }))
            .execute(pool)
            .await;

            tracing::info!(
                "[HypeWorker] Recálculo silencioso concluído: {} matérias atualizadas (Modo Eleitoral Ativo).",
                atualizadas
            );
        } else {
            tracing::info!(
                "[HypeWorker] Recálculo ordinário concluído: {} matérias atualizadas no ranking Hype.",
                atualizadas
            );
        }

        Ok(())
    }
}
