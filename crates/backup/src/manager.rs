use anyhow::{Result, anyhow};
use chamber_import_export::{ExportFormat, export_items};
use chamber_vault::{BackupConfig, Item, Vault};
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

pub trait VaultOperations {
    /// Lists all items in the vault
    fn list_items(&self) -> Result<Vec<Item>>;
}

impl VaultOperations for Vault {
    fn list_items(&self) -> Result<Vec<Item>> {
        self.list_items()
    }
}

pub struct BackupManager<V: VaultOperations> {
    pub config: BackupConfig,
    vault: V,
}

impl<V: VaultOperations> BackupManager<V> {
    pub const fn new(vault: V, config: BackupConfig) -> Self {
        Self { config, vault }
    }

    /// Attempts to create a backup if specific conditions are met.
    ///
    /// This function checks if the backup functionality is enabled in the configuration
    /// and if the current state meets the criteria for performing a backup. If both conditions
    /// are satisfied, the backup process is initiated.
    ///
    /// # Returns
    /// * `Ok(Some(PathBuf))` - Path to the backup if a backup was successfully created.
    /// * `Ok(None)` - If backup functionality is disabled or the conditions for backup are not met.
    /// * `Err` - If an error occurs during the preconditions check or while performing the backup.
    ///
    /// # Errors
    /// This function may return an error in the following cases:
    /// * An error occurs while determining if a backup should be performed (`should_backup`).
    /// * An error occurs during the execution of the backup process (`perform_backup`).
    pub fn backup_if_needed(&mut self) -> Result<Option<PathBuf>> {
        if !self.config.enabled {
            return Ok(None);
        }

        if !self.should_backup()? {
            return Ok(None);
        }

        self.perform_backup()
    }

    /// Forces the initiation of a backup and ensures its successful completion.
    ///
    /// This function triggers the backup process in a guaranteed manner and ensures that the
    /// resulting backup is successfully created. It internally calls the `perform_backup`
    /// function and expects it to return a successful result. If `perform_backup` fails or
    /// does not produce a valid backup, this function will panic with a message indicating
    /// the failure.
    ///
    /// # Returns
    ///
    /// Returns a `Result<PathBuf>` containing the path to the completed backup if successful,
    /// or an error if the backup process fails.
    ///
    /// # Errors
    ///
    /// This function propagates any errors encountered during the call to `perform_backup`.
    /// Additionally, it will panic if `perform_backup` does not yield a valid backup path.
    ///
    /// # Panics
    ///
    /// Panics with the message `"Backup failed to perform"` if the result of `perform_backup`
    /// is `None` despite the operation succeeding.
    #[allow(clippy::unwrap_in_result)]
    #[allow(clippy::expect_used)]
    pub fn force_backup(&mut self) -> Result<PathBuf> {
        self.perform_backup().map(|opt| opt.expect("Backup failed to perform"))
    }

    fn should_backup(&self) -> Result<bool> {
        // Check if a backup directory exists, create if not
        if !self.config.backup_dir.exists() {
            fs::create_dir_all(&self.config.backup_dir)?;
            return Ok(true); // First backup
        }

        // Find the most recent backup
        let most_recent = self.find_most_recent_backup()?;

        if let Some(recent_path) = most_recent {
            // Check the timestamp in filename
            if let Some(timestamp) = self.extract_timestamp_from_filename(&recent_path) {
                let now = OffsetDateTime::now_utc();
                let duration_since = now - timestamp;
                #[allow(clippy::cast_possible_wrap)]
                let interval = time::Duration::hours(self.config.interval_hours as i64);

                return Ok(duration_since >= interval);
            }
        }

        Ok(true) // No recent backup found
    }

    fn perform_backup(&mut self) -> Result<Option<PathBuf>> {
        // Ensure backup directory exists
        fs::create_dir_all(&self.config.backup_dir)?;

        // Generate backup filename with timestamp
        let timestamp = OffsetDateTime::now_utc();
        let filename = self.generate_backup_filename(&timestamp)?;
        let backup_path = self.config.backup_dir.join(&filename);

        // Export the vault data
        let items = self.vault.list_items()?;

        let export_format = match self.config.format.as_str() {
            "json" => ExportFormat::Json,
            "csv" => ExportFormat::Csv,
            "backup" => ExportFormat::ChamberBackup,
            _ => return Err(anyhow!("Invalid backup format: {}", self.config.format)),
        };

        // Perform the export
        export_items(&items, &export_format, &backup_path)?;

        // Compress if requested
        let final_path = if self.config.compress {
            Self::compress_backup(&backup_path)?
        } else {
            backup_path
        };

        // Verify backup if requested
        if self.config.verify_after_backup {
            self.verify_backup(&final_path)?;
        }

        // Clean up old backups
        self.cleanup_old_backups()?;

        Ok(Some(final_path))
    }

