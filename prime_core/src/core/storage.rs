use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Storage manager for Prime's persistent data.
pub struct Storage {
    data_dir: PathBuf,
    db: Arc<Mutex<rusqlite::Connection>>,
}

impl std::fmt::Debug for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Storage")
            .field("data_dir", &self.data_dir)
            .field("db", &"<rusqlite connection>")
            .finish()
    }
}

/// Alias for backward compatibility with existing code
pub type StorageEngine = Storage;

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupManifest {
    pub version: String,
    pub created_at: String,
    pub file_count: u32,
    pub total_size: u64,
    pub checksum: String,
}

// =============================================================================
// App Settings — persisted in SQLite key-value table
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub language: String,
    pub proxy_enabled: bool,
    pub proxy_url: Option<String>,
    pub auto_backup: bool,
    pub auto_backup_interval_mins: u64,
    pub max_checkpoints: u32,
    pub telemetry_enabled: bool,
    pub default_model: Option<String>,
    pub mcp_auto_start: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            language: "en".into(),
            proxy_enabled: false,
            proxy_url: None,
            auto_backup: true,
            auto_backup_interval_mins: 60,
            max_checkpoints: 10,
            telemetry_enabled: true,
            default_model: None,
            mcp_auto_start: true,
        }
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage {
    /// Create a new Storage with data_dir from default app data directory
    pub fn new() -> Self {
        let data_dir = dirs_next::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Prime");
        Self::with_dir(data_dir)
    }

    /// Create a new Storage with a specific data directory
    pub fn with_dir(data_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");
        std::fs::create_dir_all(data_dir.join("checkpoints"))
            .expect("Failed to create checkpoints directory");
        std::fs::create_dir_all(data_dir.join("backups"))
            .expect("Failed to create backups directory");

        let db_path = data_dir.join("prime.db");
        let db = rusqlite::Connection::open(&db_path).expect("Failed to open SQLite database");

        let storage = Self {
            data_dir,
            db: Arc::new(Mutex::new(db)),
        };

        storage
            .init_settings_table()
            .expect("Failed to initialize settings table");

        storage
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Get a reference to the database mutex
    pub fn db(&self) -> Arc<Mutex<rusqlite::Connection>> {
        Arc::clone(&self.db)
    }

    /// Lock the database and get a guard
    pub fn lock(&self) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
        self.db.lock().expect("Failed to lock database mutex")
    }

    /// Execute a SQL query and return results
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<(String, f64)>, StorageError> {
        let db = self
            .db
            .lock()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        let mut stmt = db
            .prepare(query)
            .map_err(|e| StorageError::Database(e.to_string()))?;
        let results = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let score: f64 = row.get(1).unwrap_or(0.0);
                Ok((id, score))
            })
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut out = Vec::new();
        for val in results.take(limit).flatten() {
            out.push(val);
        }
        Ok(out)
    }

    // =========================================================================
    // Backup & Restore
    // =========================================================================

    /// Create a full backup of all data to a zip file.
    /// Returns the path to the backup file.
    pub fn backup(&self) -> Result<PathBuf, StorageError> {
        use blake3::Hasher;
        use chrono::Utc;
        use std::io::Write;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_dir = self.data_dir.join("backups");
        std::fs::create_dir_all(&backup_dir).map_err(|e: std::io::Error| StorageError::Io(e))?;

        let backup_path = backup_dir.join(format!("prime_backup_{}.zip", timestamp));

        // Collect files to back up
        let mut files_to_backup: Vec<(PathBuf, String)> = Vec::new();

        // Database file
        let db_path = self.data_dir.join("prime.db");
        if db_path.exists() {
            files_to_backup.push((db_path.clone(), "prime.db".to_string()));
        }

        // Config files
        let config_path = self.data_dir.join("config.json");
        if config_path.exists() {
            files_to_backup.push((config_path, "config.json".to_string()));
        }

        // Checkpoints
        let checkpoints_dir = self.data_dir.join("checkpoints");
        if checkpoints_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&checkpoints_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        files_to_backup.push((path, format!("checkpoints/{}", name)));
                    }
                }
            }
        }

        // Index directory
        let index_dir = self.data_dir.join("index");
        if index_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&index_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        files_to_backup.push((path, format!("index/{}", name)));
                    }
                }
            }
        }

        // Create zip archive
        let file =
            std::fs::File::create(&backup_path).map_err(|e: std::io::Error| StorageError::Io(e))?;
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let mut hasher = Hasher::new();
        let mut total_size: u64 = 0;
        let mut file_count: u32 = 0;

        for (src_path, arc_name) in &files_to_backup {
            let data = std::fs::read(src_path).map_err(|e: std::io::Error| StorageError::Io(e))?;
            zip.start_file(arc_name, options)
                .map_err(|e: zip::result::ZipError| StorageError::Zip(e.to_string()))?;
            zip.write_all(&data)
                .map_err(|e: std::io::Error| StorageError::Io(e))?;
            hasher.update(&data);
            total_size += data.len() as u64;
            file_count += 1;
        }

        // Write manifest
        let manifest = BackupManifest {
            version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: Utc::now().to_rfc3339(),
            file_count,
            total_size,
            checksum: hasher.finalize().to_hex().to_string(),
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        zip.start_file("manifest.json", options)
            .map_err(|e: zip::result::ZipError| StorageError::Zip(e.to_string()))?;
        zip.write_all(manifest_json.as_bytes())
            .map_err(|e: std::io::Error| StorageError::Io(e))?;

        zip.finish()
            .map_err(|e: zip::result::ZipError| StorageError::Zip(e.to_string()))?;

        tracing::info!(
            "Backup created: {} ({} files, {} bytes)",
            backup_path.display(),
            file_count,
            total_size
        );

        Ok(backup_path)
    }

    /// Restore data from a backup zip file.
    pub fn restore(&self, backup_path: &Path) -> Result<(), StorageError> {
        if !backup_path.exists() {
            return Err(StorageError::NotFound(
                backup_path.to_string_lossy().to_string(),
            ));
        }

        let file =
            std::fs::File::open(backup_path).map_err(|e: std::io::Error| StorageError::Io(e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e: zip::result::ZipError| StorageError::Zip(e.to_string()))?;

        // Create a temporary restore directory
        let restore_dir = self.data_dir.join(".restore_temp");
        if restore_dir.exists() {
            std::fs::remove_dir_all(&restore_dir)
                .map_err(|e: std::io::Error| StorageError::Io(e))?;
        }
        std::fs::create_dir_all(&restore_dir).map_err(|e: std::io::Error| StorageError::Io(e))?;

        // Extract all files
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e: zip::result::ZipError| StorageError::Zip(e.to_string()))?;
            let name = entry.name().to_string();

            // Skip manifest
            if name == "manifest.json" {
                continue;
            }

            let target_path = restore_dir.join(&name);
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e: std::io::Error| StorageError::Io(e))?;
            }

            let mut output = std::fs::File::create(&target_path)
                .map_err(|e: std::io::Error| StorageError::Io(e))?;
            std::io::copy(&mut entry, &mut output)
                .map_err(|e: std::io::Error| StorageError::Io(e))?;
        }

        // Close the current database connection
        {
            let db = self.db.lock().unwrap();
            if let Err(e) = db.execute("PRAGMA wal_checkpoint(TRUNCATE)", []) {
                tracing::warn!("Failed to checkpoint database: {}", e);
            }
        }

        // Copy restored files to data directory
        let db_restore = restore_dir.join("prime.db");
        if db_restore.exists() {
            std::fs::copy(&db_restore, self.data_dir.join("prime.db"))
                .map_err(|e: std::io::Error| StorageError::Io(e))?;
        }

        let config_restore = restore_dir.join("config.json");
        if config_restore.exists() {
            std::fs::copy(&config_restore, self.data_dir.join("config.json"))
                .map_err(|e: std::io::Error| StorageError::Io(e))?;
        }

        // Restore checkpoints
        let checkpoints_restore = restore_dir.join("checkpoints");
        if checkpoints_restore.exists() {
            let target = self.data_dir.join("checkpoints");
            std::fs::create_dir_all(&target).map_err(|e: std::io::Error| StorageError::Io(e))?;
            for entry in std::fs::read_dir(&checkpoints_restore)
                .map_err(|e: std::io::Error| StorageError::Io(e))?
            {
                let entry = entry.map_err(|e: std::io::Error| StorageError::Io(e))?;
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().unwrap();
                    std::fs::copy(&path, target.join(name))
                        .map_err(|e: std::io::Error| StorageError::Io(e))?;
                }
            }
        }

        // Clean up
        std::fs::remove_dir_all(&restore_dir).map_err(|e: std::io::Error| StorageError::Io(e))?;

        // Reopen database connection
        {
            let db_path = self.data_dir.join("prime.db");
            let new_db = rusqlite::Connection::open(&db_path)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let mut db = self.db.lock().unwrap();
            *db = new_db;
        }

        tracing::info!("Restore complete from: {}", backup_path.display());

        Ok(())
    }

    /// Auto-backup before major operations.
    /// Creates a checkpoint backup in the checkpoints directory.
    pub fn auto_backup(&self, label: &str) -> Result<PathBuf, StorageError> {
        let checkpoints_dir = self.data_dir.join("checkpoints");
        std::fs::create_dir_all(&checkpoints_dir)
            .map_err(|e: std::io::Error| StorageError::Io(e))?;

        use chrono::Utc;
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let safe_label = label.replace(|c: char| !c.is_alphanumeric() && c != '-', "_");
        let checkpoint_path = checkpoints_dir.join(format!("{}_{}.zip", timestamp, safe_label));

        // Quick backup of just the database
        let file = std::fs::File::create(&checkpoint_path)
            .map_err(|e: std::io::Error| StorageError::Io(e))?;
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let db_path = self.data_dir.join("prime.db");
        if db_path.exists() {
            let data = std::fs::read(&db_path).map_err(|e: std::io::Error| StorageError::Io(e))?;
            zip.start_file("prime.db", options)
                .map_err(|e: zip::result::ZipError| StorageError::Zip(e.to_string()))?;
            use std::io::Write;
            zip.write_all(&data)
                .map_err(|e: std::io::Error| StorageError::Io(e))?;
        }

        // Write checkpoint manifest
        let manifest = serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "label": label,
            "created_at": Utc::now().to_rfc3339(),
        });

        let manifest_str = serde_json::to_string_pretty(&manifest)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        zip.start_file("checkpoint.json", options)
            .map_err(|e: zip::result::ZipError| StorageError::Zip(e.to_string()))?;
        use std::io::Write;
        zip.write_all(manifest_str.as_bytes())
            .map_err(|e: std::io::Error| StorageError::Io(e))?;

        zip.finish()
            .map_err(|e: zip::result::ZipError| StorageError::Zip(e.to_string()))?;

        // Keep only last 10 checkpoints
        self.prune_checkpoints(10)?;

        tracing::info!("Auto-backup created: {}", checkpoint_path.display());

        Ok(checkpoint_path)
    }

    /// Prune old checkpoints, keeping only the most recent `keep` count.
    fn prune_checkpoints(&self, keep: usize) -> Result<(), StorageError> {
        let checkpoints_dir = self.data_dir.join("checkpoints");
        if !checkpoints_dir.exists() {
            return Ok(());
        }

        let mut entries: Vec<_> = std::fs::read_dir(&checkpoints_dir)
            .map_err(|e: std::io::Error| StorageError::Io(e))?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "zip")
                    .unwrap_or(false)
            })
            .collect();

        entries.sort_by_key(|e| e.path().metadata().and_then(|m| m.modified()).ok());

        if entries.len() > keep {
            for entry in entries.iter().take(entries.len() - keep) {
                if let Err(e) = std::fs::remove_file(entry.path()) {
                    tracing::warn!("Failed to remove old checkpoint: {}", e);
                }
            }
        }

        Ok(())
    }

    // =========================================================================
    // Settings Persistence
    // =========================================================================

    /// Create the settings key-value table if it does not exist.
    fn init_settings_table(&self) -> Result<(), StorageError> {
        let conn = self.lock();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;
        tracing::debug!("Settings table initialized");
        Ok(())
    }

    /// Persist all settings via a single JSON blob under the key "app_settings".
    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), StorageError> {
        let json =
            serde_json::to_string(settings).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let conn = self.lock();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('app_settings', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [&json],
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;
        tracing::debug!("Settings saved");
        Ok(())
    }

    /// Load persisted settings, returning defaults when nothing is stored.
    pub fn load_settings(&self) -> Result<AppSettings, StorageError> {
        let conn = self.lock();
        let result: Result<String, _> = conn.query_row(
            "SELECT value FROM settings WHERE key = 'app_settings'",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(json) => serde_json::from_str(&json)
                .map_err(|e| StorageError::Serialization(e.to_string())),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(AppSettings::default()),
            Err(e) => Err(StorageError::Database(e.to_string())),
        }
    }
}

