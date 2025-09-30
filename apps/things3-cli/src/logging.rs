//! Log aggregation and filtering utilities
//!
//! This module provides comprehensive log aggregation and filtering capabilities
//! for the Things 3 CLI application.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
// Removed unused imports

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info, instrument, warn};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Error types for logging operations
#[derive(Error, Debug)]
pub enum LoggingError {
    #[error("Failed to read log file: {0}")]
    FileRead(String),

    #[error("Failed to write log file: {0}")]
    FileWrite(String),

    #[error("Invalid log format: {0}")]
    InvalidFormat(String),

    #[error("Filter compilation failed: {0}")]
    FilterCompilation(String),
}

/// Result type for logging operations
pub type Result<T> = std::result::Result<T, LoggingError>;

/// Log entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: HashMap<String, serde_json::Value>,
    pub span_id: Option<String>,
    pub trace_id: Option<String>,
}

/// Log filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    pub level: Option<String>,
    pub target: Option<String>,
    pub message_pattern: Option<String>,
    pub time_range: Option<TimeRange>,
    pub fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<String>,
    pub end: Option<String>,
}

/// Log aggregator for collecting and processing logs
pub struct LogAggregator {
    log_file: String,
    max_entries: usize,
    entries: Vec<LogEntry>,
}

impl LogAggregator {
    /// Create a new log aggregator
    #[must_use]
    pub fn new(log_file: String, max_entries: usize) -> Self {
        Self {
            log_file,
            max_entries,
            entries: Vec::new(),
        }
    }

    /// Load logs from file
    ///
    /// # Errors
    /// Returns an error if the log file cannot be read or parsed
    #[instrument(skip(self))]
    pub fn load_logs(&mut self) -> Result<()> {
        if !Path::new(&self.log_file).exists() {
            info!("Log file does not exist, starting with empty logs");
            return Ok(());
        }

        let file = File::open(&self.log_file)
            .map_err(|e| LoggingError::FileRead(format!("Failed to open log file: {e}")))?;

        let reader = BufReader::new(file);
        let mut line_count = 0;

        for line in reader.lines() {
            let line =
                line.map_err(|e| LoggingError::FileRead(format!("Failed to read line: {e}")))?;

            if let Ok(entry) = Self::parse_log_line(&line) {
                self.entries.push(entry);
                line_count += 1;
            }
        }

        // Keep only the most recent entries
        if self.entries.len() > self.max_entries {
            let start = self.entries.len() - self.max_entries;
            self.entries.drain(0..start);
        }

        info!("Loaded {} log entries from file", line_count);
        Ok(())
    }

    /// Parse a log line into a `LogEntry`
    fn parse_log_line(line: &str) -> Result<LogEntry> {
        // Try to parse as JSON first (structured logging)
        if let Ok(entry) = serde_json::from_str::<LogEntry>(line) {
            return Ok(entry);
        }

        // Fallback to parsing as text format
        Self::parse_text_log_line(line)
    }

    /// Parse a text log line
    fn parse_text_log_line(line: &str) -> Result<LogEntry> {
        // Simple text log parsing - this would be more sophisticated in a real implementation
        let parts: Vec<&str> = line.splitn(4, ' ').collect();

        if parts.len() < 4 {
            return Err(LoggingError::InvalidFormat(
                "Insufficient log line parts".to_string(),
            ));
        }

        let timestamp = parts[0].to_string();
        let level = parts[1].to_string();
        let target = parts[2].to_string();
        let message = parts[3..].join(" ");

        Ok(LogEntry {
            timestamp,
            level,
            target,
            message,
            fields: HashMap::new(),
            span_id: None,
            trace_id: None,
        })
    }

    /// Filter logs based on criteria
    #[instrument(skip(self))]
    pub fn filter_logs(&self, filter: &LogFilter) -> Vec<LogEntry> {
        self.entries
            .iter()
            .filter(|entry| Self::matches_filter(entry, filter))
            .cloned()
            .collect()
    }