    fn generate_backup_filename(&self, timestamp: &OffsetDateTime) -> Result<String> {
        let date_str = timestamp.format(&time::format_description::well_known::Rfc3339)?;
        let safe_date = date_str.replace(':', "-").replace('T', "_");

        let extension = if self.config.compress {
            format!("{}.gz", self.config.format)
        } else {
            self.config.format.clone()
        };

        Ok(format!(
            "chamber_backup_{}_{}.{}",
            safe_date,
            timestamp.unix_timestamp(),
            extension
        ))
    }

    fn compress_backup(path: &Path) -> Result<PathBuf> {
        use std::fs::File;
        use std::io::BufReader;

        let compressed_path =
            path.with_extension(format!("{}.gz", path.extension().unwrap_or_default().to_string_lossy()));

        let input = File::open(path)?;
        let output = File::create(&compressed_path)?;

        let mut encoder = flate2::write::GzEncoder::new(output, flate2::Compression::default());
        let mut reader = BufReader::new(input);

        std::io::copy(&mut reader, &mut encoder)?;
        encoder.finish()?;

        // Remove original uncompressed file
        fs::remove_file(path)?;

        Ok(compressed_path)
    }

    fn verify_backup(&self, path: &Path) -> Result<()> {
        // Basic verification - ensure file exists and is not empty
        let metadata = fs::metadata(path)?;
        if metadata.len() == 0 {
            return Err(anyhow!("Backup file is empty: {}", path.display()));
        }

        // For compressed files, try to decompress a small portion
        if path.extension().and_then(|s| s.to_str()) == Some("gz") {
            Self::verify_compressed_backup(path)?;
        } else {
            // For uncompressed files, try to parse the format
            self.verify_uncompressed_backup(path)?;
        }

        Ok(())
    }

    fn verify_compressed_backup(path: &Path) -> Result<()> {
        use std::fs::File;
        use std::io::Read;

        let file = File::open(path)?;
        let mut decoder = flate2::read::GzDecoder::new(file);
        let mut buffer = [0; 1024];

        // Try to read some data to ensure it's a valid gzip
        let _ = decoder.read(&mut buffer)?;
        Ok(())
    }

    fn verify_uncompressed_backup(&self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;

        match self.config.format.as_str() {
            "json" => {
                serde_json::from_str::<serde_json::Value>(&content)?;
            }
            "backup" => {
                // Try to parse as ChamberBackup format
                serde_json::from_str::<chamber_import_export::ChamberBackup>(&content)?;
            }
            "csv" => {
                // Basic CSV validation - check header exists
                if !content.starts_with("name,kind,value") {
                    return Err(anyhow!("Invalid CSV backup format"));
                }
            }
            _ => return Err(anyhow!("Unknown backup format for verification")),
        }

        Ok(())
    }

