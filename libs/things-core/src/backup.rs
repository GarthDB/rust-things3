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

    #[tokio::test]
    async fn test_backup_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        // Test backup creation - this will either succeed (if database exists) or fail gracefully
        let result = backup_manager
            .create_backup(temp_dir.path(), Some("test backup"))
            .await;

        // The test should either succeed or fail with a specific error about missing database
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
    fn test_backup_listing() {
        let temp_dir = TempDir::new().unwrap();
        let config = ThingsConfig::from_env();
        let backup_manager = BackupManager::new(config);

        let backups = backup_manager.list_backups(temp_dir.path()).unwrap();
        assert_eq!(backups.len(), 0);
    }
}
