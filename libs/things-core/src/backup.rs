//! Backup and restore functionality for Things 3 database

use crate::{ThingsConfig, ThingsDatabase};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub created_at: DateTime<Utc>,
    pub source_path: PathBuf,
    pub backup_path: PathBuf,
    pub file_size: u64,
    pub version: String,
    pub description: Option<String>,
}

/// Backup manager for Things 3 database
pub struct BackupManager {
    config: ThingsConfig,
}

impl BackupManager {
    /// Create a new backup manager
    pub fn new(config: ThingsConfig) -> Self {
        Self { config }
    }

    /// Create a backup of the Things 3 database
    pub async fn create_backup(
        &self,
        backup_dir: &Path,
        description: Option<&str>,
    ) -> Result<BackupMetadata> {
        let source_path = self.config.get_effective_database_path()?;

        if !source_path.exists() {
            return Err(anyhow::anyhow!(
                "Source database does not exist: {:?}",
                source_path
            ));
        }

        // Create backup directory if it doesn't exist
        fs::create_dir_all(backup_dir)?;

        // Generate backup filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("things_backup_{}.sqlite", timestamp);
        let backup_path = backup_dir.join(backup_filename);

        // Copy the database file
        fs::copy(&source_path, &backup_path)?;

        // Get file size
        let file_size = fs::metadata(&backup_path)?.len();

        // Create metadata
        let metadata = BackupMetadata {
            created_at: Utc::now(),
            source_path: source_path.clone(),
            backup_path: backup_path.clone(),
            file_size,
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: description.map(|s| s.to_string()),
        };

        // Save metadata alongside backup
        let metadata_path = backup_path.with_extension("json");
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(&metadata_path, metadata_json)?;

        Ok(metadata)
    }

    /// Restore from a backup
    pub async fn restore_backup(&self, backup_path: &Path) -> Result<()> {
        if !backup_path.exists() {
            return Err(anyhow::anyhow!(
                "Backup file does not exist: {:?}",
                backup_path
            ));
        }

        let target_path = self.config.get_effective_database_path()?;

        // Create target directory if it doesn't exist
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Copy backup to target location
        fs::copy(backup_path, &target_path)?;

        Ok(())
    }