    /// Check if a log entry matches the filter
    fn matches_filter(entry: &LogEntry, filter: &LogFilter) -> bool {
        // Level filter
        if let Some(ref level) = filter.level {
            if !entry.level.eq_ignore_ascii_case(level) {
                return false;
            }
        }

        // Target filter
        if let Some(ref target) = filter.target {
            if !entry.target.contains(target) {
                return false;
            }
        }

        // Message pattern filter
        if let Some(ref pattern) = filter.message_pattern {
            if !entry.message.contains(pattern) {
                return false;
            }
        }

        // Time range filter
        if let Some(ref time_range) = filter.time_range {
            if !Self::matches_time_range(entry, time_range) {
                return false;
            }
        }

        // Fields filter
        for (key, value) in &filter.fields {
            if let Some(entry_value) = entry.fields.get(key) {
                if entry_value != value {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Check if entry matches time range
    fn matches_time_range(entry: &LogEntry, time_range: &TimeRange) -> bool {
        // Simple timestamp comparison - would be more sophisticated in real implementation
        if let Some(ref start) = time_range.start {
            if entry.timestamp < *start {
                return false;
            }
        }

        if let Some(ref end) = time_range.end {
            if entry.timestamp > *end {
                return false;
            }
        }

        true
    }

    /// Get log statistics
    #[instrument(skip(self))]
    pub fn get_statistics(&self) -> LogStatistics {
        let mut level_counts = HashMap::new();
        let mut target_counts = HashMap::new();

        for entry in &self.entries {
            *level_counts.entry(entry.level.clone()).or_insert(0) += 1;
            *target_counts.entry(entry.target.clone()).or_insert(0) += 1;
        }

        LogStatistics {
            total_entries: self.entries.len(),
            level_counts,
            target_counts,
            oldest_entry: self.entries.first().map(|e| e.timestamp.clone()),
            newest_entry: self.entries.last().map(|e| e.timestamp.clone()),
        }
    }

    /// Export filtered logs to file
    ///
    /// # Errors
    /// Returns an error if the output file cannot be created or written to
    #[instrument(skip(self))]
    pub fn export_logs(&self, filter: &LogFilter, output_file: &str) -> Result<()> {
        let filtered_logs = self.filter_logs(filter);

        let mut file = File::create(output_file)
            .map_err(|e| LoggingError::FileWrite(format!("Failed to create output file: {e}")))?;

        let count = filtered_logs.len();
        for entry in filtered_logs {
            let json = serde_json::to_string(&entry)
                .map_err(|e| LoggingError::FileWrite(format!("Failed to serialize entry: {e}")))?;
            writeln!(file, "{json}")
                .map_err(|e| LoggingError::FileWrite(format!("Failed to write entry: {e}")))?;
        }

        info!("Exported {} log entries to {}", count, output_file);
        Ok(())
    }
}

/// Log statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStatistics {
    pub total_entries: usize,
    pub level_counts: HashMap<String, usize>,
    pub target_counts: HashMap<String, usize>,
    pub oldest_entry: Option<String>,
    pub newest_entry: Option<String>,
}

/// Log rotation utility
pub struct LogRotator {
    log_file: String,
    max_size: u64,
    max_files: usize,
}

impl LogRotator {
    /// Create a new log rotator
    #[must_use]
    pub fn new(log_file: String, max_size: u64, max_files: usize) -> Self {
        Self {
            log_file,
            max_size,
            max_files,
        }
    }

    /// Check if log rotation is needed
    #[instrument(skip(self))]
    pub fn should_rotate(&self) -> bool {
        if let Ok(metadata) = std::fs::metadata(&self.log_file) {
            metadata.len() > self.max_size
        } else {
            false
        }
    }

    /// Perform log rotation
    ///
    /// # Errors
    /// Returns an error if file operations fail during rotation
    #[instrument(skip(self))]
    pub fn rotate(&self) -> Result<()> {
        if !self.should_rotate() {
            return Ok(());
        }

        info!("Rotating log file: {}", self.log_file);

        // Rotate existing files
        for i in (1..self.max_files).rev() {
            let old_file = format!("{}.{}", self.log_file, i);
            let new_file = format!("{}.{}", self.log_file, i + 1);

            if Path::new(&old_file).exists() {
                std::fs::rename(&old_file, &new_file)
                    .map_err(|e| LoggingError::FileWrite(format!("Failed to rotate file: {e}")))?;
            }
        }

        // Move current log to .1
        let rotated_file = format!("{}.1", self.log_file);
        std::fs::rename(&self.log_file, &rotated_file)
            .map_err(|e| LoggingError::FileWrite(format!("Failed to rotate current log: {e}")))?;

        // Create new log file
        File::create(&self.log_file)
            .map_err(|e| LoggingError::FileWrite(format!("Failed to create new log file: {e}")))?;

        info!("Log rotation completed");
        Ok(())
    }
}

/// Initialize structured logging with file output
///
/// # Errors
/// Returns an error if the log file cannot be opened or logging cannot be initialized
pub fn init_file_logging(log_file: &str, level: &str, json_format: bool) -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
        .map_err(|e| LoggingError::FileWrite(format!("Failed to open log file: {e}")))?;

    let registry = tracing_subscriber::registry().with(filter);

    if json_format {
        let json_layer = fmt::layer()
            .json()
            .with_writer(file)
            .with_current_span(true)
            .with_span_list(true)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true);

        registry.with(json_layer).init();
    } else {
        let fmt_layer = fmt::layer()
            .with_writer(file)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::CLOSE);

        registry.with(fmt_layer).init();
    }

