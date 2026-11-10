use crate::error::AppError;
use crate::models::{ModoEleitoralConfig, Noticia, TipoEscopo, TipoFormatoMateria};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
pub struct FeedQueryParams {
    pub uf: Option<String>,
    pub ibge: Option<String>,
    pub q: Option<String>,
    pub formato: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct FeedResponse {
    pub modo_eleitoral_ativo: bool,
    pub banner_legal: Option<String>,
    pub total_itens: usize,
    pub noticias: Vec<NoticiaFeedItem>,
}

#[derive(Debug, Serialize)]
pub struct NoticiaFeedItem {
    pub id: String,
    pub titulo: String,
    pub slug: String,
    pub subtitulo: Option<String>,
    pub escopo: TipoEscopo,
    pub ibge_codigo: Option<String>,
    pub formato: TipoFormatoMateria,
    pub video_url: Option<String>,
    pub audio_url: Option<String>,
    pub pdf_edicao_url: Option<String>,
    pub conteudo_sanitizado: String,
    pub imagem_destaque_url: Option<String>,
    pub publicado_em: Option<String>,
    pub votos_positivos: i32,
    pub votos_negativos: i32,
    pub hype_score: Option<f64>,
}

pub async fn get_feed(
    State(pool): State<PgPool>,
    Query(params): Query<FeedQueryParams>,
) -> Result<Json<FeedResponse>, AppError> {
    // 1. Inspecionar Contrato Operacional do Modo Eleitoral (ADR D6)
    let config_val: Option<serde_json::Value> = sqlx::query_scalar(
        r#"SELECT valor FROM configuracoes WHERE chave = 'modo_eleitoral'"#,
    )
    .fetch_optional(&pool)
    .await?;

    let mut modo_eleitoral_ativo = false;
    let mut banner_texto: Option<String> = None;

    if let Some(valor) = config_val {
        if let Ok(config) = serde_json::from_value::<ModoEleitoralConfig>(valor) {
            modo_eleitoral_ativo = config.ativo;
            if config.ativo && config.banner_obrigatorio {
                banner_texto = Some(config.banner_texto);
            }
        }
    }

    let limit = params.limit.unwrap_or(20).clamp(1, 50);
    let offset = params.offset.unwrap_or(0).max(0);
    let termo_busca = params
        .q
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s));

    // 2. Consulta parametrizada com filtros combinados (IBGE, UF, Busca Textual e Formato)
    let query_str = if modo_eleitoral_ativo {
        r#"
        SELECT n.id, n.titulo, n.slug, n.subtitulo, n.autor_id, n.escopo,
               n.ibge_codigo, n.formato, n.video_url, n.audio_url,
               n.audio_duracao_segundos, n.pdf_edicao_url, n.pdf_numero_paginas,
               n.conteudo_sanitizado, n.imagem_destaque_url,
               n.status, n.votos_positivos, n.votos_negativos,
               n.hype_score::float8 as hype_score, n.publicado_em, n.criado_em
        FROM noticias n
        LEFT JOIN territorios t ON n.ibge_codigo = t.codigo_ibge
        WHERE n.status = 'PUBLICADO'
          AND ($1::text IS NULL OR n.ibge_codigo = $1)
          AND ($2::text IS NULL OR UPPER(t.uf) = UPPER($2))
          AND ($3::text IS NULL OR n.titulo ILIKE $3 OR COALESCE(n.subtitulo, '') ILIKE $3)
          AND ($4::text IS NULL OR n.formato::text = UPPER($4))
        ORDER BY n.publicado_em DESC NULLS LAST, n.criado_em DESC
        LIMIT $5 OFFSET $6
        "#
    } else {
        r#"
        SELECT n.id, n.titulo, n.slug, n.subtitulo, n.autor_id, n.escopo,
               n.ibge_codigo, n.formato, n.video_url, n.audio_url,
               n.audio_duracao_segundos, n.pdf_edicao_url, n.pdf_numero_paginas,
               n.conteudo_sanitizado, n.imagem_destaque_url,
               n.status, n.votos_positivos, n.votos_negativos,
               n.hype_score::float8 as hype_score, n.publicado_em, n.criado_em
        FROM noticias n
        LEFT JOIN territorios t ON n.ibge_codigo = t.codigo_ibge
        WHERE n.status = 'PUBLICADO'
          AND ($1::text IS NULL OR n.ibge_codigo = $1)
          AND ($2::text IS NULL OR UPPER(t.uf) = UPPER($2))
          AND ($3::text IS NULL OR n.titulo ILIKE $3 OR COALESCE(n.subtitulo, '') ILIKE $3)
          AND ($4::text IS NULL OR n.formato::text = UPPER($4))
        ORDER BY n.hype_score DESC, n.publicado_em DESC NULLS LAST, n.criado_em DESC
        LIMIT $5 OFFSET $6
        "#
    };

    let noticias_db = sqlx::query_as::<_, Noticia>(query_str)
        .bind(params.ibge.as_deref())
        .bind(params.uf.as_deref())
        .bind(termo_busca.as_deref())
        .bind(params.formato.as_deref())
        .bind(limit)
        .bind(offset)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let feed_items: Vec<NoticiaFeedItem> = noticias_db
        .into_iter()
        .map(|n| NoticiaFeedItem {
            id: n.id.to_string(),
            titulo: n.titulo,
            slug: n.slug,
            subtitulo: n.subtitulo,
            escopo: n.escopo,
            ibge_codigo: n.ibge_codigo,
            formato: n.formato,
            video_url: n.video_url,
            audio_url: n.audio_url,
            pdf_edicao_url: n.pdf_edicao_url,
            conteudo_sanitizado: n.conteudo_sanitizado,
            imagem_destaque_url: n.imagem_destaque_url,
            publicado_em: n.publicado_em.map(|d| d.to_rfc3339()),
            votos_positivos: n.votos_positivos,
            votos_negativos: n.votos_negativos,
            hype_score: if modo_eleitoral_ativo {
                None
            } else {
                Some(n.hype_score)
            },
        })
        .collect();

    let total = feed_items.len();

    Ok(Json(FeedResponse {
        modo_eleitoral_ativo,
        banner_legal: banner_texto,
        total_itens: total,
        noticias: feed_items,
    }))
}