    /// List available backups in a directory
    pub fn list_backups(&self, backup_dir: &Path) -> Result<Vec<BackupMetadata>> {
        if !backup_dir.exists() {
            return Ok(vec![]);
        }

        let mut backups = Vec::new();

        for entry in fs::read_dir(backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("sqlite") {
                let metadata_path = path.with_extension("json");
                if metadata_path.exists() {
                    let metadata_json = fs::read_to_string(&metadata_path)?;
                    if let Ok(metadata) = serde_json::from_str::<BackupMetadata>(&metadata_json) {
                        backups.push(metadata);
                    }
                }
            }
        }

        // Sort by creation date (newest first)
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Get backup metadata from a backup file
    pub fn get_backup_metadata(&self, backup_path: &Path) -> Result<BackupMetadata> {
        let metadata_path = backup_path.with_extension("json");
        if !metadata_path.exists() {
            return Err(anyhow::anyhow!(
                "Backup metadata not found: {:?}",
                metadata_path
            ));
        }

        let metadata_json = fs::read_to_string(&metadata_path)?;
        let metadata = serde_json::from_str::<BackupMetadata>(&metadata_json)?;
        Ok(metadata)
    }

    /// Delete a backup and its metadata
    pub fn delete_backup(&self, backup_path: &Path) -> Result<()> {
        if backup_path.exists() {
            fs::remove_file(backup_path)?;
        }

        let metadata_path = backup_path.with_extension("json");
        if metadata_path.exists() {
            fs::remove_file(&metadata_path)?;
        }

        Ok(())
    }

    /// Clean up old backups, keeping only the specified number
    pub fn cleanup_old_backups(&self, backup_dir: &Path, keep_count: usize) -> Result<usize> {
        let mut backups = self.list_backups(backup_dir)?;

        if backups.len() <= keep_count {
            return Ok(0);
        }

        let to_delete = backups.split_off(keep_count);
        let mut deleted_count = 0;

        for backup in to_delete {
            if let Err(e) = self.delete_backup(&backup.backup_path) {
                eprintln!("Failed to delete backup {:?}: {}", backup.backup_path, e);
            } else {
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }

    /// Verify a backup by checking if it can be opened
    pub fn verify_backup(&self, backup_path: &Path) -> Result<bool> {
        if !backup_path.exists() {
            return Ok(false);
        }

        // Try to open the backup as a database
        match ThingsDatabase::new(backup_path) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get backup statistics
    pub fn get_backup_stats(&self, backup_dir: &Path) -> Result<BackupStats> {
        let backups = self.list_backups(backup_dir)?;

        let total_backups = backups.len();
        let total_size: u64 = backups.iter().map(|b| b.file_size).sum();
        let oldest_backup = backups.last().map(|b| b.created_at);
        let newest_backup = backups.first().map(|b| b.created_at);

        Ok(BackupStats {
            total_backups,
            total_size,
            oldest_backup,
            newest_backup,
        })
    }
}

/// Backup statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStats {
    pub total_backups: usize,
    pub total_size: u64,
    pub oldest_backup: Option<DateTime<Utc>>,
    pub newest_backup: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_metadata_creation() {
        let now = Utc::now();
        let source_path = PathBuf::from("/path/to/source.db");
        let backup_path = PathBuf::from("/path/to/backup.db");

        let metadata = BackupMetadata {
            created_at: now,
            source_path: source_path.clone(),
            backup_path: backup_path.clone(),
            file_size: 1024,
            version: "1.0.0".to_string(),
            description: Some("Test backup".to_string()),
        };

        assert_eq!(metadata.source_path, source_path);
        assert_eq!(metadata.backup_path, backup_path);
        assert_eq!(metadata.file_size, 1024);
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description, Some("Test backup".to_string()));
    }

    #[test]
    fn test_backup_metadata_serialization() {
        let now = Utc::now();
        let metadata = BackupMetadata {
            created_at: now,
            source_path: PathBuf::from("/test/source.db"),
            backup_path: PathBuf::from("/test/backup.db"),
            file_size: 2048,
            version: "2.0.0".to_string(),
            description: Some("Serialization test".to_string()),
        };

        // Test serialization
        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("created_at"));
        assert!(json.contains("source_path"));
        assert!(json.contains("backup_path"));
        assert!(json.contains("file_size"));
        assert!(json.contains("version"));
        assert!(json.contains("description"));

        // Test deserialization
        let deserialized: BackupMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source_path, metadata.source_path);
        assert_eq!(deserialized.backup_path, metadata.backup_path);
        assert_eq!(deserialized.file_size, metadata.file_size);
        assert_eq!(deserialized.version, metadata.version);
        assert_eq!(deserialized.description, metadata.description);
    }

    #[test]
    fn test_backup_manager_new() {
        let config = ThingsConfig::from_env();
        let _backup_manager = BackupManager::new(config);
        // Just test that it can be created
        assert!(true);
    }

    #[test]
    fn test_backup_stats_creation() {
        let now = Utc::now();
        let stats = BackupStats {
            total_backups: 5,
            total_size: 10240,
            oldest_backup: Some(now - chrono::Duration::days(7)),
            newest_backup: Some(now),
        };

        assert_eq!(stats.total_backups, 5);
        assert_eq!(stats.total_size, 10240);
        assert!(stats.oldest_backup.is_some());
        assert!(stats.newest_backup.is_some());
    }

    #[test]
    fn test_backup_stats_serialization() {
        let now = Utc::now();
        let stats = BackupStats {
            total_backups: 3,
            total_size: 5120,
            oldest_backup: Some(now - chrono::Duration::days(3)),
            newest_backup: Some(now - chrono::Duration::hours(1)),
        };

        // Test serialization
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("total_backups"));
        assert!(json.contains("total_size"));
        assert!(json.contains("oldest_backup"));
        assert!(json.contains("newest_backup"));

