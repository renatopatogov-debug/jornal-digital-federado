use crate::models::TipoEscopo;
use chrono::{DateTime, Utc};

/// ADR D2 & Seção 4.2 do Plano Diretor v2.1
/// Constantes de Decaimento Gravitacional
pub const G_GRAVIDADE_NACIONAL: f64 = 1.8;
pub const G_GRAVIDADE_REGIONAL: f64 = 1.4;
pub const OFFSET_HORAS: f64 = 2.0;
pub const MIN_VOTOS_QUALIFICADOS: i32 = 10;

pub struct HypeEngine;

impl HypeEngine {
    /// Calcula o Hype Score de uma matéria com base no escopo e no tempo decorrido
    pub fn calcular_score(
        escopo: &TipoEscopo,
        votos_positivos: i32,
        votos_negativos: i32,
        publicado_em: Option<DateTime<Utc>>,
        agora: DateTime<Utc>,
    ) -> f64 {
        let total_votos = votos_positivos + votos_negativos;

        // 1. Fase Inicial (< 10 votos): Proteção aos pequenos municípios
        // Retorna score 0.0 pois a matéria é ordenada puramente por publicado_em DESC
        if total_votos < MIN_VOTOS_QUALIFICADOS {
            return 0.0;
        }

        let saldo_votos = f64::from(votos_positivos - votos_negativos);
        if saldo_votos <= 0.0 {
            return 0.0;
        }

        // 2. Horas decorridas desde a publicação oficial
        let horas_decorridas = match publicado_em {
            Some(dt) => {
                let diff_segundos = agora.signed_duration_since(dt).num_seconds().max(0);
                (diff_segundos as f64) / 3600.0
            }
            None => 0.0,
        };

        // 3. Gravidade temporal por escopo territorial
        let g = match escopo {
            TipoEscopo::Nacional => G_GRAVIDADE_NACIONAL,
            TipoEscopo::Estadual | TipoEscopo::Municipal => G_GRAVIDADE_REGIONAL,
        };

        // 4. Equação Gravitacional Pública: Saldo / (Horas + 2)^G
        let denominador = (horas_decorridas + OFFSET_HORAS).powf(g);
        let score = saldo_votos / denominador;

        // Arredondamento para 6 casas decimais
        (score * 1_000_000.0).round() / 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protecao_pequenos_municipios_abaixo_quorum() {
        let agora = Utc::now();
        let score = HypeEngine::calcular_score(&TipoEscopo::Municipal, 7, 1, Some(agora), agora);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_calculo_gravitacional_qualificado() {
        let agora = Utc::now();
        let score = HypeEngine::calcular_score(&TipoEscopo::Municipal, 20, 2, Some(agora), agora);
        assert!(score > 0.0);
    }
}
