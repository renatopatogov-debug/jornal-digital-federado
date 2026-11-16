use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::env;

pub struct CryptoService {
    cipher: Aes256Gcm,
}

impl CryptoService {
    pub fn new(chave_mestra: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new_from_slice(chave_mestra)
            .expect("Chave mestra inválida para AES-256-GCM (deve ter 32 bytes)");
        Self { cipher }
    }

    pub fn from_raw_secret(raw_key: &str) -> Self {
        let derived: [u8; 32] = Sha256::digest(raw_key.as_bytes()).into();
        Self::new(&derived)
    }

    pub fn from_env() -> Self {
        let raw_key = env::var("AES_256_GCM_MASTER_KEY")
            .unwrap_or_else(|_| "U0VNUEJBTkNBREFTRUdVUkFOQ0ExMjM0NTY3ODkwMTI=".to_string());
        Self::from_raw_secret(&raw_key)
    }

    pub fn previous_from_env() -> Option<Self> {
        env::var("AES_256_GCM_PREVIOUS_MASTER_KEY")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .map(|raw| Self::from_raw_secret(&raw))
    }

    /// ADR D8: Cifra o segredo TOTP gerando um envelope com Nonce (12B) + Ciphertext + Tag (16B)
    pub fn cifrar_envelope(
        &self,
        dado_em_claro: &[u8],
        nonce_bytes: &[u8; 12],
    ) -> Result<Vec<u8>, String> {
        let nonce = Nonce::from_slice(nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, dado_em_claro)
            .map_err(|e| format!("Falha na encriptação AES-GCM: {:?}", e))?;

        // Estrutura do envelope: [ Nonce (12B) | Ciphertext + Tag ]
        let mut envelope = Vec::with_capacity(12 + ciphertext.len());
        envelope.extend_from_slice(nonce_bytes);
        envelope.extend_from_slice(&ciphertext);

        Ok(envelope)
    }

    /// ADR D8: Cifra usando nonce aleatório criptográfico de 96 bits (ideal para rotação de chaves)
    pub fn cifrar_envelope_aleatorio(&self, dado_em_claro: &[u8]) -> Result<Vec<u8>, String> {
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        self.cifrar_envelope(dado_em_claro, &nonce_bytes)
    }

    /// Decifra com fallback opcional para chave mestra anterior durante rotação.
    /// Retorna `(texto_em_claro, usou_chave_anterior)`.
    pub fn decifrar_com_fallback(
        &self,
        envelope: &[u8],
        chave_anterior: Option<&Self>,
    ) -> Result<(Vec<u8>, bool), String> {
        if let Ok(plain) = self.decifrar_envelope(envelope) {
            return Ok((plain, false));
        }

        if let Some(prev) = chave_anterior {
            let plain = prev.decifrar_envelope(envelope)?;
            return Ok((plain, true));
        }

        Err("Envelope não pôde ser autenticado pela chave ativa nem pela anterior.".to_string())
    }

    /// ADR D8: Decifra o envelope validando a Tag de autenticação antes de retornar o segredo
    pub fn decifrar_envelope(&self, envelope: &[u8]) -> Result<Vec<u8>, String> {
        if envelope.len() < 28 {
            return Err(
                "Envelope corrompido ou menor que o tamanho mínimo de 28 bytes.".to_string(),
            );
        }

        let (nonce_bytes, ciphertext_with_tag) = envelope.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext_with_tag)
            .map_err(|e| format!("Falha na autenticação/decriptação AES-GCM: {:?}", e))
    }

    /// Hash SHA-256 seguro para tokens de cookie HTTP-Only
    pub fn hash_token_sha256(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Hash iterado (10.000 rodadas + salt) para armazenamento e verificação de senhas
    pub fn gerar_hash_senha(senha: &str, salt_hex: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(salt_hex.as_bytes());
        hasher.update(b":");
        hasher.update(senha.as_bytes());
        let mut digest: [u8; 32] = hasher.finalize().into();

        let salt_bytes = salt_hex.as_bytes();
        for _ in 1..10_000 {
            let mut round = Sha256::new();
            round.update(digest);
            round.update(salt_bytes);
            digest = round.finalize().into();
        }

        let hex_digest: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
        format!("$sha256$i=10000${}${}", salt_hex, hex_digest)
    }

    /// Verifica senha em tempo constante contra `$sha256$i=10000$<salt>$<digest>`
    pub fn verificar_senha_hash(senha: &str, hash_armazenado: &str) -> bool {
        let mut parts = hash_armazenado.split('$');
        if parts.next() != Some("")
            || parts.next() != Some("sha256")
            || parts.next() != Some("i=10000")
        {
            return false;
        }
        let Some(salt_hex) = parts.next() else {
            return false;
        };
        if parts.next().is_none() || parts.next().is_some() || salt_hex.is_empty() {
            return false;
        }

        let esperado = Self::gerar_hash_senha(senha, salt_hex);
        Self::constant_time_eq(esperado.as_bytes(), hash_armazenado.as_bytes())
    }

    pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
        let mut key_block = [0u8; 64];
        if key.len() > 64 {
            let hashed: [u8; 32] = Sha256::digest(key).into();
            key_block[..32].copy_from_slice(&hashed);
        } else {
            key_block[..key.len()].copy_from_slice(key);
        }

        let mut ipad = [0x36u8; 64];
        let mut opad = [0x5cu8; 64];
        for i in 0..64 {
            ipad[i] ^= key_block[i];
            opad[i] ^= key_block[i];
        }

        let mut inner = Sha256::new();
        inner.update(ipad);
        inner.update(data);
        let inner_hash: [u8; 32] = inner.finalize().into();

        let mut outer = Sha256::new();
        outer.update(opad);
        outer.update(inner_hash);
        outer.finalize().into()
    }

    /// RFC 6238 / RFC 4226 TOTP com HMAC-SHA256 (janela de 30s, 6 dígitos)
    pub fn gerar_totp_sha256(secret: &[u8], counter: u64) -> String {
        let mac = Self::hmac_sha256(secret, &counter.to_be_bytes());
        let offset = (mac[31] & 0x0f) as usize;
        let bin_code = (u32::from(mac[offset] & 0x7f) << 24)
            | (u32::from(mac[offset + 1]) << 16)
            | (u32::from(mac[offset + 2]) << 8)
            | u32::from(mac[offset + 3]);
        format!("{:06}", bin_code % 1_000_000)
    }

    pub fn verificar_totp_sha256(secret: &[u8], codigo: &str, timestamp_secs: u64) -> bool {
        let codigo = codigo.trim();
        if codigo.len() != 6 || !codigo.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        let step = timestamp_secs / 30;
        let mut matched = false;
        for delta in [-1i64, 0, 1] {
            if let Some(window_step) = step.checked_add_signed(delta) {
                let expected = Self::gerar_totp_sha256(secret, window_step);
                if Self::constant_time_eq(expected.as_bytes(), codigo.as_bytes()) {
                    matched = true;
                }
            }
        }
        matched
    }

    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut diff = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            diff |= x ^ y;
        }
        diff == 0
    }
}