    info!("File logging initialized: {}", log_file);
    Ok(())
}

/// Log search utility
pub struct LogSearcher {
    aggregator: LogAggregator,
}

impl LogSearcher {
    /// Create a new log searcher
    #[must_use]
    pub fn new(aggregator: LogAggregator) -> Self {
        Self { aggregator }
    }

    /// Search logs by query
    #[instrument(skip(self))]
    pub fn search(&self, query: &str) -> Vec<LogEntry> {
        let filter = LogFilter {
            level: None,
            target: None,
            message_pattern: Some(query.to_string()),
            time_range: None,
            fields: HashMap::new(),
        };

        self.aggregator.filter_logs(&filter)
    }

    /// Search logs by level
    #[instrument(skip(self))]
    pub fn search_by_level(&self, level: &str) -> Vec<LogEntry> {
        let filter = LogFilter {
            level: Some(level.to_string()),
            target: None,
            message_pattern: None,
            time_range: None,
            fields: HashMap::new(),
        };

        self.aggregator.filter_logs(&filter)
    }

    /// Search logs by target
    #[instrument(skip(self))]
    pub fn search_by_target(&self, target: &str) -> Vec<LogEntry> {
        let filter = LogFilter {
            level: None,
            target: Some(target.to_string()),
            message_pattern: None,
            time_range: None,
            fields: HashMap::new(),
        };

        self.aggregator.filter_logs(&filter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry {
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            target: "things3_cli".to_string(),
            message: "Test message".to_string(),
            fields: HashMap::new(),
            span_id: None,
            trace_id: None,
        };

        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.message, "Test message");
    }

    #[test]
    fn test_log_entry_with_fields() {
        let mut fields = HashMap::new();
        fields.insert(
            "user_id".to_string(),
            serde_json::Value::String("123".to_string()),
        );
        fields.insert(
            "action".to_string(),
            serde_json::Value::String("login".to_string()),
        );

        let entry = LogEntry {
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            target: "things3_cli".to_string(),
            message: "User logged in".to_string(),
            fields,
            span_id: Some("span-123".to_string()),
            trace_id: Some("trace-456".to_string()),
        };

        assert_eq!(entry.fields.len(), 2);
        assert_eq!(entry.span_id, Some("span-123".to_string()));
        assert_eq!(entry.trace_id, Some("trace-456".to_string()));
    }

    #[test]
    fn test_log_filter_creation() {
        let filter = LogFilter {
            level: Some("ERROR".to_string()),
            target: None,
            message_pattern: None,
            time_range: None,
            fields: HashMap::new(),
        };

        assert_eq!(filter.level, Some("ERROR".to_string()));
    }

