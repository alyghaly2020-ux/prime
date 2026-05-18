//! Plugin signing and verification system.
//!
//! Uses blake3 for hashing and ed25519-dalek for asymmetric signature verification.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};

/// Plugin signing and verification system
pub struct PluginSigning {
    trusted_publishers: RwLock<HashSet<String>>,
    public_keys: RwLock<HashMap<String, Vec<u8>>>,
}

impl Default for PluginSigning {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginSigning {
    pub fn new() -> Self {
        Self {
            trusted_publishers: RwLock::new(HashSet::new()),
            public_keys: RwLock::new(HashMap::new()),
        }
    }

    /// Verify a plugin manifest signature using ed25519-dalek.
    ///
    /// The manifest is first hashed with blake3, then the signature is verified
    /// against that hash using the provided ed25519 public key.
    pub fn verify_signature(manifest: &[u8], signature: &[u8], public_key: Option<&[u8]>) -> bool {
        if signature.len() != 64 {
            return false;
        }

        let key_bytes = match public_key {
            Some(key) if key.len() == 32 => key,
            _ => return false,
        };

        let key_array: &[u8; 32] = key_bytes.try_into().expect("key length already checked");

        let Ok(verifying_key) = VerifyingKey::from_bytes(key_array) else {
            return false;
        };

        let Ok(sig) = Signature::from_slice(signature) else {
            return false;
        };

        let hash = blake3::hash(manifest);
        verifying_key.verify(hash.as_bytes(), &sig).is_ok()
    }

    /// Hash a manifest for signing
    pub fn hash_manifest(manifest: &[u8]) -> [u8; 32] {
        *blake3::hash(manifest).as_bytes()
    }

    /// Check if a publisher is trusted
    pub fn is_trusted(&self, publisher: &str) -> bool {
        self.trusted_publishers.read().contains(publisher)
    }

    /// Add a trusted publisher with their public key
    pub fn add_trusted(&self, publisher: String, public_key: Vec<u8>) {
        self.trusted_publishers.write().insert(publisher.clone());
        self.public_keys.write().insert(publisher, public_key);
    }

    /// Remove a trusted publisher
    pub fn remove_trusted(&self, publisher: &str) {
        self.trusted_publishers.write().remove(publisher);
        self.public_keys.write().remove(publisher);
    }

    /// Get public key for a publisher
    pub fn get_public_key(&self, publisher: &str) -> Option<Vec<u8>> {
        self.public_keys.read().get(publisher).cloned()
    }

    /// List all trusted publishers
    pub fn list_trusted(&self) -> Vec<String> {
        self.trusted_publishers.read().iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    #[test]
    fn test_hash_manifest() {
        let manifest = b"test manifest content";
        let hash = PluginSigning::hash_manifest(manifest);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_verify_signature_valid() {
        let manifest = b"test manifest";
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let hash = PluginSigning::hash_manifest(manifest);
        let signature = signing_key.sign(&hash);

        assert!(PluginSigning::verify_signature(
            manifest,
            &signature.to_bytes(),
            Some(&verifying_key.to_bytes()),
        ));
    }

    #[test]
    fn test_verify_signature_invalid() {
        let manifest = b"test manifest";
        let signing_key1 = SigningKey::generate(&mut OsRng);
        let signing_key2 = SigningKey::generate(&mut OsRng);

        let hash = PluginSigning::hash_manifest(manifest);
        let signature = signing_key1.sign(&hash);

        assert!(!PluginSigning::verify_signature(
            manifest,
            &signature.to_bytes(),
            Some(&signing_key2.verifying_key().to_bytes()),
        ));
    }

    #[test]
    fn test_verify_signature_wrong_length() {
        let manifest = b"test manifest";
        let key = [0u8; 32];
        assert!(!PluginSigning::verify_signature(
            manifest,
            &[0u8; 16],
            Some(&key),
        ));
    }

    #[test]
    fn test_trusted_publishers() {
        let signing = PluginSigning::new();
        signing.add_trusted("prime-labs".to_string(), vec![0u8; 32]);
        assert!(signing.is_trusted("prime-labs"));
        assert!(!signing.is_trusted("unknown"));

        signing.remove_trusted("prime-labs");
        assert!(!signing.is_trusted("prime-labs"));
    }
}
