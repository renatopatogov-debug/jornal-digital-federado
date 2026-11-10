use crate::error::AppError;
use crate::models::TipoEscopo;
use crate::services::hype::HypeEngine;
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SubmeterVotoRequest {
    pub usuario_id: Uuid,
    pub tipo_voto: i16,
}

#[derive(Debug, Serialize)]
pub struct VotoResponse {
    pub sucesso: bool,
    pub mensagem: String,
    pub noticia_id: Uuid,
    pub votos_positivos: i32,
    pub votos_negativos: i32,
    pub hype_score: f64,
}

pub async fn submeter_voto(
    State(pool): State<PgPool>,
    Path(noticia_id): Path<Uuid>,
    Json(payload): Json<SubmeterVotoRequest>,
) -> Result<Json<VotoResponse>, AppError> {
    if payload.tipo_voto != 1 && payload.tipo_voto != -1 {
        return Err(AppError::BadRequest(
            "O voto deve ser +1 (Hype) ou -1 (Rejeição).".to_string(),
        ));
    }

    // 1. Validar elegibilidade eleitoral do filiado (ADR D1)
    let row = sqlx::query(
        r#"
        SELECT status::text as status, filiado_verificado
        FROM usuarios
        WHERE id = $1
        "#,
    )
    .bind(payload.usuario_id)
    .fetch_optional(&pool)
    .await?;

    let row = match row {
        Some(u) => u,
        None => {
            return Err(AppError::Unauthorized("Usuário não encontrado.".to_string()));
        }
    };

    let status: String = row.try_get("status")?;
    let filiado_verificado: bool = row.try_get("filiado_verificado")?;

    if status != "ATIVO" || !filiado_verificado {
        return Err(AppError::Forbidden(
            "Votação exclusiva para filiados partidários formalmente verificados e ativos."
                .to_string(),
        ));
    }

    // 1.1 Rate Limit de 10 votos por minuto por filiado (ADR D2)
    let votos_ultimo_minuto: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM votos_noticias
        WHERE usuario_id = $1
          AND criado_em >= NOW() - INTERVAL '1 minute'
        "#,
    )
    .bind(payload.usuario_id)
    .fetch_one(&pool)
    .await?;

    if votos_ultimo_minuto >= 10 {
        return Err(AppError::RateLimited(
            "Limite de segurança atingido: máximo de 10 votos por minuto por filiado.".to_string(),
        ));
    }

    // 2. Persistir no Ledger Imutável com Transação Atômica
    let mut tx = pool.begin().await?;

    let insert_result = sqlx::query(
        r#"
        INSERT INTO votos_noticias (noticia_id, usuario_id, tipo_voto)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(noticia_id)
    .bind(payload.usuario_id)
    .bind(payload.tipo_voto)
    .execute(&mut *tx)
    .await;

    if let Err(e) = insert_result {
        let _ = tx.rollback().await;

        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                return Err(AppError::BadRequest(
                    "Voto já computado. Cada filiado possui direito estrito a 1 voto por pauta (ADR D1)."
                        .to_string(),
                ));
            }
        }
        return Err(AppError::Database(e));
    }

    // 3. Atualizar contadores agregados da notícia e recalcular Hype Score se qualificada (>= 10 votos)
    let (inc_pos, inc_neg) = if payload.tipo_voto == 1 {
        (1, 0)
    } else {
        (0, 1)
    };

    let noticia_row = sqlx::query(
        r#"
        UPDATE noticias
        SET votos_positivos = votos_positivos + $1,
            votos_negativos = votos_negativos + $2
        WHERE id = $3
        RETURNING votos_positivos, votos_negativos, escopo, publicado_em
        "#,
    )
    .bind(inc_pos)
    .bind(inc_neg)
    .bind(noticia_id)
    .fetch_one(&mut *tx)
    .await?;

    let votos_positivos: i32 = noticia_row.try_get("votos_positivos")?;
    let votos_negativos: i32 = noticia_row.try_get("votos_negativos")?;
    let escopo: TipoEscopo = noticia_row.try_get("escopo")?;
    let publicado_em: Option<DateTime<Utc>> = noticia_row.try_get("publicado_em")?;

    let novo_hype = HypeEngine::calcular_score(
        &escopo,
        votos_positivos,
        votos_negativos,
        publicado_em,
        Utc::now(),
    );

    sqlx::query(
        r#"
        UPDATE noticias
        SET hype_score = $1::float8::numeric
        WHERE id = $2
        "#,
    )
    .bind(novo_hype)
    .bind(noticia_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(VotoResponse {
        sucesso: true,
        mensagem: "Voto persistido de forma imutável no ledger eleitoral.".to_string(),
        noticia_id,
        votos_positivos,
        votos_negativos,
        hype_score: novo_hype,
    }))
}
