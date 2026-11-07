//! Structured logging system for win-smooth shell.
//! Writes timestamped log entries to a rotating file in %TEMP%.
//! Supports log levels: TRACE, DEBUG, INFO, WARN, ERROR, FATAL.
//! All shell components log through this module.

use anyhow::Result;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Log severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRACE" => Some(LogLevel::Trace),
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" => Some(LogLevel::Warn),
            "ERROR" => Some(LogLevel::Error),
            "FATAL" => Some(LogLevel::Fatal),
            _ => None,
        }
    }
}

/// A single log entry.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp_ms: u64,
    pub level: LogLevel,
    pub module: &'static str,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, module: &'static str, message: impl Into<String>) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            timestamp_ms,
            level,
            module,
            message: message.into(),
        }
    }

    pub fn format(&self) -> String {
        let seconds = (self.timestamp_ms / 1000) % 60;
        let minutes = (self.timestamp_ms / 60_000) % 60;
        let hours = (self.timestamp_ms / 3_600_000) % 24;
        let millis = self.timestamp_ms % 1000;

        format!(
            "{:02}:{:02}:{:02}.{:03} [{}] {}: {}",
            hours,
            minutes,
            seconds,
            millis,
            self.level.as_str(),
            self.module,
            self.message
        )
    }
}

/// File-backed logger with size-based rotation.
pub struct Logger {
    inner: Arc<Mutex<InnerLogger>>,
}

struct InnerLogger {
    writer: Option<BufWriter<File>>,
    log_path: PathBuf,
    max_size_bytes: u64,
    current_size_bytes: u64,
    min_level: LogLevel,
    rotation_count: u32,
}

impl Logger {
    /// Create a new logger writing to `log_path`.
    /// Rotates when the file exceeds `max_size_bytes`, keeping up to
    /// `rotation_count` older files.
    pub fn new(
        log_path: PathBuf,
        max_size_bytes: u64,
        rotation_count: u32,
        min_level: LogLevel,
    ) -> Result<Self> {
        let writer = Self::open_writer(&log_path)?;
        let current_size = log_path.metadata().map(|m| m.len()).unwrap_or(0);

        Ok(Self {
            inner: Arc::new(Mutex::new(InnerLogger {
                writer: Some(writer),
                log_path,
                max_size_bytes,
                current_size_bytes: current_size,
                min_level,
                rotation_count,
            })),
        })
    }

    fn open_writer(path: &PathBuf) -> Result<BufWriter<File>> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(path)
            .map_err(|e| anyhow::anyhow!("failed to open log file {}: {}", path.display(), e))?;
        Ok(BufWriter::new(file))
    }

    fn maybe_rotate(inner: &mut InnerLogger) -> Result<()> {
        if inner.current_size_bytes < inner.max_size_bytes {
            return Ok(());
        }

        // Close current writer
        if let Some(mut w) = inner.writer.take() {
            w.flush()?;
        }

        // Rotate existing files
        for i in (1..inner.rotation_count).rev() {
            let from = format!("{}.{}", inner.log_path.display(), i);
            let to = format!("{}.{}", inner.log_path.display(), i + 1);
            let _ = std::fs::rename(&from, &to);
        }

        // Move current to .1
        let _ = std::fs::rename(&inner.log_path, format!("{}.1", inner.log_path.display()));

        // Reopen
        let writer = Self::open_writer(&inner.log_path)?;
        inner.current_size_bytes = 0;
        inner.writer = Some(writer);

        Ok(())
    }

    /// Write a log entry.
    pub fn log(&self, level: LogLevel, module: &'static str, message: impl Into<String>) -> Result<()> {
        if (level as u8) < (self.inner.lock().unwrap().min_level as u8) {
            return Ok(());
        }

        let entry = LogEntry::new(level, module, message);
        let formatted = entry.format();

        let mut inner = self.inner.lock().unwrap();
        Self::maybe_rotate(&mut inner)?;

        if let Some(ref mut writer) = inner.writer {
            writeln!(writer, "{}", formatted)?;
            writer.flush()?;
            inner.current_size_bytes += formatted.len() as u64 + 1; // +1 for newline
        }

        Ok(())
    }

    /// Convenience: log at INFO level.
    pub fn info(&self, module: &'static str, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Info, module, message)
    }

    /// Convenience: log at WARN level.
    pub fn warn(&self, module: &'static str, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Warn, module, message)
    }

    /// Convenience: log at ERROR level.
    pub fn error(&self, module: &'static str, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Error, module, message)
    }

    /// Convenience: log at DEBUG level.
    pub fn debug(&self, module: &'static str, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Debug, module, message)
    }

    /// Convenience: log at TRACE level.
    pub fn trace(&self, module: &'static str, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Trace, module, message)
    }

    /// Convenience: log at FATAL level.
    pub fn fatal(&self, module: &'static str, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Fatal, module, message)
    }

    /// Flush pending writes.
    pub fn flush(&self) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(ref mut writer) = inner.writer {
            writer.flush()?;
        }
        Ok(())
    }
}

