use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use blake3;

#[derive(Debug)]
pub struct EncryptionEngine {
    key: Option<[u8; 32]>,
}

impl Default for EncryptionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl EncryptionEngine {
    pub fn new() -> Self {
        Self { key: None }
    }

    pub fn init_with_password(&mut self, password: &str, salt: &[u8]) -> anyhow::Result<()> {
        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), salt, &mut key)
            .map_err(|e| anyhow::anyhow!("Key derivation failed: {}", e))?;
        self.key = Some(key);
        Ok(())
    }

    pub fn encrypt(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let key = self
            .key
            .ok_or_else(|| anyhow::anyhow!("Encryption key not initialized"))?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| anyhow::anyhow!("Invalid key length: {}", e))?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, data)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub fn decrypt(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let key = self
            .key
            .ok_or_else(|| anyhow::anyhow!("Encryption key not initialized"))?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| anyhow::anyhow!("Invalid key length: {}", e))?;

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
        Ok(plaintext)
    }

    pub fn hash(data: &[u8]) -> String {
        blake3::hash(data).to_hex().to_string()
    }

    pub fn verify(data: &[u8], hash: &str) -> bool {
        Self::hash(data) == hash
    }
}
