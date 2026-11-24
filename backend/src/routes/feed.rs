use crate::config::AppConfig;
use crate::error::AppError;
use crate::models::{ModoEleitoralConfig, TipoEscopo, TipoFormatoMateria};
use crate::services::redis_cache::RedisService;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderName, HeaderValue},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

const LIMITE_PADRAO: i64 = 20;
const LIMITE_MAXIMO: i64 = 50;
const OFFSET_MAXIMO: i64 = 10_000;

#[derive(Debug, Deserialize)]
pub struct FeedQueryParams {
    pub uf: Option<String>,
    pub ibge: Option<String>,
    pub q: Option<String>,
    pub formato: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Quando `true`, matérias NACIONAIS continuam visíveis ao filtrar por UF/IBGE
    pub incluir_nacional: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedResponse {
    pub modo_eleitoral_ativo: bool,
    pub banner_legal: Option<String>,
    pub total_itens: usize,
    pub limit: i64,
    pub offset: i64,
    pub tem_mais: bool,
    pub noticias: Vec<NoticiaFeedItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoticiaFeedItem {
    pub id: String,
    pub titulo: String,
    pub slug: String,
    pub subtitulo: Option<String>,
    pub escopo: TipoEscopo,
    pub ibge_codigo: Option<String>,
    pub uf: Option<String>,
    pub territorio_nome: Option<String>,
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

#[derive(Debug, sqlx::FromRow)]
struct FeedRow {
    id: Uuid,
    titulo: String,
    slug: String,
    subtitulo: Option<String>,
    escopo: TipoEscopo,
    ibge_codigo: Option<String>,
    uf: Option<String>,
    territorio_nome: Option<String>,
    formato: TipoFormatoMateria,
    video_url: Option<String>,
    audio_url: Option<String>,
    pdf_edicao_url: Option<String>,
    conteudo_sanitizado: String,
    imagem_destaque_url: Option<String>,
    votos_positivos: i32,
    votos_negativos: i32,
    hype_score: f64,
    publicado_em: Option<DateTime<Utc>>,
}

/// Escapa curingas do ILIKE (`\`, `%`, `_`) para que a busca textual seja literal
fn escapar_termo_like(termo: &str) -> String {
    let mut out = String::with_capacity(termo.len() + 2);
    for c in termo.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn normalizar_filtro(valor: Option<&String>) -> Option<String> {
    valor
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub async fn get_feed(
    State(pool): State<PgPool>,
    Query(params): Query<FeedQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let limit = params.limit.unwrap_or(LIMITE_PADRAO).clamp(1, LIMITE_MAXIMO);
    let offset = params.offset.unwrap_or(0).clamp(0, OFFSET_MAXIMO);
    let ibge = normalizar_filtro(params.ibge.as_ref());
    let uf = normalizar_filtro(params.uf.as_ref());
    let formato = normalizar_filtro(params.formato.as_ref());
    let q_norm = normalizar_filtro(params.q.as_ref());
    let termo_busca = q_norm.as_deref().map(|s| format!("%{}%", escapar_termo_like(s)));
    let incluir_nacional = params.incluir_nacional.unwrap_or(false);

    // 0. Tenta servir a partir do cache Redis 7 de baixa latência (TTL curto + invalidação por epoch)
    let ttl_secs = AppConfig::feed_cache_ttl_from_env();
    let redis = RedisService::global();
    let cache_key = if ttl_secs > 0 {
        redis.obter_epoch_feed().await.map(|epoch| {
            format!(
                "jornal:feed:v{}:ibge={}:uf={}:q={}:fmt={}:lim={}:off={}:nac={}",
                epoch,
                ibge.as_deref().unwrap_or("-"),
                uf.as_deref().unwrap_or("-").to_ascii_uppercase(),
                q_norm.as_deref().unwrap_or("-"),
                formato.as_deref().unwrap_or("-").to_ascii_uppercase(),
                limit,
                offset,
                incluir_nacional
            )
        })
    } else {
        None
    };

    if let Some(ref key) = cache_key {
        if let Some(cached_json) = redis.get_string(key).await {
            if let Ok(cached_feed) = serde_json::from_str::<FeedResponse>(&cached_json) {
                let mut headers = HeaderMap::new();
                headers.insert(
                    HeaderName::from_static("x-cache"),
                    HeaderValue::from_static("HIT"),
                );
                return Ok((headers, Json(cached_feed)));
            }
        }
    }

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

    // 2. Ordenação comutável pelo Modo Eleitoral (ADR D6)
    let ordenacao = if modo_eleitoral_ativo {
        "n.publicado_em DESC NULLS LAST, n.criado_em DESC"
    } else {
        "n.hype_score DESC, n.publicado_em DESC NULLS LAST, n.criado_em DESC"
    };

    // 3. Consulta parametrizada com filtros combinados (IBGE, UF, Busca Textual e Formato)
    //    e enriquecimento territorial (UF + nome oficial) via tabela territorios
    let sql = format!(
        r#"
        SELECT n.id, n.titulo, n.slug, n.subtitulo, n.escopo,
               n.ibge_codigo, t.uf::text AS uf, t.nome AS territorio_nome,
               n.formato, n.video_url, n.audio_url, n.pdf_edicao_url,
               n.conteudo_sanitizado, n.imagem_destaque_url,
               n.votos_positivos, n.votos_negativos,
               n.hype_score::float8 AS hype_score, n.publicado_em
        FROM noticias n
        LEFT JOIN territorios t ON n.ibge_codigo = t.codigo_ibge
        WHERE n.status = 'PUBLICADO'
          AND ($1::text IS NULL OR n.ibge_codigo = $1 OR ($7::bool AND n.escopo = 'NACIONAL'))
          AND ($2::text IS NULL OR UPPER(t.uf) = UPPER($2) OR ($7::bool AND n.escopo = 'NACIONAL'))
          AND ($3::text IS NULL OR n.titulo ILIKE $3 OR COALESCE(n.subtitulo, '') ILIKE $3
               OR COALESCE(t.nome, '') ILIKE $3)
          AND ($4::text IS NULL OR n.formato::text = UPPER($4))
        ORDER BY {}
        LIMIT $5 OFFSET $6
        "#,
        ordenacao
    );

    // Busca limit + 1 para sinalizar se existe próxima página sem COUNT(*) adicional
    let mut linhas = sqlx::query_as::<_, FeedRow>(&sql)
        .bind(ibge.as_deref())
        .bind(uf.as_deref())
        .bind(termo_busca.as_deref())
        .bind(formato.as_deref())
        .bind(limit + 1)
        .bind(offset)
        .bind(incluir_nacional)
        .fetch_all(&pool)
        .await?;

    let limite_usize = usize::try_from(limit).unwrap_or(20);
    let tem_mais = linhas.len() > limite_usize;
    linhas.truncate(limite_usize);

    let feed_items: Vec<NoticiaFeedItem> = linhas
        .into_iter()
        .map(|n| NoticiaFeedItem {
            id: n.id.to_string(),
            titulo: n.titulo,
            slug: n.slug,
            subtitulo: n.subtitulo,
            escopo: n.escopo,
            ibge_codigo: n.ibge_codigo,
            uf: n.uf,
            territorio_nome: n.territorio_nome,
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

    let response_payload = FeedResponse {
        modo_eleitoral_ativo,
        banner_legal: banner_texto,
        total_itens: feed_items.len(),
        limit,
        offset,
        tem_mais,
        noticias: feed_items,
    };

    if let Some(ref key) = cache_key {
        if let Ok(serialized) = serde_json::to_string(&response_payload) {
            let _ = redis.set_ex(key, ttl_secs, &serialized).await;
        }
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-cache"),
        HeaderValue::from_static("MISS"),
    );

    Ok((headers, Json(response_payload)))
}

#[cfg(test)]
mod tests {
    use super::{escapar_termo_like, normalizar_filtro};

    #[test]
    fn busca_textual_escapa_curingas_do_ilike() {
        assert_eq!(escapar_termo_like("São Paulo"), "São Paulo");
        assert_eq!(escapar_termo_like("100%"), "100\\%");
        assert_eq!(escapar_termo_like("a_b"), "a\\_b");
        assert_eq!(escapar_termo_like("c:\\x"), "c:\\\\x");
    }

    #[test]
    fn filtros_vazios_sao_ignorados() {
        assert_eq!(normalizar_filtro(Some(&"   ".to_string())), None);
        assert_eq!(normalizar_filtro(Some(&" SP ".to_string())), Some("SP".to_string()));
        assert_eq!(normalizar_filtro(None), None);
    }
}