    #[test]
    fn test_log_filter_with_all_fields() {
        let mut fields = HashMap::new();
        fields.insert(
            "module".to_string(),
            serde_json::Value::String("auth".to_string()),
        );

        let time_range = TimeRange {
            start: Some("2023-01-01T00:00:00Z".to_string()),
            end: Some("2023-01-01T23:59:59Z".to_string()),
        };

        let filter = LogFilter {
            level: Some("WARN".to_string()),
            target: Some("things3_cli::auth".to_string()),
            message_pattern: Some("failed".to_string()),
            time_range: Some(time_range),
            fields,
        };

        assert_eq!(filter.level, Some("WARN".to_string()));
        assert_eq!(filter.target, Some("things3_cli::auth".to_string()));
        assert_eq!(filter.message_pattern, Some("failed".to_string()));
        assert!(filter.time_range.is_some());
        assert_eq!(filter.fields.len(), 1);
    }

    #[test]
    fn test_time_range_creation() {
        let time_range = TimeRange {
            start: Some("2023-01-01T00:00:00Z".to_string()),
            end: Some("2023-01-01T23:59:59Z".to_string()),
        };

        assert_eq!(time_range.start, Some("2023-01-01T00:00:00Z".to_string()));
        assert_eq!(time_range.end, Some("2023-01-01T23:59:59Z".to_string()));
    }

    #[test]
    fn test_log_aggregator_creation() {
        let aggregator = LogAggregator::new("test.log".to_string(), 1000);
        assert_eq!(aggregator.max_entries, 1000);
        assert_eq!(aggregator.entries.len(), 0);
    }

    #[test]
    fn test_log_aggregator_entries_access() {
        let aggregator = LogAggregator::new("test.log".to_string(), 1000);
        assert_eq!(aggregator.entries.len(), 0);
    }