    fn cleanup_old_backups(&self) -> Result<()> {
        let mut backups = self.find_all_backups()?;

        if backups.len() <= self.config.max_backups {
            return Ok(());
        }

        // Sort by timestamp (newest first)
        backups.sort_by(|a, b| {
            let time_a = self
                .extract_timestamp_from_filename(a)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH);
            let time_b = self
                .extract_timestamp_from_filename(b)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH);
            time_b.cmp(&time_a)
        });

        // Remove old backups
        for old_backup in backups.iter().skip(self.config.max_backups) {
            if let Err(e) = fs::remove_file(old_backup) {
                eprintln!("Warning: Failed to remove old backup {}: {}", old_backup.display(), e);
            }
        }

        Ok(())
    }

    /// Finds and returns all backup files located in the configured backup directory.
    ///
    /// This method scans the backup directory specified in the configuration
    /// and identifies all files that meet the criteria for being considered
    /// backup files (as determined by the `is_backup_file` method).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PathBuf>)` - A vector containing the paths to all identified backup files.
    /// * `Err(io::Error)` - If an error occurs while reading the backup directory or its entries.
    ///
    /// # Behavior
    ///
    /// * If the configured backup directory does not exist, the method
    ///   returns an empty vector wrapped in `Ok`.
    /// * If the directory exists, the method iterates through its contents and
    ///   adds any files matching the backup file criteria to the result vector.
    ///
    /// # Errors
    ///
    /// This function returns an `Err` if:
    /// - The `backup_dir` cannot be read (e.g., due to insufficient permissions).
    /// - An error occurs while iterating over entries within the directory.
    pub fn find_all_backups(&self) -> Result<Vec<PathBuf>> {
        let mut backups = Vec::new();

        if !self.config.backup_dir.exists() {
            return Ok(backups);
        }

        for entry in fs::read_dir(&self.config.backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && Self::is_backup_file(&path) {
                backups.push(path);
            }
        }

        Ok(backups)
    }

    /// Finds the most recent backup file from a list of backup files.
    ///
    /// This method retrieves all backup files by invoking the `find_all_backups` method. It iterates
    /// through each backup file to extract its timestamp using the `extract_timestamp_from_filename`
    /// method. The backup file with the most recent timestamp is identified and returned.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(PathBuf))` - A `PathBuf` representing the most recent backup file, if any backups exist.
    /// * `Ok(None)` - Indicates no backup files were found.
    /// * `Err` - Propagates any errors that occur while retrieving the list of backups or extracting timestamps.
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The `find_all_backups` method fails to retrieve the list of backup files.
    /// - Any other internal method call fails.
    ///
    /// # Implementation Details
    ///
    /// - The backups are identified and sorted based on their timestamp. The timestamp is derived
    ///   from the filename by using the `extract_timestamp_from_filename` method.
    /// - The initial reference timestamp is set to the UNIX epoch (`OffsetDateTime::UNIX_EPOCH`).
    pub fn find_most_recent_backup(&self) -> Result<Option<PathBuf>> {
        let backups = self.find_all_backups()?;

        let mut most_recent = None;
        let mut most_recent_time = OffsetDateTime::UNIX_EPOCH;

        for backup in backups {
            if let Some(timestamp) = self.extract_timestamp_from_filename(&backup) {
                if timestamp > most_recent_time {
                    most_recent_time = timestamp;
                    most_recent = Some(backup);
                }
            }
        }

        Ok(most_recent)
    }

    fn is_backup_file(path: &Path) -> bool {
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            filename.starts_with("chamber_backup_")
        } else {
            false
        }
    }

    pub fn extract_timestamp_from_filename(&self, path: &Path) -> Option<OffsetDateTime> {
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            // Extract Unix timestamp from filename
            // Format: chamber_backup_YYYY-MM-DD_HH-MM-SSZ_TIMESTAMP.format
            if let Some(timestamp_part) = filename.split('_').nth(3) {
                if let Some(timestamp_str) = timestamp_part.split('.').next() {
                    if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                        return OffsetDateTime::from_unix_timestamp(timestamp).ok();
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use anyhow::Result;
    use chamber_vault::{BackupConfig, Item, ItemKind};
    use std::fs;
    use tempfile::TempDir;
    use time::OffsetDateTime;

    // Mock vault implementation for testing
    struct MockVault {
        items: Vec<Item>,
        should_fail: bool,
    }

    impl MockVault {
        fn new(items: Vec<Item>) -> Self {
            Self {
                items,
                should_fail: false,
            }
        }

        fn new_failing() -> Self {
            Self {
                items: vec![],
                should_fail: true,
            }
        }
    }

    impl VaultOperations for MockVault {
        fn list_items(&self) -> Result<Vec<Item>> {
            if self.should_fail {
                return Err(anyhow::anyhow!("Mock vault error"));
            }
            Ok(self.items.clone())
        }
    }

    fn create_test_item(id: i64, name: &str) -> Item {
        Item {
            id,
            name: name.to_string(),
            kind: ItemKind::Password,
            value: "test_value".to_string(),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    fn create_test_config(temp_dir: &TempDir) -> BackupConfig {
        BackupConfig {
            enabled: true,
            backup_dir: temp_dir.path().join("backups"),
            format: "json".to_string(),
            compress: false,
            interval_hours: 24,
            max_backups: 5,
            verify_after_backup: false, // Disable for testing since we can't mock export_items
        }
    }

    #[test]
    fn test_generic_backup_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let items = vec![create_test_item(1, "test_item")];
        let vault = MockVault::new(items);

        let manager = BackupManager::new(vault, config.clone());

        assert_eq!(manager.config.enabled, config.enabled);
        assert_eq!(manager.config.format, config.format);
        assert_eq!(manager.config.max_backups, config.max_backups);
    }

    #[test]
    fn test_vault_operations_trait() {
        let items = vec![create_test_item(1, "test_item_1"), create_test_item(2, "test_item_2")];
        let vault = MockVault::new(items);

        let result = vault.list_items().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "test_item_1");
        assert_eq!(result[1].name, "test_item_2");

        // Verify items are the concrete Item type
        assert_eq!(result[0].kind, ItemKind::Password);
        assert_eq!(result[0].value, "test_value");
    }

    #[test]
    fn test_vault_operations_failure() {
        let vault = MockVault::new_failing();

        let result = vault.list_items();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Mock vault error"));
    }

    #[test]
    fn test_backup_if_needed_disabled_with_generic() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config(&temp_dir);
        config.enabled = false;
        let items = vec![create_test_item(1, "test_item")];
        let vault = MockVault::new(items);

        let mut manager = BackupManager::new(vault, config);
        let result = manager.backup_if_needed().unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_should_backup_first_time() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let items = vec![create_test_item(1, "test_item")];
        let vault = MockVault::new(items);

        let manager = BackupManager::new(vault, config);

        // Should return true for first backup (no backup directory exists)
        assert!(manager.should_backup().unwrap());
    }

    #[test]
    fn test_find_all_backups_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let vault = MockVault::new(vec![create_test_item(1, "test_item")]);
        let manager = BackupManager::new(vault, config);

        let backups = manager.find_all_backups().unwrap();
        assert!(backups.is_empty());
    }

    #[test]
    fn test_find_all_backups_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        // Create valid backup files
        let backup1 = backup_dir.join("chamber_backup_2024-01-01_00-00-00Z_1640995200.json");
        let backup2 = backup_dir.join("chamber_backup_2024-01-02_00-00-00Z_1641081600.json");

        // Create invalid file
        let invalid_file = backup_dir.join("not_a_backup.txt");

        fs::write(&backup1, "backup1 content").unwrap();
        fs::write(&backup2, "backup2 content").unwrap();
        fs::write(&invalid_file, "invalid content").unwrap();

        let config = create_test_config(&temp_dir);
        let vault = MockVault::new(vec![create_test_item(1, "test_item")]);
        let manager = BackupManager::new(vault, config);

        let backups = manager.find_all_backups().unwrap();
        assert_eq!(backups.len(), 2);
        assert!(backups.contains(&backup1));
        assert!(backups.contains(&backup2));
        assert!(!backups.iter().any(|p| p == &invalid_file));
    }

    #[test]
    fn test_concrete_item_usage() {
        // Test that we can work with the concrete Item type
        let items = vec![
            Item {
                id: 1,
                name: "password_item".to_string(),
                kind: ItemKind::Password,
                value: "secret123".to_string(),
                created_at: OffsetDateTime::now_utc(),
                updated_at: OffsetDateTime::now_utc(),
            },
            Item {
                id: 2,
                name: "api_key_item".to_string(),
                kind: ItemKind::ApiKey,
                value: "api_key_abc".to_string(),
                created_at: OffsetDateTime::now_utc(),
                updated_at: OffsetDateTime::now_utc(),
            },
        ];

        let vault = MockVault::new(items);
        let result = vault.list_items().unwrap();

        // Can access all Item fields without issues
        assert_eq!(result[0].name, "password_item");
        assert_eq!(result[0].kind, ItemKind::Password);
        assert_eq!(result[0].value, "secret123");
        assert!(result[0].id > 0);

        assert_eq!(result[1].name, "api_key_item");
        assert_eq!(result[1].kind, ItemKind::ApiKey);
        assert_eq!(result[1].value, "api_key_abc");
    }

    #[test]
    fn test_trait_object_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        let _ = create_test_config(&temp_dir);

        // Test that we can use trait objects
        let vault: Box<dyn VaultOperations> = Box::new(MockVault::new(vec![create_test_item(1, "test")]));
        let items = vault.list_items().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "test");
    }

    // Test with different vault implementations but same Item type
    struct AlternativeVault {
        data: Vec<Item>,
    }

    impl AlternativeVault {
        fn new(data: Vec<Item>) -> Self {
            Self { data }
        }
    }

    impl VaultOperations for AlternativeVault {
        fn list_items(&self) -> Result<Vec<Item>> {
            Ok(self.data.clone())
        }
    }

    #[test]
    fn test_multiple_vault_implementations() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let items = vec![create_test_item(1, "shared_item")];

        // Both vault implementations work with the same Item type
        let mock_vault = MockVault::new(items.clone());
        let alt_vault = AlternativeVault::new(items);

        let manager1 = BackupManager::new(mock_vault, create_test_config(&temp_dir1));
        let manager2 = BackupManager::new(alt_vault, create_test_config(&temp_dir2));

        // Both managers work with the same Item type
        let items1 = manager1.vault.list_items().unwrap();
        let items2 = manager2.vault.list_items().unwrap();

        assert_eq!(items1[0].name, items2[0].name);
        assert_eq!(items1[0].kind, items2[0].kind);
        assert_eq!(items1[0].value, items2[0].value);
    }
}
