use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable/disable automatic backups
    pub enabled: bool,

    /// Backup directory path
    pub backup_dir: PathBuf,

    /// Backup interval (in hours)
    pub interval_hours: u64,

    /// Maximum number of backups to retain
    pub max_backups: usize,

    /// Backup format (json, csv, backup)
    pub format: String,

    /// Compress backups
    pub compress: bool,

    /// Verify backup after creation
    pub verify_after_backup: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backup_dir: dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("chamber")
                .join("backups"),
            interval_hours: 24, // Daily backups
            max_backups: 7,     // Keep 7 backups
            format: "backup".to_string(),
            compress: true,
            verify_after_backup: true,
        }
    }
}
