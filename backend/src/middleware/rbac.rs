use crate::error::AppError;
use crate::models::{Cargo, StatusUsuario, TipoEscopo};
use crate::services::crypto::CryptoService;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, HeaderMap},
};
use sqlx::PgPool;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub usuario_id: Uuid,
    #[allow(dead_code)]
    pub nome: String,
    #[allow(dead_code)]
    pub email: String,
    pub cargo: Cargo,
    pub escopo: TipoEscopo,
    pub ibge_codigo: Option<String>,
    pub filiado_verificado: bool,
    pub status: StatusUsuario,
}

#[derive(sqlx::FromRow)]
struct AuthSessionRow {
    usuario_id: Uuid,
    nome: String,
    email: String,
    escopo: TipoEscopo,
    ibge_codigo: Option<String>,
    filiado_verificado: bool,
    status: StatusUsuario,
    cargo_id: i32,
    cargo_slug: String,
    cargo_nome: String,
    cargo_descricao: Option<String>,
    escopo_permitido: Option<TipoEscopo>,
    nivel_hierarquico: i32,
    pode_gerenciar_infra: bool,
    pode_alterar_cargos: bool,
    pode_ativar_filiados: bool,
    pode_revogar_lote: bool,
    pode_publicar_nacional: bool,
    pode_publicar_estadual: bool,
    pode_publicar_municipal: bool,
    pode_solicitar_escalonamento: bool,
    pode_aprovar_escalonamento: bool,
    pode_votar_hype: bool,
    pode_leitura: bool,
    limite_ativacao_semanal: Option<i32>,
}

