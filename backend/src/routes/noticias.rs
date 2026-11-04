use crate::error::AppError;
use crate::models::{StatusNoticia, TipoEscopo, TipoFormatoMateria};
use crate::services::sanitizer::sanitizar_html_editorial;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CriarNoticiaRequest {
    pub autor_id: Uuid,
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
    pub solicitante_id: Uuid,
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

pub async fn preview_sanitizado(
    Json(payload): Json<PreviewSanitizadoRequest>,
) -> Result<Json<PreviewSanitizadoResponse>, AppError> {
    let limpo = sanitizar_html_editorial(&payload.conteudo_html);
    Ok(Json(PreviewSanitizadoResponse {
        conteudo_sanitizado: limpo,
    }))
}

pub async fn criar_noticia(
    State(pool): State<PgPool>,
    Json(payload): Json<CriarNoticiaRequest>,
) -> Result<Json<CriarNoticiaResponse>, AppError> {
    if payload.titulo.trim().len() < 5 {
        return Err(AppError::BadRequest(
            "O título da matéria deve conter pelo menos 5 caracteres.".to_string(),
        ));
    }

    // 1. Validar Consistência Territorial do Payload (ADR D4)
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
    let noticia = sqlx::query!(
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
        payload.titulo.trim(),
        slug,
        payload.subtitulo,
        payload.autor_id,
        payload.escopo as TipoEscopo,
        payload.ibge_codigo,
        formato as TipoFormatoMateria,
        payload.video_url,
        payload.audio_url,
        payload.pdf_edicao_url,
        payload.conteudo_html,
        conteudo_sanitizado,
        payload.imagem_destaque_url,
        status_alvo.clone() as StatusNoticia,
        publicar
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(CriarNoticiaResponse {
        sucesso: true,
        noticia_id: noticia.id,
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
    State(pool): State<PgPool>,
    Path(noticia_id): Path<Uuid>,
    Json(payload): Json<SolicitarEscalonamentoRequest>,
) -> Result<Json<EscalonamentoResponse>, AppError> {
    let valido = matches!(
        (&payload.escopo_origem, &payload.escopo_destino),
        (TipoEscopo::Municipal, TipoEscopo::Estadual) | (TipoEscopo::Estadual, TipoEscopo::Nacional)
    );

    if !valido {
        return Err(AppError::BadRequest(
            "Direção de escalonamento inválida: deve ser MUNICIPAL -> ESTADUAL ou ESTADUAL -> NACIONAL.".to_string(),
        ));
    }

    let row = sqlx::query!(
        r#"
        INSERT INTO solicitacoes_escalonamento (
            noticia_id, solicitante_id, escopo_origem, escopo_destino, justificativa
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
        noticia_id,
        payload.solicitante_id,
        payload.escopo_origem as TipoEscopo,
        payload.escopo_destino as TipoEscopo,
        payload.justificativa
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(EscalonamentoResponse {
        sucesso: true,
        solicitacao_id: row.id,
        mensagem: "Solicitação de escalonamento editorial registrada para avaliação da instância superior.".to_string(),
    }))
}
