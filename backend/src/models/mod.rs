use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "tipo_escopo", rename_all = "UPPERCASE")]
pub enum TipoEscopo {
    Nacional,
    Estadual,
    Municipal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "status_usuario", rename_all = "UPPERCASE")]
pub enum StatusUsuario {
    Ativo,
    Suspenso,
    Pendente,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "status_noticia", rename_all = "UPPERCASE")]
pub enum StatusNoticia {
    Rascunho,
    EmRevisao,
    Publicado,
    Arquivado,
    Rejeitado,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "tipo_formato_materia", rename_all = "UPPERCASE")]
pub enum TipoFormatoMateria {
    Texto,
    Video,
    Audio,
    ImpressoPdf,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Territorio {
    pub codigo_ibge: String,
    pub nome: String,
    pub tipo: TipoEscopo,
    pub uf: String,
    pub ibge_uf_codigo: String,
    pub populacao_estimada: Option<i32>,
    pub criado_em: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Cargo {
    pub id: i32,
    pub slug: String,
    pub nome: String,
    pub descricao: Option<String>,
    pub escopo_permitido: Option<TipoEscopo>,
    pub nivel_hierarquico: i32,
    pub pode_gerenciar_infra: bool,
    pub pode_alterar_cargos: bool,
    pub pode_ativar_filiados: bool,
    pub pode_revogar_lote: bool,
    pub pode_publicar_nacional: bool,
    pub pode_publicar_estadual: bool,
    pub pode_publicar_municipal: bool,
    pub pode_solicitar_escalonamento: bool,
    pub pode_aprovar_escalonamento: bool,
    pub pode_votar_hype: bool,
    pub pode_leitura: bool,
    pub limite_ativacao_semanal: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Noticia {
    pub id: Uuid,
    pub titulo: String,
    pub slug: String,
    pub subtitulo: Option<String>,
    pub autor_id: Uuid,
    pub escopo: TipoEscopo,
    pub ibge_codigo: Option<String>,
    pub formato: TipoFormatoMateria,
    pub video_url: Option<String>,
    pub audio_url: Option<String>,
    pub audio_duracao_segundos: Option<i32>,
    pub pdf_edicao_url: Option<String>,
    pub pdf_numero_paginas: Option<i32>,
    pub conteudo_sanitizado: String,
    pub imagem_destaque_url: Option<String>,
    pub status: StatusNoticia,
    pub votos_positivos: i32,
    pub votos_negativos: i32,
    pub hype_score: f64,
    pub publicado_em: Option<DateTime<Utc>>,
    pub criado_em: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModoEleitoralConfig {
    pub ativo: bool,
    pub banner_obrigatorio: bool,
    pub banner_texto: String,
    pub freeze_hype: bool,
    pub motivo: String,
}