pub fn extrair_token_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie_str = headers.get(header::COOKIE)?.to_str().ok()?;
    for prefix in ["__Host-jornal_session=", "jornal_session="] {
        for part in cookie_str.split(';') {
            let part = part.trim();
            if let Some(token) = part.strip_prefix(prefix) {
                let token = token.trim();
                if !token.is_empty() {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

pub fn extrair_ip_cliente(headers: &HeaderMap) -> IpAddr {
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = xff.split(',').next().map(str::trim) {
            if let Ok(ip) = first.parse::<IpAddr>() {
                return ip;
            }
        }
    }
    if let Some(real_ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        if let Ok(ip) = real_ip.trim().parse::<IpAddr>() {
            return ip;
        }
    }
    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
}

#[async_trait]
impl FromRequestParts<PgPool> for AuthContext {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        pool: &PgPool,
    ) -> Result<Self, Self::Rejection> {
        let token = extrair_token_cookie(&parts.headers).ok_or_else(|| {
            AppError::Unauthorized(
                "Autenticação obrigatória via cookie HTTP-Only de sessão.".to_string(),
            )
        })?;

        let token_hash = CryptoService::hash_token_sha256(&token);

        let row = sqlx::query_as::<_, AuthSessionRow>(
            r#"
            SELECT u.id AS usuario_id,
                   u.nome,
                   u.email::text AS email,
                   u.escopo,
                   u.ibge_codigo,
                   u.filiado_verificado,
                   u.status,
                   c.id AS cargo_id,
                   c.slug AS cargo_slug,
                   c.nome AS cargo_nome,
                   c.descricao AS cargo_descricao,
                   c.escopo_permitido,
                   c.nivel_hierarquico,
                   c.pode_gerenciar_infra,
                   c.pode_alterar_cargos,
                   c.pode_ativar_filiados,
                   c.pode_revogar_lote,
                   c.pode_publicar_nacional,
                   c.pode_publicar_estadual,
                   c.pode_publicar_municipal,
                   c.pode_solicitar_escalonamento,
                   c.pode_aprovar_escalonamento,
                   c.pode_votar_hype,
                   c.pode_leitura,
                   c.limite_ativacao_semanal
            FROM sessoes s
            JOIN usuarios u ON s.usuario_id = u.id
            JOIN cargos c ON u.cargo_id = c.id
            WHERE s.token_hash = $1
              AND s.expira_em > NOW()
            LIMIT 1
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Sessão inválida ou expirada.".to_string()))?;

        if row.status != StatusUsuario::Ativo {
            return Err(AppError::Forbidden(
                "Conta suspensa ou pendente de ativação pelo diretório.".to_string(),
            ));
        }

        let _ = sqlx::query("UPDATE sessoes SET ultimo_acesso = NOW() WHERE token_hash = $1")
            .bind(&token_hash)
            .execute(pool)
            .await;

        Ok(Self {
            usuario_id: row.usuario_id,
            nome: row.nome,
            email: row.email,
            cargo: Cargo {
                id: row.cargo_id,
                slug: row.cargo_slug,
                nome: row.cargo_nome,
                descricao: row.cargo_descricao,
                escopo_permitido: row.escopo_permitido,
                nivel_hierarquico: row.nivel_hierarquico,
                pode_gerenciar_infra: row.pode_gerenciar_infra,
                pode_alterar_cargos: row.pode_alterar_cargos,
                pode_ativar_filiados: row.pode_ativar_filiados,
                pode_revogar_lote: row.pode_revogar_lote,
                pode_publicar_nacional: row.pode_publicar_nacional,
                pode_publicar_estadual: row.pode_publicar_estadual,
                pode_publicar_municipal: row.pode_publicar_municipal,
                pode_solicitar_escalonamento: row.pode_solicitar_escalonamento,
                pode_aprovar_escalonamento: row.pode_aprovar_escalonamento,
                pode_votar_hype: row.pode_votar_hype,
                pode_leitura: row.pode_leitura,
                limite_ativacao_semanal: row.limite_ativacao_semanal,
            },
            escopo: row.escopo,
            ibge_codigo: row.ibge_codigo,
            filiado_verificado: row.filiado_verificado,
            status: row.status,
        })
    }
}

impl AuthContext {
    /// Validação estrita de jurisdição territorial (ADR D4)
    pub fn validar_jurisdicao_alvo(
        &self,
        escopo_alvo: &TipoEscopo,
        ibge_alvo: Option<&str>,
    ) -> Result<(), AppError> {
        match (&self.escopo, escopo_alvo) {
            (TipoEscopo::Nacional, _) => Ok(()),
            (TipoEscopo::Estadual, TipoEscopo::Nacional) => Err(AppError::Forbidden(
                "Violação de Jurisdição Territorial: instância estadual não atua em escopo nacional."
                    .to_string(),
            )),
            (TipoEscopo::Estadual, TipoEscopo::Estadual | TipoEscopo::Municipal) => {
                match (self.ibge_codigo.as_deref(), ibge_alvo) {
                    (Some(user_uf), Some(dest_ibge)) if dest_ibge.starts_with(user_uf) => Ok(()),
                    _ => Err(AppError::Forbidden(
                        "Violação de Jurisdição Territorial: território fora da UF autorizada."
                            .to_string(),
                    )),
                }
            }
            (TipoEscopo::Municipal, TipoEscopo::Municipal) => {
                match (self.ibge_codigo.as_deref(), ibge_alvo) {
                    (Some(user_mun), Some(dest_mun)) if user_mun == dest_mun => Ok(()),
                    _ => Err(AppError::Forbidden(
                        "Violação de Jurisdição Territorial: município diferente do autorizado."
                            .to_string(),
                    )),
                }
            }
            (TipoEscopo::Municipal, _) => Err(AppError::Forbidden(
                "Violação de Jurisdição Territorial: instância municipal não atua fora do município."
                    .to_string(),
            )),
        }
    }

    /// Validação estrita de privilégio e jurisdição editorial (ADR D4)
    pub fn pode_publicar_em(
        &self,
        escopo_destino: &TipoEscopo,
        ibge_destino: Option<&str>,
    ) -> Result<(), AppError> {
        match escopo_destino {
            TipoEscopo::Nacional => {
                if !self.cargo.pode_publicar_nacional {
                    return Err(AppError::Forbidden(
                        "Sem privilégio para publicar no Feed Nacional.".to_string(),
                    ));
                }
            }
            TipoEscopo::Estadual => {
                if !self.cargo.pode_publicar_estadual {
                    return Err(AppError::Forbidden(
                        "Sem privilégio para publicar no Feed Estadual.".to_string(),
                    ));
                }
                self.validar_jurisdicao_alvo(escopo_destino, ibge_destino)?;
            }
            TipoEscopo::Municipal => {
                if !self.cargo.pode_publicar_municipal {
                    return Err(AppError::Forbidden(
                        "Sem privilégio para publicar no Feed Municipal.".to_string(),
                    ));
                }
                self.validar_jurisdicao_alvo(escopo_destino, ibge_destino)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AuthContext;
    use crate::models::{Cargo, StatusUsuario, TipoEscopo};
    use uuid::Uuid;

    fn mock_auth(escopo: TipoEscopo, ibge: Option<&str>) -> AuthContext {
        AuthContext {
            usuario_id: Uuid::nil(),
            nome: "Teste".to_string(),
            email: "teste@jornalfederado.org.br".to_string(),
            cargo: Cargo {
                id: 1,
                slug: "teste".to_string(),
                nome: "Teste".to_string(),
                descricao: None,
                escopo_permitido: Some(escopo.clone()),
                nivel_hierarquico: 3,
                pode_gerenciar_infra: false,
                pode_alterar_cargos: false,
                pode_ativar_filiados: true,
                pode_revogar_lote: false,
                pode_publicar_nacional: escopo == TipoEscopo::Nacional,
                pode_publicar_estadual: escopo != TipoEscopo::Municipal,
                pode_publicar_municipal: true,
                pode_solicitar_escalonamento: true,
                pode_aprovar_escalonamento: false,
                pode_votar_hype: true,
                pode_leitura: true,
                limite_ativacao_semanal: Some(50),
            },
            escopo,
            ibge_codigo: ibge.map(str::to_string),
            filiado_verificado: true,
            status: StatusUsuario::Ativo,
        }
    }

    #[test]
    fn isolamento_territorial_impede_desvio_de_municipio_ou_uf() {
        let sp_estadual = mock_auth(TipoEscopo::Estadual, Some("35"));
        assert!(sp_estadual
            .pode_publicar_em(&TipoEscopo::Estadual, Some("35"))
            .is_ok());
        assert!(sp_estadual
            .pode_publicar_em(&TipoEscopo::Municipal, Some("3550308"))
            .is_ok());
        assert!(sp_estadual
            .pode_publicar_em(&TipoEscopo::Municipal, Some("3304557"))
            .is_err());
        assert!(sp_estadual
            .pode_publicar_em(&TipoEscopo::Nacional, None)
            .is_err());

        let sp_cap = mock_auth(TipoEscopo::Municipal, Some("3550308"));
        assert!(sp_cap
            .pode_publicar_em(&TipoEscopo::Municipal, Some("3550308"))
            .is_ok());
        assert!(sp_cap
            .pode_publicar_em(&TipoEscopo::Municipal, Some("3509502"))
            .is_err());
        assert!(sp_cap
            .pode_publicar_em(&TipoEscopo::Estadual, Some("35"))
            .is_err());
    }
}
