use crate::error::AppError;
use crate::middleware::rbac::AuthContext;
use crate::models::{ModoEleitoralConfig, StatusNoticia, TipoEscopo, TipoFormatoMateria};
use crate::services::sanitizer::sanitizar_html_editorial;
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CriarNoticiaRequest {
    pub titulo: String,
    pub subtitulo: Option<String>,
    pub escopo: TipoEscopo,
    pub ibge_codigo: Option<String>,
    pub formato: Option<TipoFormatoMateria>,
    pub video_url: Option<String>,
    pub audio_url: Option<String>,
    pub pdf_edicao_url: Option<String>,
    pub conteudo_html: String,
    pub imagem_destaque_url: Option<String>,
    pub publicar_agora: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CriarNoticiaResponse {
    pub sucesso: bool,
    pub noticia_id: Uuid,
    pub slug: String,
    pub status: StatusNoticia,
    pub conteudo_sanitizado: String,
    pub mensagem: String,
}

#[derive(Debug, Deserialize)]
pub struct PreviewSanitizadoRequest {
    pub conteudo_html: String,
}

#[derive(Debug, Serialize)]
pub struct PreviewSanitizadoResponse {
    pub conteudo_sanitizado: String,
}

#[derive(Debug, Deserialize)]
pub struct SolicitarEscalonamentoRequest {
    pub escopo_origem: TipoEscopo,
    pub escopo_destino: TipoEscopo,
    pub justificativa: String,
}

#[derive(Debug, Serialize)]
pub struct EscalonamentoResponse {
    pub sucesso: bool,
    pub solicitacao_id: Uuid,
    pub mensagem: String,
}

#[derive(Debug, Serialize)]
pub struct NoticiaDetalheItem {
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
    pub autor_nome: Option<String>,
    pub publicado_em: Option<String>,
    pub votos_positivos: i32,
    pub votos_negativos: i32,
    pub hype_score: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct NoticiaDetalheResponse {
    pub modo_eleitoral_ativo: bool,
    pub noticia: NoticiaDetalheItem,
}

#[derive(Debug, sqlx::FromRow)]
struct NoticiaDetalheRow {
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
    autor_nome: Option<String>,
    votos_positivos: i32,
    votos_negativos: i32,
    hype_score: f64,
    publicado_em: Option<DateTime<Utc>>,
}

pub async fn obter_noticia_por_slug(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
) -> Result<Json<NoticiaDetalheResponse>, AppError> {
    let config_val: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT valor FROM configuracoes WHERE chave = 'modo_eleitoral'")
            .fetch_optional(&pool)
            .await?;

    let modo_eleitoral_ativo = config_val
        .and_then(|v| serde_json::from_value::<ModoEleitoralConfig>(v).ok())
        .map(|c| c.ativo)
        .unwrap_or(false);

    let row = sqlx::query_as::<_, NoticiaDetalheRow>(
        r#"
        SELECT n.id, n.titulo, n.slug, n.subtitulo, n.escopo,
               n.ibge_codigo, t.uf::text AS uf, t.nome AS territorio_nome,
               n.formato, n.video_url, n.audio_url,
               n.pdf_edicao_url, n.conteudo_sanitizado, n.imagem_destaque_url,
               u.nome AS autor_nome,
               n.votos_positivos, n.votos_negativos,
               n.hype_score::float8 AS hype_score,
               n.publicado_em
        FROM noticias n
        LEFT JOIN usuarios u ON n.autor_id = u.id
        LEFT JOIN territorios t ON n.ibge_codigo = t.codigo_ibge
        WHERE n.slug = $1 AND n.status = 'PUBLICADO'
        LIMIT 1
        "#,
    )
    .bind(slug.trim())
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Matéria não encontrada.".to_string()))?;

    Ok(Json(NoticiaDetalheResponse {
        modo_eleitoral_ativo,
        noticia: NoticiaDetalheItem {
            id: row.id.to_string(),
            titulo: row.titulo,
            slug: row.slug,
            subtitulo: row.subtitulo,
            escopo: row.escopo,
            ibge_codigo: row.ibge_codigo,
            uf: row.uf,
            territorio_nome: row.territorio_nome,
            formato: row.formato,
            video_url: row.video_url,
            audio_url: row.audio_url,
            pdf_edicao_url: row.pdf_edicao_url,
            conteudo_sanitizado: row.conteudo_sanitizado,
            imagem_destaque_url: row.imagem_destaque_url,
            autor_nome: row.autor_nome,
            publicado_em: row.publicado_em.map(|d| d.to_rfc3339()),
            votos_positivos: row.votos_positivos,
            votos_negativos: row.votos_negativos,
            hype_score: if modo_eleitoral_ativo {
                None
            } else {
                Some(row.hype_score)
            },
        },
    }))
}

pub async fn preview_sanitizado(
    auth: AuthContext,
    Json(payload): Json<PreviewSanitizadoRequest>,
) -> Result<Json<PreviewSanitizadoResponse>, AppError> {
    if !auth.cargo.pode_publicar_municipal
        && !auth.cargo.pode_publicar_estadual
        && !auth.cargo.pode_publicar_nacional
    {
        return Err(AppError::Forbidden(
            "Sem privilégio editorial para pré-visualização no CMS.".to_string(),
        ));
    }

    let limpo = sanitizar_html_editorial(&payload.conteudo_html);
    Ok(Json(PreviewSanitizadoResponse {
        conteudo_sanitizado: limpo,
    }))
}

pub async fn criar_noticia(
    auth: AuthContext,
    State(pool): State<PgPool>,
    Json(payload): Json<CriarNoticiaRequest>,
) -> Result<Json<CriarNoticiaResponse>, AppError> {
    if payload.titulo.trim().len() < 5 {
        return Err(AppError::BadRequest(
            "O título da matéria deve conter pelo menos 5 caracteres.".to_string(),
        ));
    }

    // 1. Validar Consistência Territorial do Payload e RBAC do Redator (ADR D4)
    match payload.escopo {
        TipoEscopo::Nacional => {
            if payload.ibge_codigo.is_some() {
                return Err(AppError::BadRequest(
                    "Matérias de escopo NACIONAL devem ter ibge_codigo nulo.".to_string(),
                ));
            }
        }
        TipoEscopo::Estadual | TipoEscopo::Municipal => {
            if payload.ibge_codigo.as_deref().unwrap_or("").trim().is_empty() {
                return Err(AppError::BadRequest(
                    "Matérias ESTADUAIS ou MUNICIPAIS exigem código IBGE válido.".to_string(),
                ));
            }
        }
    }

    auth.pode_publicar_em(&payload.escopo, payload.ibge_codigo.as_deref())?;

    // 2. Sanitização estrita contra XSS via Crate Ammonia (ADR D8)
    let conteudo_sanitizado = sanitizar_html_editorial(&payload.conteudo_html);

    // 3. Geração de Slug URL-friendly
    let slug_base = payload
        .titulo
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ã' | 'â' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            c if c.is_ascii_alphanumeric() => c,
            ' ' | '-' => '-',
            _ => '\0',
        })
        .filter(|&c| c != '\0')
        .collect::<String>();
    let slug_limpo = slug_base
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = format!("{}-{}", slug_limpo, &Uuid::new_v4().to_string()[..8]);

    let formato = payload.formato.unwrap_or(TipoFormatoMateria::Texto);
    let publicar = payload.publicar_agora.unwrap_or(true);
    let status_alvo = if publicar {
        StatusNoticia::Publicado
    } else {
        StatusNoticia::Rascunho
    };

    // 4. Persistência transacional no PostgreSQL 16
    let noticia_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO noticias (
            titulo, slug, subtitulo, autor_id, escopo, ibge_codigo,
            formato, video_url, audio_url, pdf_edicao_url,
            conteudo_original, conteudo_sanitizado, imagem_destaque_url,
            status, publicado_em
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9, $10,
            $11, $12, $13,
            $14, CASE WHEN $15 THEN NOW() ELSE NULL END
        )
        RETURNING id
        "#,
    )
    .bind(payload.titulo.trim())
    .bind(&slug)
    .bind(&payload.subtitulo)
    .bind(auth.usuario_id)
    .bind(&payload.escopo)
    .bind(&payload.ibge_codigo)
    .bind(&formato)
    .bind(&payload.video_url)
    .bind(&payload.audio_url)
    .bind(&payload.pdf_edicao_url)
    .bind(&payload.conteudo_html)
    .bind(&conteudo_sanitizado)
    .bind(&payload.imagem_destaque_url)
    .bind(&status_alvo)
    .bind(publicar)
    .fetch_one(&pool)
    .await?;

    Ok(Json(CriarNoticiaResponse {
        sucesso: true,
        noticia_id,
        slug,
        status: status_alvo,
        conteudo_sanitizado,
        mensagem: if publicar {
            "Matéria sanitizada via Ammonia e publicada imediatamente no feed!".to_string()
        } else {
            "Rascunho sanitizado e salvo com sucesso.".to_string()
        },
    }))
}