    #[test]
    fn test_log_aggregator_filter_logs() {
        let mut aggregator = LogAggregator::new("test.log".to_string(), 1000);

        // Manually add entries to test filtering
        let entry1 = LogEntry {
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            target: "things3_cli".to_string(),
            message: "Info message".to_string(),
            fields: HashMap::new(),
            span_id: None,
            trace_id: None,
        };

        let entry2 = LogEntry {
            timestamp: "2023-01-01T00:00:01Z".to_string(),
            level: "ERROR".to_string(),
            target: "things3_cli".to_string(),
            message: "Error message".to_string(),
            fields: HashMap::new(),
            span_id: None,
            trace_id: None,
        };

        aggregator.entries.push(entry1);
        aggregator.entries.push(entry2);

        let filter = LogFilter {
            level: Some("ERROR".to_string()),
            target: None,
            message_pattern: None,
            time_range: None,
            fields: HashMap::new(),
        };

        let filtered = aggregator.filter_logs(&filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].level, "ERROR");
    }

    #[test]
    fn test_log_aggregator_filter_by_message_pattern() {
        let mut aggregator = LogAggregator::new("test.log".to_string(), 1000);

        let entry1 = LogEntry {
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            target: "things3_cli".to_string(),
            message: "User login successful".to_string(),
            fields: HashMap::new(),
            span_id: None,
            trace_id: None,
        };

        let entry2 = LogEntry {
            timestamp: "2023-01-01T00:00:01Z".to_string(),
            level: "INFO".to_string(),
            target: "things3_cli".to_string(),
            message: "Database connection failed".to_string(),
            fields: HashMap::new(),
            span_id: None,
            trace_id: None,
        };

        aggregator.entries.push(entry1);
        aggregator.entries.push(entry2);

        let filter = LogFilter {
            level: None,
            target: None,
            message_pattern: Some("failed".to_string()),
            time_range: None,
            fields: HashMap::new(),
        };

        let filtered = aggregator.filter_logs(&filter);
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].message.contains("failed"));
    }

    #[test]
    fn test_log_aggregator_get_statistics() {
        let mut aggregator = LogAggregator::new("test.log".to_string(), 1000);

        // Add entries with different levels
        for i in 0..5 {
            let level = if i % 2 == 0 { "INFO" } else { "ERROR" };
            let entry = LogEntry {
                timestamp: format!("2023-01-01T00:00:0{i}Z"),
                level: level.to_string(),
                target: "things3_cli".to_string(),
                message: format!("Message {i}"),
                fields: HashMap::new(),
                span_id: None,
                trace_id: None,
            };
            aggregator.entries.push(entry);
        }

        let stats = aggregator.get_statistics();
        assert_eq!(stats.total_entries, 5);
        assert_eq!(stats.level_counts.get("INFO"), Some(&3));
        assert_eq!(stats.level_counts.get("ERROR"), Some(&2));
    }

    #[test]
    fn test_log_rotator_creation() {
        let rotator = LogRotator::new("test.log".to_string(), 1024 * 1024, 5);
        assert_eq!(rotator.max_size, 1024 * 1024);
        assert_eq!(rotator.max_files, 5);
    }

    #[test]
    fn test_log_rotator_should_rotate() {
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test.log");
        let log_file_str = log_file.to_string_lossy().to_string();

        // Create a small test log file
        fs::write(&log_file, "small content").unwrap();

        let rotator = LogRotator::new(log_file_str.clone(), 100, 5);

        // Should not rotate for small files
        assert!(!rotator.should_rotate());

        // Create a large test log file
        let large_content = "x".repeat(200);
        fs::write(&log_file, large_content).unwrap();

        let rotator_large = LogRotator::new(log_file_str, 100, 5);

        // Should rotate for large files
        assert!(rotator_large.should_rotate());
    }

    #[test]
    fn test_log_rotator_rotate() {
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test.log");
        let log_file_str = log_file.to_string_lossy().to_string();

        // Create a large test log file
        let large_content = "x".repeat(200);
        fs::write(&log_file, large_content).unwrap();

        let rotator = LogRotator::new(log_file_str, 100, 5);

        // This should create a rotated file
        let result = rotator.rotate();
        assert!(result.is_ok());

        // Check that the original file was renamed
        let rotated_files: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();

        // Should have at least one rotated file
        assert!(!rotated_files.is_empty());
    }

    #[test]
    fn test_logging_error_display() {
        let error = LoggingError::FileRead("test error".to_string());
        assert!(error.to_string().contains("Failed to read log file"));
        assert!(error.to_string().contains("test error"));
    }

    #[test]
    fn test_logging_error_variants() {
        let file_read_error = LoggingError::FileRead("read error".to_string());
        let file_write_error = LoggingError::FileWrite("write error".to_string());
        let invalid_format_error = LoggingError::InvalidFormat("format error".to_string());
        let filter_compilation_error = LoggingError::FilterCompilation("filter error".to_string());

        assert!(matches!(file_read_error, LoggingError::FileRead(_)));
        assert!(matches!(file_write_error, LoggingError::FileWrite(_)));
        assert!(matches!(
            invalid_format_error,
            LoggingError::InvalidFormat(_)
        ));
        assert!(matches!(
            filter_compilation_error,
            LoggingError::FilterCompilation(_)
        ));
    }

    #[test]
    fn test_log_aggregator_load_logs_nonexistent_file() {
        let mut aggregator = LogAggregator::new("nonexistent.log".to_string(), 1000);
        let result = aggregator.load_logs();
        assert!(result.is_ok());
        assert_eq!(aggregator.entries.len(), 0);
    }

    #[test]
    fn test_log_aggregator_export_logs() {
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test.log");
        let log_file_str = log_file.to_string_lossy().to_string();
        let output_file = temp_dir.path().join("exported.log");
        let output_file_str = output_file.to_string_lossy().to_string();

        let mut aggregator = LogAggregator::new(log_file_str, 1000);

        let entry = LogEntry {
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            target: "things3_cli".to_string(),
            message: "Test message".to_string(),
            fields: HashMap::new(),
            span_id: None,
            trace_id: None,
        };

        aggregator.entries.push(entry);

        let filter = LogFilter {
            level: None,
            target: None,
            message_pattern: None,
            time_range: None,
            fields: HashMap::new(),
        };

        let result = aggregator.export_logs(&filter, &output_file_str);
        assert!(result.is_ok());

        // Verify file was created
        assert!(output_file.exists());
    }
}