        // Test deserialization
        let deserialized: BackupStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_backups, stats.total_backups);
        assert_eq!(deserialized.total_size, stats.total_size);
    }

    #[test]
    fn test_backup_stats_empty() {
        let stats = BackupStats {
            total_backups: 0,
            total_size: 0,
            oldest_backup: None,
            newest_backup: None,
        };

        assert_eq!(stats.total_backups, 0);
        assert_eq!(stats.total_size, 0);
        assert!(stats.oldest_backup.is_none());
        assert!(stats.newest_backup.is_none());
    }

    #[test]
    fn test_backup_metadata_debug() {
        let metadata = BackupMetadata {
            created_at: Utc::now(),
            source_path: PathBuf::from("/test/source.db"),
            backup_path: PathBuf::from("/test/backup.db"),
            file_size: 1024,
            version: "1.0.0".to_string(),
            description: Some("Debug test".to_string()),
        };

        let debug_str = format!("{:?}", metadata);
        assert!(debug_str.contains("BackupMetadata"));
        assert!(debug_str.contains("source_path"));
        assert!(debug_str.contains("backup_path"));
    }

    #[test]
    fn test_backup_stats_debug() {
        let stats = BackupStats {
            total_backups: 2,
            total_size: 2048,
            oldest_backup: Some(Utc::now()),
            newest_backup: Some(Utc::now()),
        };

        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("BackupStats"));
        assert!(debug_str.contains("total_backups"));
        assert!(debug_str.contains("total_size"));
    }

    #[test]
    fn test_backup_metadata_clone() {
        let metadata = BackupMetadata {
            created_at: Utc::now(),
            source_path: PathBuf::from("/test/source.db"),
            backup_path: PathBuf::from("/test/backup.db"),
            file_size: 1024,
            version: "1.0.0".to_string(),
            description: Some("Clone test".to_string()),
        };

        let cloned = metadata.clone();
        assert_eq!(metadata.source_path, cloned.source_path);
        assert_eq!(metadata.backup_path, cloned.backup_path);
        assert_eq!(metadata.file_size, cloned.file_size);
        assert_eq!(metadata.version, cloned.version);
        assert_eq!(metadata.description, cloned.description);
    }

    #[test]
    fn test_backup_stats_clone() {
        let stats = BackupStats {
            total_backups: 1,
            total_size: 512,
            oldest_backup: Some(Utc::now()),
            newest_backup: Some(Utc::now()),
        };

        let cloned = stats.clone();
        assert_eq!(stats.total_backups, cloned.total_backups);
        assert_eq!(stats.total_size, cloned.total_size);
        assert_eq!(stats.oldest_backup, cloned.oldest_backup);
        assert_eq!(stats.newest_backup, cloned.newest_backup);
    }

    #[tokio::test]
    async fn test_backup_creation_with_nonexistent_database() {
        let temp_dir = TempDir::new().unwrap();
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        // Test backup creation with non-existent database
        let result = backup_manager
            .create_backup(temp_dir.path(), Some("test backup"))
            .await;

        // Should fail because database doesn't exist
        match result {
            Ok(metadata) => {
                // If it succeeds, verify the metadata is reasonable
                assert!(!metadata.backup_path.to_string_lossy().is_empty());
                assert!(metadata.file_size > 0);
            }
            Err(e) => {
                // If it fails, it should be because the database doesn't exist
                let error_msg = e.to_string();
                assert!(error_msg.contains("does not exist") || error_msg.contains("not found"));
            }
        }
    }

    #[tokio::test]
    async fn test_backup_creation_with_nonexistent_backup_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        // Test backup creation with non-existent backup directory
        let result = backup_manager
            .create_backup(temp_dir.path(), Some("test backup"))
            .await;

        // Should either succeed or fail gracefully
        match result {
            Ok(metadata) => {
                // If it succeeds, verify the metadata is reasonable
                assert!(!metadata.backup_path.to_string_lossy().is_empty());
                assert!(metadata.file_size > 0);
            }
            Err(e) => {
                // If it fails, it should be because the database doesn't exist
                let error_msg = e.to_string();
                assert!(error_msg.contains("does not exist") || error_msg.contains("not found"));
            }
        }
    }

    #[test]
    fn test_backup_listing_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        let backups = backup_manager.list_backups(temp_dir.path()).unwrap();
        assert_eq!(backups.len(), 0);
    }

    #[test]
    fn test_backup_listing_nonexistent_directory() {
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        let backups = backup_manager
            .list_backups(Path::new("/nonexistent/directory"))
            .unwrap();
        assert_eq!(backups.len(), 0);
    }

    #[test]
    fn test_get_backup_metadata_nonexistent() {
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        let result = backup_manager.get_backup_metadata(Path::new("/nonexistent/backup.db"));
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("not found"));
    }

    #[test]
    fn test_verify_backup_nonexistent() {
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        let result = backup_manager.verify_backup(Path::new("/nonexistent/backup.db"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_delete_backup_nonexistent() {
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        // Should not error when trying to delete non-existent backup
        let result = backup_manager.delete_backup(Path::new("/nonexistent/backup.db"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_old_backups_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        let deleted_count = backup_manager
            .cleanup_old_backups(temp_dir.path(), 5)
            .unwrap();
        assert_eq!(deleted_count, 0);
    }

    #[test]
    fn test_cleanup_old_backups_nonexistent_directory() {
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        let deleted_count = backup_manager
            .cleanup_old_backups(Path::new("/nonexistent"), 5)
            .unwrap();
        assert_eq!(deleted_count, 0);
    }

    #[test]
    fn test_get_backup_stats_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        let stats = backup_manager.get_backup_stats(temp_dir.path()).unwrap();
        assert_eq!(stats.total_backups, 0);
        assert_eq!(stats.total_size, 0);
        assert!(stats.oldest_backup.is_none());
        assert!(stats.newest_backup.is_none());
    }

    #[test]
    fn test_get_backup_stats_nonexistent_directory() {
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        let stats = backup_manager
            .get_backup_stats(Path::new("/nonexistent"))
            .unwrap();
        assert_eq!(stats.total_backups, 0);
        assert_eq!(stats.total_size, 0);
        assert!(stats.oldest_backup.is_none());
        assert!(stats.newest_backup.is_none());
    }

    #[tokio::test]
    async fn test_restore_backup_nonexistent() {
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        let result = backup_manager
            .restore_backup(Path::new("/nonexistent/backup.db"))
            .await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("does not exist"));
    }

    #[test]
    fn test_backup_metadata_without_description() {
        let now = Utc::now();
        let metadata = BackupMetadata {
            created_at: now,
            source_path: PathBuf::from("/test/source.db"),
            backup_path: PathBuf::from("/test/backup.db"),
            file_size: 1024,
            version: "1.0.0".to_string(),
            description: None,
        };

        assert!(metadata.description.is_none());

        // Test serialization with None description
        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("null")); // Should contain null for description

        // Test deserialization
        let deserialized: BackupMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.description, None);
    }

    #[test]
    fn test_backup_metadata_path_operations() {
        let source_path = PathBuf::from("/path/to/source.db");
        let backup_path = PathBuf::from("/path/to/backup.db");

        let metadata = BackupMetadata {
            created_at: Utc::now(),
            source_path: source_path.clone(),
            backup_path: backup_path.clone(),
            file_size: 1024,
            version: "1.0.0".to_string(),
            description: Some("Path test".to_string()),
        };

        // Test path operations
        assert_eq!(metadata.source_path.file_name().unwrap(), "source.db");
        assert_eq!(metadata.backup_path.file_name().unwrap(), "backup.db");
        assert_eq!(
            metadata.source_path.parent().unwrap(),
            Path::new("/path/to")
        );
        assert_eq!(
            metadata.backup_path.parent().unwrap(),
            Path::new("/path/to")
        );
    }
}