pub async fn solicitar_escalonamento(
    auth: AuthContext,
    State(pool): State<PgPool>,
    Path(noticia_id): Path<Uuid>,
    Json(payload): Json<SolicitarEscalonamentoRequest>,
) -> Result<Json<EscalonamentoResponse>, AppError> {
    if !auth.cargo.pode_solicitar_escalonamento {
        return Err(AppError::Forbidden(
            "Cargo atual não possui permissão para solicitar escalonamento editorial.".to_string(),
        ));
    }

    let valido = matches!(
        (&payload.escopo_origem, &payload.escopo_destino),
        (TipoEscopo::Municipal, TipoEscopo::Estadual)
            | (TipoEscopo::Estadual, TipoEscopo::Nacional)
    );

    if !valido {
        return Err(AppError::BadRequest(
            "Direção de escalonamento inválida: deve ser MUNICIPAL -> ESTADUAL ou ESTADUAL -> NACIONAL."
                .to_string(),
        ));
    }

    let noticia_row = sqlx::query("SELECT escopo, ibge_codigo FROM noticias WHERE id = $1")
        .bind(noticia_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Matéria não encontrada.".to_string()))?;

    let noticia_escopo: TipoEscopo = noticia_row.try_get("escopo")?;
    let noticia_ibge: Option<String> = noticia_row.try_get("ibge_codigo")?;

    if noticia_escopo != payload.escopo_origem {
        return Err(AppError::BadRequest(
            "O escopo de origem informado não corresponde ao escopo real da matéria.".to_string(),
        ));
    }

    auth.validar_jurisdicao_alvo(&noticia_escopo, noticia_ibge.as_deref())?;

    let solicitacao_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO solicitacoes_escalonamento (
            noticia_id, solicitante_id, escopo_origem, escopo_destino, justificativa
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(noticia_id)
    .bind(auth.usuario_id)
    .bind(&payload.escopo_origem)
    .bind(&payload.escopo_destino)
    .bind(&payload.justificativa)
    .fetch_one(&pool)
    .await?;

    Ok(Json(EscalonamentoResponse {
        sucesso: true,
        solicitacao_id,
        mensagem: "Solicitação de escalonamento editorial registrada para avaliação da instância superior."
            .to_string(),
    }))
}
