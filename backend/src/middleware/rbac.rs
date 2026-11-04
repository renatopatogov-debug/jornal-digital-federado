use crate::error::AppError;
use crate::models::{Cargo, StatusUsuario, TipoEscopo};
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AuthContext {
    pub usuario_id: Uuid,
    pub nome: String,
    pub email: String,
    pub cargo: Cargo,
    pub escopo: TipoEscopo,
    pub ibge_codigo: Option<String>,
    pub filiado_verificado: bool,
    pub status: StatusUsuario,
}

#[allow(dead_code)]
impl AuthContext {
    /// Validação estrita de jurisdição editorial (ADR D4)
    pub fn pode_publicar_em(
        &self,
        escopo_destino: &TipoEscopo,
        ibge_destino: Option<&str>,
    ) -> Result<(), AppError> {
        match (escopo_destino, &self.cargo.escopo_permitido) {
            (TipoEscopo::Nacional, _) => {
                if !self.cargo.pode_publicar_nacional {
                    return Err(AppError::Forbidden(
                        "Sem privilégio para publicar no Feed Nacional.".to_string(),
                    ));
                }
            }
            (TipoEscopo::Estadual, _) => {
                if !self.cargo.pode_publicar_estadual {
                    return Err(AppError::Forbidden(
                        "Sem privilégio para publicar no Feed Estadual.".to_string(),
                    ));
                }
                if let (Some(user_ibge), Some(dest_ibge)) = (&self.ibge_codigo, ibge_destino) {
                    if user_ibge != dest_ibge {
                        return Err(AppError::Forbidden(
                            "Violação de Jurisdição Territorial: UF de destino diferente da autorizada.".to_string(),
                        ));
                    }
                }
            }
            (TipoEscopo::Municipal, _) => {
                if !self.cargo.pode_publicar_municipal {
                    return Err(AppError::Forbidden(
                        "Sem privilégio para publicar no Feed Municipal.".to_string(),
                    ));
                }
                if let (Some(user_ibge), Some(dest_ibge)) = (&self.ibge_codigo, ibge_destino) {
                    if user_ibge != dest_ibge {
                        return Err(AppError::Forbidden(
                            "Violação de Jurisdição Territorial: Município de destino diferente do autorizado.".to_string(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
