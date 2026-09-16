use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, Generate},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};

use crate::{AppError, AppResult};

#[derive(Clone)]
pub struct CryptoService {
    cipher: Aes256Gcm,
}

impl CryptoService {
    pub fn from_base64(key: &str) -> AppResult<Self> {
        let key = STANDARD
            .decode(key)
            .map_err(|_| AppError::configuration("ENCRYPTION_KEY must be valid base64"))?;

        if key.len() != 32 {
            return Err(AppError::configuration(
                "ENCRYPTION_KEY must decode to exactly 32 bytes",
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| AppError::configuration("ENCRYPTION_KEY is invalid"))?;

        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> AppResult<Vec<u8>> {
        let nonce = aes_gcm::Nonce::generate();
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| AppError::internal(anyhow::anyhow!("encryption failed")))?;

        let mut payload = Vec::with_capacity(nonce.len() + ciphertext.len());
        payload.extend_from_slice(nonce.as_ref());
        payload.extend_from_slice(&ciphertext);
        Ok(payload)
    }

    pub fn decrypt(&self, payload: &[u8]) -> AppResult<Vec<u8>> {
        if payload.len() <= 12 {
            return Err(AppError::internal(anyhow::anyhow!(
                "encrypted payload is invalid"
            )));
        }

        let (nonce, ciphertext) = payload.split_at(12);
        let nonce_bytes: [u8; 12] = nonce
            .try_into()
            .map_err(|_| AppError::internal(anyhow::anyhow!("invalid nonce")))?;
        let nonce = aes_gcm::Nonce::from(nonce_bytes);
        self.cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|_| AppError::internal(anyhow::anyhow!("decryption failed")))
    }

    pub fn hash(value: &[u8]) -> String {
        hex::encode(Sha256::digest(value))
    }

    pub fn mask(value: &str) -> String {
        let characters: Vec<char> = value.chars().collect();
        match characters.len() {
            0 => String::new(),
            1..=4 => "••••".to_owned(),
            length => {
                let suffix: String = characters[length - 4..].iter().collect();
                format!("•••• {suffix}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> CryptoService {
        CryptoService::from_base64(&STANDARD.encode([7_u8; 32])).expect("valid key")
    }

    #[test]
    fn encrypts_with_unique_nonces_and_decrypts() {
        let service = service();
        let first = service.encrypt(b"private value").expect("encrypt");
        let second = service.encrypt(b"private value").expect("encrypt");

        assert_ne!(first, second);
        assert_eq!(service.decrypt(&first).expect("decrypt"), b"private value");
    }

    #[test]
    fn masks_sensitive_values() {
        assert_eq!(CryptoService::mask("1234567890"), "•••• 7890");
        assert_eq!(CryptoService::mask("123"), "••••");
    }

    #[test]
    fn rejects_tampered_ciphertext() {
        let service = service();
        let mut payload = service.encrypt(b"private value").expect("encrypt");
        let last = payload.len() - 1;
        payload[last] ^= 1;
        assert!(service.decrypt(&payload).is_err());
    }
}
