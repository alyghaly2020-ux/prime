use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, span, warn, Level, Span};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use uuid::Uuid;

const MAX_LOG_ENTRIES: usize = 500;

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub id: String,
    pub level: String,
    pub message: String,
    pub target: String,
    pub file: String,
    pub line: u32,
    pub timestamp: String,
}

pub struct LogContext {
    span: Span,
}

impl LogContext {
    /// Enter the span context, returning a guard that exits on drop.
    pub fn enter(&self) -> tracing::span::Entered<'_> {
        self.span.enter()
    }

    /// Attach a key-value pair to this context.
    pub fn with(&self, key: &str, value: &str) -> Self {
        let new_span = span!(parent: &self.span, Level::INFO, "ctx", key = %key, value = %value);
        Self { span: new_span }
    }
}

pub struct StructuredLogger {
    session_id: String,
    file_writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    logs: Arc<Mutex<Vec<LogEntry>>>,
    _guard: Option<WorkerGuard>,
    initialized: AtomicBool,
}

impl StructuredLogger {
    pub fn new() -> Self {
        let session_id = Uuid::new_v4().to_string();

        // Set up file-rotation appender
        let log_dir = dirs_next::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("prime")
            .join("logs");
        std::fs::create_dir_all(&log_dir).ok();

        let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "prime.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        Self {
            session_id,
            file_writer: Some(Arc::new(Mutex::new(Box::new(non_blocking)))),
            logs: Arc::new(Mutex::new(Vec::with_capacity(MAX_LOG_ENTRIES))),
            _guard: Some(guard),
            initialized: AtomicBool::new(false),
        }
    }

    /// The session-unique trace identifier.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    // ------------------------------------------------------------------
    // Level-based log helpers
    // ------------------------------------------------------------------

    pub fn log_error(&self, msg: &str) {
        error!(session_id = %self.session_id, "{msg}");
        self.push_log("ERROR".to_string(), msg.to_string());
        self.write_file("ERROR", msg);
    }

    pub fn log_info(&self, msg: &str) {
        info!(session_id = %self.session_id, "{msg}");
        self.push_log("INFO".to_string(), msg.to_string());
        self.write_file("INFO", msg);
    }

    pub fn log_warn(&self, msg: &str) {
        warn!(session_id = %self.session_id, "{msg}");
        self.push_log("WARN".to_string(), msg.to_string());
        self.write_file("WARN", msg);
    }

    pub fn log_debug(&self, msg: &str) {
        debug!(session_id = %self.session_id, "{msg}");
        self.push_log("DEBUG".to_string(), msg.to_string());
        self.write_file("DEBUG", msg);
    }

    // ------------------------------------------------------------------
    // Structured context
    // ------------------------------------------------------------------

    /// Create a child span that carries a key-value pair.  
    /// Use the returned [`LogContext`] to attach further context.
    pub fn with_context(&self, key: &str, value: &str) -> LogContext {
        let sp = span!(Level::INFO, "structured_log", session_id = %self.session_id, key = %key, value = %value);
        LogContext { span: sp }
    }

    // ------------------------------------------------------------------
    // One-time global initialisation (file-rotation layer)
    // ------------------------------------------------------------------

    /// Initialise the file-rotation **non_blocking** writer once.  
    /// Safe to call multiple times – only the first call has an effect.
    pub fn init_global(&self) {
        if !self.initialized.swap(true, Ordering::SeqCst) {
            info!(
                session_id = %self.session_id,
                "StructuredLogger file rotation initialised"
            );
        }
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // In-memory log buffer
    // ------------------------------------------------------------------

    #[track_caller]
    fn push_log(&self, level: String, message: String) {
        let location = std::panic::Location::caller();
        let entry = LogEntry {
            id: Uuid::new_v4().to_string(),
            level,
            message,
            target: module_path!().to_string(),
            file: location.file().to_string(),
            line: location.line(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let mut logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        logs.push(entry);
        if logs.len() > MAX_LOG_ENTRIES {
            logs.remove(0);
        }
    }

    pub fn get_recent(&self, n: usize) -> Vec<LogEntry> {
        let logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        let len = logs.len();
        if len == 0 {
            return Vec::new();
        }
        let start = len.saturating_sub(n);
        logs[start..].to_vec()
    }

    pub fn get_logs_json(&self) -> Result<String, String> {
        let entries = self.get_recent(100);
        let logs: Vec<serde_json::Value> = entries
            .into_iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "level": e.level,
                    "message": e.message,
                    "target": e.target,
                    "file": e.file,
                    "line": e.line,
                    "timestamp": e.timestamp
                })
            })
            .collect();
        serde_json::to_string(&logs).map_err(|e| e.to_string())
    }

    // ------------------------------------------------------------------
    // File I/O
    // ------------------------------------------------------------------

    fn write_file(&self, level: &str, msg: &str) {
        if let Some(ref w) = self.file_writer {
            if let Ok(mut writer) = w.try_lock() {
                let _ = writeln!(
                    writer,
                    "[{}][{}][{}] {}",
                    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                    level,
                    self.session_id,
                    msg
                );
            }
        }
    }
}

impl Default for StructuredLogger {
    fn default() -> Self {
        Self::new()
    }
}