// =============================================================================
// Error Type
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Zip error: {0}")]
    Zip(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl From<zip::result::ZipError> for StorageError {
    fn from(e: zip::result::ZipError) -> Self {
        StorageError::Zip(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("prime_data");
        std::fs::create_dir_all(&data_dir).unwrap();

        // Create a dummy database file
        let db_path = data_dir.join("prime.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", [])
            .unwrap();
        conn.execute("INSERT INTO test (value) VALUES ('hello')", [])
            .unwrap();

        let storage = Storage::with_dir(data_dir.clone());

        // Create backup
        let backup_path = storage.backup().unwrap();
        assert!(backup_path.exists(), "Backup file should exist");

        // Modify the database
        conn.execute("INSERT INTO test (value) VALUES ('world')", [])
            .unwrap();

        // Restore
        storage.restore(&backup_path).unwrap();

        // Verify data is restored
        let restored_conn = rusqlite::Connection::open(&db_path).unwrap();
        let count: i64 = restored_conn
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "Should have 1 row after restore");
    }

    #[test]
    fn test_auto_backup() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("prime_data");
        std::fs::create_dir_all(&data_dir).unwrap();

        // Create a dummy database
        let db_path = data_dir.join("prime.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", [])
            .unwrap();

        let storage = Storage::with_dir(data_dir.clone());

        // Create auto-backups
        let bp1 = storage.auto_backup("before_migration").unwrap();
        let bp2 = storage.auto_backup("before_index").unwrap();

        assert!(bp1.exists(), "Auto-backup should exist");
        assert!(bp2.exists(), "Auto-backup should exist");

        // Check checkpoints directory
        let checkpoints_dir = data_dir.join("checkpoints");
        assert!(
            checkpoints_dir.exists(),
            "Checkpoints directory should exist"
        );

        let count = std::fs::read_dir(&checkpoints_dir).unwrap().count();
        assert_eq!(count, 2, "Should have 2 checkpoint files");
    }

    #[test]
    fn test_prune_checkpoints() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("prime_data");
        std::fs::create_dir_all(&data_dir).unwrap();

        let db_path = data_dir.join("prime.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", [])
            .unwrap();

        let storage = Storage::with_dir(data_dir.clone());

        // Create more checkpoints than the keep limit
        for i in 0..15 {
            storage.auto_backup(&format!("backup_{}", i)).unwrap();
        }

        // Should have kept only 10
        let checkpoints_dir = data_dir.join("checkpoints");
        let count = std::fs::read_dir(&checkpoints_dir).unwrap().count();
        assert!(
            count <= 10,
            "Should have at most 10 checkpoints, got {}",
            count
        );
    }
}
