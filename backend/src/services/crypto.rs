use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::{Digest, Sha256};

#[allow(dead_code)]
pub struct CryptoService {
    cipher: Aes256Gcm,
}

#[allow(dead_code)]
impl CryptoService {
    pub fn new(chave_mestra: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new_from_slice(chave_mestra)
            .expect("Chave mestra inválida para AES-256-GCM (deve ter 32 bytes)");
        Self { cipher }
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
}