#[cfg(test)]
mod tests {
    use super::CryptoService;

    #[test]
    fn senha_hash_e_verificacao_em_tempo_constante() {
        let hash_seed = "$sha256$i=10000$4a6f726e616c466564536565643031$29697afdd480efff2146336ac0b3cc59e5e5eb3302a30a2b518275711fc6f3ed";
        assert!(CryptoService::verificar_senha_hash(
            "JornalFed@2026#Admin!",
            hash_seed
        ));
        assert!(!CryptoService::verificar_senha_hash(
            "senha-errada",
            hash_seed
        ));
        assert!(!CryptoService::verificar_senha_hash(
            "qualquer",
            "$argon2id$v=19$m=65536,t=3,p=4$fake$fake"
        ));
    }

    #[test]
    fn aes_gcm_envelope_roundtrip() {
        let svc = CryptoService::new(&[7u8; 32]);
        let nonce = [3u8; 12];
        let segredo = b"SEGREDO_TOTP_2FA";
        let envelope = svc.cifrar_envelope(segredo, &nonce).expect("cifra ok");
        let decifrado = svc.decifrar_envelope(&envelope).expect("decifra ok");
        assert_eq!(decifrado, segredo);
    }

    #[test]
    fn totp_sha256_valida_janela_e_rejeita_codigo_arbitrario() {
        let segredo = b"chave-totp-secreta-partido";
        let ts = 1_794_600_000u64;
        let codigo = CryptoService::gerar_totp_sha256(segredo, ts / 30);
        assert!(CryptoService::verificar_totp_sha256(segredo, &codigo, ts));
        assert!(CryptoService::verificar_totp_sha256(
            segredo,
            &codigo,
            ts + 30
        ));
        assert!(!CryptoService::verificar_totp_sha256(
            segredo, "000000", ts
        ));
    }

    #[test]
    fn rotacao_chave_aes_gcm_recriptografa_envelope_antigo() {
        let antiga = CryptoService::from_raw_secret("chave-mestra-antiga-v1");
        let nova = CryptoService::from_raw_secret("chave-mestra-nova-v2");
        let segredo = b"SEGREDO_TOTP_ROTACIONAVEL";

        let envelope_v1 = antiga
            .cifrar_envelope_aleatorio(segredo)
            .expect("cifra com v1 ok");

        // Chave nova sozinha rejeita envelope v1; com fallback decifra e sinaliza rotação
        assert!(nova.decifrar_envelope(&envelope_v1).is_err());
        let (plain, usou_anterior) = nova
            .decifrar_com_fallback(&envelope_v1, Some(&antiga))
            .expect("fallback v1 ok");
        assert!(usou_anterior);
        assert_eq!(plain, segredo);

        // Re-cifra para v2 e garante validação direta sem fallback
        let envelope_v2 = nova
            .cifrar_envelope_aleatorio(&plain)
            .expect("cifra com v2 ok");
        let (plain_v2, usou_anterior_v2) = nova
            .decifrar_com_fallback(&envelope_v2, Some(&antiga))
            .expect("v2 direta ok");
        assert!(!usou_anterior_v2);
        assert_eq!(plain_v2, segredo);
    }
}
