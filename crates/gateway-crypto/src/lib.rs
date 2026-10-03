//! AES-256-GCM encryption for provider API keys at rest, behind the
//! `KeyEnvelopeCipher` trait so a future KMS/Vault-backed implementation can
//! replace `EnvKeyCipher` without changing callers or the stored envelope
//! shape (the `key_version` field is what makes that migration possible).

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use secrecy::SecretString;
use zeroize::Zeroizing;

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("invalid master key: {0}")]
    InvalidKey(String),
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed")]
    Decrypt,
}

/// A ciphertext + its nonce, ready to persist. `key_version` records which
/// envelope key/scheme produced it, so decryption can dispatch correctly
/// even after a future key rotation or KMS migration.
#[derive(Debug, Clone)]
pub struct EncryptedBlob {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub key_version: i16,
}

pub trait KeyEnvelopeCipher: Send + Sync {
    fn encrypt(&self, plaintext: &str) -> Result<EncryptedBlob, CryptoError>;
    fn decrypt(&self, blob: &EncryptedBlob) -> Result<SecretString, CryptoError>;
}

/// Phase 1 implementation: a single master key loaded from an env var
/// (base64-encoded, 32 raw bytes). Always writes/reads `key_version = 1`.
pub struct EnvKeyCipher {
    key: [u8; KEY_LEN],
}

impl EnvKeyCipher {
    pub fn from_base64(master_key_b64: &str) -> Result<Self, CryptoError> {
        let bytes = BASE64
            .decode(master_key_b64.trim())
            .map_err(|e| CryptoError::InvalidKey(e.to_string()))?;
        if bytes.len() != KEY_LEN {
            return Err(CryptoError::InvalidKey(format!(
                "expected {KEY_LEN} raw bytes, got {}",
                bytes.len()
            )));
        }
        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(&bytes);
        Ok(Self { key })
    }
}

impl KeyEnvelopeCipher for EnvKeyCipher {
    fn encrypt(&self, plaintext: &str) -> Result<EncryptedBlob, CryptoError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));

        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| CryptoError::Encrypt)?;

        Ok(EncryptedBlob {
            nonce: nonce_bytes.to_vec(),
            ciphertext,
            key_version: 1,
        })
    }

    fn decrypt(&self, blob: &EncryptedBlob) -> Result<SecretString, CryptoError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let nonce = Nonce::from_slice(&blob.nonce);

        let plaintext = Zeroizing::new(
            cipher
                .decrypt(nonce, blob.ciphertext.as_ref())
                .map_err(|_| CryptoError::Decrypt)?,
        );
        let text = std::str::from_utf8(&plaintext).map_err(|_| CryptoError::Decrypt)?;
        Ok(SecretString::new(text.to_string()))
    }
}

impl From<CryptoError> for gateway_common::AppError {
    fn from(e: CryptoError) -> Self {
        gateway_common::AppError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;
    use std::collections::HashSet;

    fn test_cipher() -> EnvKeyCipher {
        let key_b64 = BASE64.encode([7u8; KEY_LEN]);
        EnvKeyCipher::from_base64(&key_b64).unwrap()
    }

    #[test]
    fn round_trip_encrypt_decrypt() {
        let cipher = test_cipher();
        for secret in [
            "sk-abc123",
            "a very long api key with special chars !@#$%^&*()",
            "",
        ] {
            let blob = cipher.encrypt(secret).unwrap();
            let decrypted = cipher.decrypt(&blob).unwrap();
            assert_eq!(decrypted.expose_secret(), secret);
        }
    }

    #[test]
    fn nonces_are_unique_across_many_encryptions() {
        let cipher = test_cipher();
        let mut nonces = HashSet::new();
        for _ in 0..1000 {
            let blob = cipher.encrypt("sk-test-key").unwrap();
            assert!(
                nonces.insert(blob.nonce.clone()),
                "nonce collision detected"
            );
        }
    }

    #[test]
    fn rejects_master_key_of_wrong_length() {
        let short_key = BASE64.encode([1u8; 16]);
        assert!(EnvKeyCipher::from_base64(&short_key).is_err());
    }

    #[test]
    fn rejects_non_base64_master_key() {
        assert!(EnvKeyCipher::from_base64("not-valid-base64!!!").is_err());
    }

    #[test]
    fn tampered_ciphertext_fails_to_decrypt() {
        let cipher = test_cipher();
        let mut blob = cipher.encrypt("sk-test-key").unwrap();
        blob.ciphertext[0] ^= 0xFF;
        assert!(cipher.decrypt(&blob).is_err());
    }

    #[test]
    fn ciphertext_never_contains_the_plaintext_bytes() {
        let cipher = test_cipher();
        let secret = "sk-super-secret-value-1234";
        let blob = cipher.encrypt(secret).unwrap();
        let ciphertext_str = String::from_utf8_lossy(&blob.ciphertext);
        assert!(!ciphertext_str.contains(secret));
    }
}
