//! Integrity & Tamper Detection System
//!
//! Computes and verifies hashes of critical files to detect unauthorized
//! modifications. Uses blake3 for hashing.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileIntegrity {
    pub path: PathBuf,
    pub hash: String,
    pub last_verified: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IntegrityAlert {
    pub path: PathBuf,
    pub expected_hash: String,
    pub actual_hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum IntegrityStatus {
    Verified,
    Tampered(IntegrityAlert),
    NotFound,
}

pub struct IntegrityChecker {
    known_hashes: RwLock<HashMap<PathBuf, String>>,
    alerts: RwLock<Vec<IntegrityAlert>>,
}

impl Default for IntegrityChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl IntegrityChecker {
    pub fn new() -> Self {
        Self {
            known_hashes: RwLock::new(HashMap::new()),
            alerts: RwLock::new(Vec::new()),
        }
    }

    /// Compute the blake3 hash of a file.
    pub fn hash_file(path: &Path) -> Result<String, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
        Ok(blake3::hash(&data).to_hex().to_string())
    }

    /// Store the hash of a file for later verification.
    pub fn record_hash(&self, path: PathBuf, hash: String) {
        self.known_hashes.write().insert(path, hash);
    }

    /// Hash and record a file's current state.
    pub fn hash_and_record(&self, path: PathBuf) -> Result<String, String> {
        let hash = Self::hash_file(&path)?;
        self.record_hash(path, hash.clone());
        Ok(hash)
    }

    /// Record a hash for a file by reading it from disk.
    pub fn record_file(&self, path: PathBuf) -> Result<String, String> {
        self.hash_and_record(path)
    }

    /// Verify a file has not been tampered with.
    /// Returns `IntegrityStatus::Verified` if hash matches,
    /// `IntegrityStatus::Tampered` if mismatch, or `IntegrityStatus::NotFound`
    /// if the file is missing or no hash is recorded.
    pub fn check_tampered(&self, path: &Path) -> IntegrityStatus {
        let known = {
            let hashes = self.known_hashes.read();
            hashes.get(path).cloned()
        };

        let known_hash = match known {
            Some(h) => h,
            None => return IntegrityStatus::NotFound,
        };

        let current_hash = match Self::hash_file(path) {
            Ok(h) => h,
            Err(_) => return IntegrityStatus::NotFound,
        };

        if current_hash == known_hash {
            IntegrityStatus::Verified
        } else {
            let alert = IntegrityAlert {
                path: path.to_path_buf(),
                expected_hash: known_hash,
                actual_hash: current_hash,
                timestamp: chrono::Utc::now(),
            };
            self.alerts.write().push(alert.clone());
            IntegrityStatus::Tampered(alert)
        }
    }

    /// Hash all files matching a glob pattern and record their hashes.
    pub fn hash_critical_files(
        &self,
        paths: &[PathBuf],
    ) -> HashMap<PathBuf, Result<String, String>> {
        let mut results = HashMap::new();
        for path in paths {
            let result = self.record_file(path.clone());
            results.insert(path.clone(), result);
        }
        results
    }

    /// Verify all recorded files and return tampered ones.
    pub fn verify_all(&self) -> Vec<IntegrityAlert> {
        let paths: Vec<PathBuf> = self.known_hashes.read().keys().cloned().collect();
        let mut tampered = Vec::new();
        for path in &paths {
            if let IntegrityStatus::Tampered(alert) = self.check_tampered(path) {
                tampered.push(alert);
            }
        }
        tampered
    }

    /// Drain all accumulated integrity alerts.
    pub fn drain_alerts(&self) -> Vec<IntegrityAlert> {
        self.alerts.write().drain(..).collect()
    }

    /// Get the number of tracked files.
    pub fn tracked_files(&self) -> usize {
        self.known_hashes.read().len()
    }

    /// Clear all known hashes and alerts.
    pub fn reset(&self) {
        self.known_hashes.write().clear();
        self.alerts.write().clear();
    }
}