impl Default for Logger {
    fn default() -> Self {
        let log_path = std::env::temp_dir().join("win-smooth.log");
        Self::new(log_path, 5_000_000, 3, LogLevel::Info).expect("failed to create logger")
    }
}

// Global logger instance (set once at startup).
static GLOBAL_LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

/// Initialize the global logger. Must be called once at startup.
pub fn init_global(log_path: Option<PathBuf>, min_level: LogLevel) -> Result<&'static Logger> {
    let path = log_path.unwrap_or_else(|| std::env::temp_dir().join("win-smooth.log"));
    let logger = Logger::new(path, 5_000_000, 3, min_level)?;
    GLOBAL_LOGGER.set(logger).map_err(|_| anyhow::anyhow!("logger already initialized"))?;
    Ok(GLOBAL_LOGGER.get().expect("logger just set"))
}

/// Get the global logger instance.
pub fn global() -> Option<&'static Logger> {
    GLOBAL_LOGGER.get()
}

/// Macro for convenient logging at INFO level.
#[macro_export]
macro_rules! log_info {
    ($module:expr, $msg:expr) => {
        if let Some(logger) = crate::logging::global() {
            let _ = logger.log(crate::logging::LogLevel::Info, $module, format!($msg));
        }
    };
    ($module:expr, $fmt:expr, $($arg:tt)*) => {
        if let Some(logger) = crate::logging::global() {
            let _ = logger.log(
                crate::logging::LogLevel::Info,
                $module,
                format!($fmt, $($arg)*),
            );
        }
    };
}

/// Macro for convenient logging at WARN level.
#[macro_export]
macro_rules! log_warn {
    ($module:expr, $msg:expr) => {
        if let Some(logger) = crate::logging::global() {
            let _ = logger.log(crate::logging::LogLevel::Warn, $module, format!($msg));
        }
    };
    ($module:expr, $fmt:expr, $($arg:tt)*) => {
        if let Some(logger) = crate::logging::global() {
            let _ = logger.log(
                crate::logging::LogLevel::Warn,
                $module,
                format!($fmt, $($arg)*),
            );
        }
    };
}

/// Macro for convenient logging at ERROR level.
#[macro_export]
macro_rules! log_error {
    ($module:expr, $msg:expr) => {
        if let Some(logger) = crate::logging::global() {
            let _ = logger.log(crate::logging::LogLevel::Error, $module, format!($msg));
        }
    };
    ($module:expr, $fmt:expr, $($arg:tt)*) => {
        if let Some(logger) = crate::logging::global() {
            let _ = logger.log(
                crate::logging::LogLevel::Error,
                $module,
                format!($fmt, $($arg)*),
            );
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Fatal);
    }

    #[test]
    fn test_log_entry_format() {
        let entry = LogEntry::new(LogLevel::Info, "test", "hello world");
        let formatted = entry.format();
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("test"));
        assert!(formatted.contains("hello world"));
    }

    #[test]
    fn test_logger_write() {
        let path = temp_dir().join(format!("win-smooth-test-{}.log", std::process::id()));
        let logger = Logger::new(path.clone(), 10_000, 1, LogLevel::Info).unwrap();
        logger.log(LogLevel::Info, "test", "hello").unwrap();
        logger.flush().unwrap();

        let contents = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(contents.contains("INFO"));
        assert!(contents.contains("test"));
        assert!(contents.contains("hello"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_logger_level_filter() {
        let path = temp_dir().join(format!("win-smooth-test-{}.log", std::process::id()));
        let logger = Logger::new(path.clone(), 10_000, 1, LogLevel::Error).unwrap();
        logger.log(LogLevel::Info, "test", "should not appear").unwrap();
        logger.log(LogLevel::Error, "test", "should appear").unwrap();
        logger.flush().unwrap();

        let contents = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(!contents.contains("should not appear"));
        assert!(contents.contains("should appear"));

        let _ = std::fs::remove_file(&path);
    }
}
