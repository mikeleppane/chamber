use crate::registry::VaultInfo;
use crate::{Vault, VaultCategory, VaultRegistry};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug)]
pub struct VaultManager {
    pub registry: VaultRegistry,
    open_vaults: HashMap<String, Vault>,
}

impl VaultManager {
    /// Creates a new instance of the struct.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a newly initialized instance of `Self`
    /// if the operation is successful. Otherwise, it returns an error.
    ///
    /// # Procedure
    ///
    /// - Loads the vault registry using the `VaultRegistry::load()` method.
    /// - Initializes the `open_vaults` field as an empty `HashMap`.
    ///
    /// # Errors
    ///
    /// This function will return an error if `VaultRegistry::load()` fails.
    pub fn new() -> Result<Self> {
        let registry = VaultRegistry::load()?;
        Ok(Self {
            registry,
            open_vaults: HashMap::new(),
        })
    }

    /// List all available vaults
    #[must_use]
    pub fn list_vaults(&self) -> Vec<&VaultInfo> {
        self.registry.list_vaults()
    }

    /// Creates a new vault with the specified parameters and initializes it with the given master password.
    ///
    /// # Arguments
    ///
    /// * `name` - A `String` representing the name of the vault. This will serve as the identifying label for the vault.
    /// * `path` - An optional `PathBuf` specifying the file system location where the vault will be stored.
    ///   If `None` is provided, a default location will be determined by the system.
    /// * `category` - A `VaultCategory` enum indicating the category or use case associated with this vault (e.g., personal, business).
    /// * `description` - An optional `String` for providing additional details or metadata about the vault.
    /// * `master_password` - A reference to a `str` that will be used to secure the freshly created vault. This master password is required for future vault access.
    ///
    /// # Returns
    ///
    /// Returns a `Result<String>`:
    /// * `Ok(String)` - The unique identifier (vault ID) of the newly created vault on successful creation and initialization.
    /// * `Err` - An error is returned if vault creation, retrieval, or initialization fails.
    ///
    /// # Errors
    ///
    /// This function may return an error in the following scenarios:
    /// * If the vault could not be successfully created in the registry.
    /// * If the vault information retrieval process encounters an issue.
    /// * If the vault initialization process fails (e.g., due to encryption setup or invalid master password).
    ///
    /// # Panics
    ///
    /// # Notes
    ///
    /// * Ensure that `master_password` is secure and not easily guessable, as it safeguards the integrity of the vault's contents.
    /// * It is the caller's responsibility to manage and securely store the returned vault ID for future reference.
    ///
    /// # See Also
    ///
    /// * `self.registry.create_vault` - Handles the creation of the vault in the internal registry.
    /// * `Vault::open_or_create` - Opens an existing vault or creates a new one if it does not exist.
    /// * `Vault::initialize` - Prepares the vault for use by setting up encryption and other necessary configurations.
    #[allow(clippy::panic)]
    pub fn create_vault(
        &mut self,
        name: String,
        path: Option<PathBuf>,
        category: VaultCategory,
        description: Option<String>,
        master_password: &str,
    ) -> Result<String> {
        let vault_id = self.registry.create_vault(name, path, category, description)?;

        // Initialize the new vault
        let vault_info = self
            .registry
            .get_vault(&vault_id)
            .ok_or_else(|| anyhow!("Vault with id {} not found", vault_id))?;
        let mut vault = Vault::open_or_create(Some(&vault_info.path))?;
        vault.initialize(master_password)?;

        Ok(vault_id)
    }

    /// Opens a vault by its identifier and unlocks it using the provided master password.
    ///
    /// # Parameters
    /// - `vault_id`: A string slice representing the unique identifier of the vault to be opened.
    /// - `master_password`: A string slice representing the master password used to unlock the vault.
    ///
    /// # Returns
    /// - `Ok(())`: If the vault is successfully opened and unlocked.
    /// - `Err`: If the vault does not exist or an error occurs during any operation (e.g., unlocking or reading the vault).
    ///
    /// # Errors
    /// - Returns an error if the vault with the given `vault_id` is not found in the registry.
    /// - Returns an error if unlocking the vault with the `master_password` fails.
    /// - Returns an error if there is an issue opening or creating the vault.
    ///
    /// # Side Effects
    /// - Adds the opened vault to the `open_vaults` collection, which keeps track of currently open vaults.
    pub fn open_vault(&mut self, vault_id: &str, master_password: &str) -> Result<()> {
        let vault_info = self
            .registry
            .get_vault(vault_id)
            .ok_or_else(|| anyhow!("Vault '{}' not found", vault_id))?;

        let mut vault = Vault::open_or_create(Some(&vault_info.path))?;
        vault.unlock(master_password)?;

        self.open_vaults.insert(vault_id.to_string(), vault);
        Ok(())
    }

    /// Switches the active vault to the specified vault ID.
    ///
    /// This function checks if the given `vault_id` exists in the registry's list of vaults.
    /// If the vault exists, it sets the specified vault as the active vault. If the vault
    /// does not exist, an error is returned.
    ///
    /// # Arguments
    ///
    /// * `vault_id` - A string slice that holds the ID of the vault to be set as active.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the active vault is successfully switched.
    /// * `Err` if the specified `vault_id` is not found in the registry's list of vaults or
    ///   if there is an error setting the active vault.
    ///
    /// # Errors
    ///
    /// This function returns an error under the following conditions:
    /// - The specified `vault_id` is not found in the registry.
    /// - There is an error while calling `set_active_vault`.
    pub fn switch_active_vault(&mut self, vault_id: &str) -> Result<()> {
        if !self.registry.vaults.contains_key(vault_id) {
            return Err(anyhow!("Vault '{}' not found", vault_id));
        }

        self.registry.set_active_vault(vault_id)?;
        Ok(())
    }

    /// Retrieves a mutable reference to the currently active `Vault`.
    ///
    /// This function checks if there is an active vault ID registered in `self.registry`.
    /// If no active vault ID is set, it returns an error indicating that there is no active vault.
    /// If an active vault ID is set but the corresponding vault is not unlocked (i.e., not present
    /// in `self.open_vaults`), it returns an error indicating that the vault is not unlocked.
    /// On success, it returns a mutable reference to the active `Vault`.
    ///
    /// # Returns
    ///
    /// * `Ok(&mut Vault)` - A mutable reference to the active `Vault` if one exists and is unlocked.
    /// * `Err(anyhow::Error)` - An error if there is no active vault ID set or the vault is not unlocked.
    ///
    /// # Errors
    ///
    /// * Returns an error with the message `"No active vault"` if `self.registry.active_vault_id` is `None`.
    /// * Returns an error with the message `"Active vault '<ID>' is not unlocked"` if the active vault is not
    ///   found in `self.open_vaults`.
    ///
    /// # Panics
    ///
    /// * This function will panic if `unwrap()` is called and the vault corresponding
    ///   to the active ID is unexpectedly missing from `self.open_vaults`. However, this condition
    ///   should not occur due to the prior `contains_key` check.
    pub fn get_active_vault(&mut self) -> Result<&mut Vault> {
        let active_id = self
            .registry
            .active_vault_id
            .as_ref()
            .ok_or_else(|| anyhow!("No active vault"))?;

        if !self.open_vaults.contains_key(active_id) {
            return Err(anyhow!("Active vault '{}' is not unlocked", active_id));
        }
        let message = format!("Active vault '{active_id}' not found in open vaults.");
        Ok(self.open_vaults.get_mut(active_id).expect(&message))
    }

    /// Closes the vault associated with the given vault ID.
    ///
    /// # Parameters
    /// - `vault_id`: A string slice that represents the unique identifier of the vault to be closed.
    ///
    /// # Returns
    /// - `Ok(())` if the operation was successful.
    /// - `Err(e)` if an error occurs (specific error type depends on the implementation of `Result`).
    ///
    /// # Errors
    ///
    /// # Behavior
    /// - This function removes the specified `vault_id` from the `open_vaults` collection.
    /// - After the operation, the vault will no longer be considered open.
    pub fn close_vault(&mut self, vault_id: &str) -> Result<()> {
        self.open_vaults.remove(vault_id);
        Ok(())
    }

    /// Close all vaults
    pub fn close_all_vaults(&mut self) {
        self.open_vaults.clear();
    }

    /// Deletes a specified vault from the system.
    ///
    /// This method performs the following operations:
    /// 1. Closes the vault if it is currently open.
    /// 2. Removes the vault from the registry.
    /// 3. Optionally deletes the vault's associated files from the storage if `delete_file` is set to `true`.
    ///
    /// # Arguments
    ///
    /// * `vault_id` - A string slice that represents the unique identifier of the vault to be deleted.
    /// * `delete_file` - A boolean that, when `true`, ensures the associated vault files are also deleted from the system.
    ///
    /// # Returns
    ///
    /// * `Ok(())` on successful deletion of the vault.
    /// * `Err` if there is an issue with deleting the vault from the registry or associated files.
    ///
    /// # Errors
    ///
    /// Returns an error in the following cases:
    /// * If there is a failure while removing the vault from the registry.
    /// * If deleting the associated files fails when `delete_file` is `true`.
    pub fn delete_vault(&mut self, vault_id: &str, delete_file: bool) -> Result<()> {
        // Close the vault if it's open
        self.open_vaults.remove(vault_id);

        // Remove from the registry
        self.registry.delete_vault(vault_id, delete_file)?;
        Ok(())
    }

    /// Imports a vault into the registry.
    ///
    /// This function allows you to import a vault by specifying the path to the vault file,
    /// a name for the vault, a category, and whether you want to copy the file during the import process.
    ///
    /// # Arguments
    ///
    /// * `vault_file` - A reference to the file path of the vault to be imported.
    /// * `name` - A `String` representing the name to be assigned to the imported vault.
    /// * `category` - A `VaultCategory` specifying the category of the vault.
    /// * `copy_file` - A `bool` indicating whether the vault file should be copied during the import.
    ///   - `true`: The file will be copied to the registry.
    ///   - `false`: The original file path will be used without copying.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// * `Ok(String)`: A success message or identifier for the imported vault.
    /// * `Err(_)`: An error if the import fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The file at the given path cannot be accessed.
    /// * The file cannot be imported due to issues such as invalid format or permissions.
    /// * The registry encounters an internal error during the import process.
    pub fn import_vault(
        &mut self,
        vault_file: &std::path::Path,
        name: String,
        category: VaultCategory,
        copy_file: bool,
    ) -> Result<String> {
        self.registry.import_vault(vault_file, name, category, copy_file)
    }

    ///
    /// Updates the information of a vault identified by its `vault_id`.
    ///
    /// This method allows updating various properties of a vault stored in the registry,
    /// such as its name, description, category, and favorite status. Each property can
    /// be updated optionally by passing a `Some` value, or left unchanged by passing `None`.
    ///
    /// # Parameters
    /// - `vault_id`: A string slice that uniquely identifies the vault to be updated.
    /// - `name`: An `Option<String>` representing the new name of the vault. Pass `Some(new_name)` to update
    ///   the name or `None` to leave it unchanged.
    /// - `description`: An `Option<String>` representing the new description for the vault. Pass `Some(new_description)`
    ///   to update the description or `None` to leave it unchanged.
    /// - `category`: An `Option<VaultCategory>` specifying a new category for the vault. Pass `Some(new_category)`
    ///   to update the category or `None` to leave it unchanged.
    /// - `is_favorite`: An `Option<bool>` indicating whether the vault is a favorite. Pass `Some(true)` to mark it
    ///   as a favorite, `Some(false)` to remove it as a favorite, or `None` to leave this setting unchanged.
    ///
    /// # Returns
    /// - `Result<()>`: Returns an `Ok(())` on success, indicating that the vault's information was successfully updated.
    ///   Returns an error if `vault_id` does not exist or if there is an issue with updating the registry.
    ///
    /// # Errors
    /// This method will return an error in the following cases:
    /// - The registry fails to find the vault with the given `vault_id`.
    /// - An unexpected error occurs while trying to update the vault's information.
    pub fn update_vault_info(
        &mut self,
        vault_id: &str,
        name: Option<String>,
        description: Option<String>,
        category: Option<VaultCategory>,
        is_favorite: Option<bool>,
    ) -> Result<()> {
        self.registry
            .update_vault(vault_id, name, description, category, is_favorite)
    }

    /// Check if a vault is currently open/unlocked
    #[must_use]
    pub fn is_vault_open(&self, vault_id: &str) -> bool {
        self.open_vaults.contains_key(vault_id)
    }

    /// Get vault by ID (must be open)
    pub fn get_vault(&mut self, vault_id: &str) -> Option<&mut Vault> {
        self.open_vaults.get_mut(vault_id)
    }
}

impl Default for VaultManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize VaultManager")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::absurd_extreme_comparisons)]
    #![allow(unused_comparisons)]

    use super::*;
    use crate::registry::{VaultCategory, VaultInfo, VaultRegistry};
    use crate::{BackupConfig, Item, ItemKind};
    use anyhow::Result;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;
    use time::OffsetDateTime;

    // Test helper functions using Solution 1: Isolated environment
    fn create_isolated_vault_manager() -> (VaultManager, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let registry_path = temp_dir.path().join("isolated_registry.json");
        let vaults_dir = temp_dir.path().join("vaults");
        fs::create_dir_all(&vaults_dir).expect("Failed to create vaults directory");

        // Create an isolated registry
        let registry = VaultRegistry {
            vaults: HashMap::new(),
            active_vault_id: None,
            registry_path,
        };

        let manager = VaultManager {
            registry,
            open_vaults: HashMap::new(),
        };

        (manager, temp_dir)
    }

    fn create_isolated_vault() -> (Vault, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let vault_path = temp_dir.path().join("test_vault.db");

        let vault = Vault::open_or_create(Some(&vault_path)).expect("Failed to create isolated vault");

        (vault, temp_dir)
    }

    fn create_test_vault_info(id: &str, name: &str, category: VaultCategory, temp_dir: &TempDir) -> VaultInfo {
        let vault_path = temp_dir.path().join(format!("{name}.db"));

        VaultInfo {
            id: id.to_string(),
            name: name.to_string(),
            path: vault_path,
            created_at: OffsetDateTime::now_utc(),
            last_accessed: OffsetDateTime::now_utc(),
            description: Some(format!("Test vault: {name}")),
            category,
            is_active: false,
            is_favorite: false,
        }
    }

    fn create_test_vault_file(path: &std::path::Path) -> Result<()> {
        // Create a minimal vault file for testing
        let mut vault = Vault::open_or_create(Some(path))?;
        vault.initialize("test_master_password")?;
        Ok(())
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

    // VaultManager tests using isolated environment
    #[test]
    fn test_isolated_vault_manager_new_creates_empty_open_vaults() {
        let (manager, _temp_dir) = create_isolated_vault_manager();
        assert!(manager.open_vaults.is_empty());
        assert!(manager.registry.vaults.is_empty());
        assert!(manager.registry.active_vault_id.is_none());
    }

    #[test]
    fn test_isolated_list_vaults_delegates_to_registry() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();

        // Add test vaults to the isolated registry
        let vault1 = create_test_vault_info("vault1", "Personal Vault", VaultCategory::Personal, &temp_dir);
        let vault2 = create_test_vault_info("vault2", "Work Vault", VaultCategory::Work, &temp_dir);

        manager.registry.vaults.insert("vault1".to_string(), vault1);
        manager.registry.vaults.insert("vault2".to_string(), vault2);

        let vaults = manager.list_vaults();
        assert_eq!(vaults.len(), 2);

        let vault_names: Vec<&str> = vaults.iter().map(|v| v.name.as_str()).collect();
        assert!(vault_names.contains(&"Personal Vault"));
        assert!(vault_names.contains(&"Work Vault"));
    }

    #[test]
    fn test_isolated_is_vault_open_returns_false_for_nonexistent_vault() {
        let (manager, _temp_dir) = create_isolated_vault_manager();
        assert!(!manager.is_vault_open("nonexistent_vault"));
    }

    #[test]
    fn test_isolated_close_vault_removes_from_open_vaults() {
        let (mut manager, _temp_dir) = create_isolated_vault_manager();
        let (vault, _vault_temp_dir) = create_isolated_vault();

        // Add vault to open_vaults
        manager.open_vaults.insert("test_vault".to_string(), vault);
        assert!(manager.is_vault_open("test_vault"));

        // Close the vault
        let result = manager.close_vault("test_vault");
        assert!(result.is_ok());
        assert!(!manager.is_vault_open("test_vault"));
    }

    #[test]
    fn test_isolated_close_vault_succeeds_even_for_nonexistent_vault() {
        let (mut manager, _temp_dir) = create_isolated_vault_manager();

        // Should succeed even if vault doesn't exist
        let result = manager.close_vault("nonexistent_vault");
        assert!(result.is_ok());
    }

    #[test]
    fn test_isolated_close_all_vaults_clears_open_vaults() {
        let (mut manager, _temp_dir) = create_isolated_vault_manager();
        let (vault1, _temp1) = create_isolated_vault();
        let (vault2, _temp2) = create_isolated_vault();

        // Add vaults to open_vaults
        manager.open_vaults.insert("vault1".to_string(), vault1);
        manager.open_vaults.insert("vault2".to_string(), vault2);

        assert_eq!(manager.open_vaults.len(), 2);

        manager.close_all_vaults();
        assert!(manager.open_vaults.is_empty());
    }

    #[test]
    fn test_isolated_get_vault_returns_none_for_closed_vault() {
        let (mut manager, _temp_dir) = create_isolated_vault_manager();

        let result = manager.get_vault("nonexistent_vault");
        assert!(result.is_none());
    }

    #[test]
    fn test_isolated_get_vault_returns_some_for_open_vault() {
        let (mut manager, _temp_dir) = create_isolated_vault_manager();
        let (vault, _vault_temp_dir) = create_isolated_vault();

        // Add vault to open_vaults
        manager.open_vaults.insert("test_vault".to_string(), vault);

        let result = manager.get_vault("test_vault");
        assert!(result.is_some());
    }

    #[test]
    fn test_isolated_get_active_vault_fails_when_no_active_vault() {
        let (mut manager, _temp_dir) = create_isolated_vault_manager();

        let result = manager.get_active_vault();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No active vault"));
    }

    #[test]
    fn test_isolated_get_active_vault_fails_when_active_vault_not_open() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let vault_info = create_test_vault_info("test_vault", "Test Vault", VaultCategory::Personal, &temp_dir);

        // Add vault to registry and set as active, but don't open it
        manager.registry.vaults.insert("test_vault".to_string(), vault_info);
        manager.registry.active_vault_id = Some("test_vault".to_string());

        let result = manager.get_active_vault();
        assert!(result.is_err());
    }

    #[test]
    fn test_isolated_switch_active_vault_fails_for_nonexistent_vault() {
        let (mut manager, _temp_dir) = create_isolated_vault_manager();

        let result = manager.switch_active_vault("nonexistent_vault");
        assert!(result.is_err());
    }

    #[test]
    fn test_isolated_vault_lifecycle_create_open_close() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let master_password = "test_master_password";

        // Create vault
        let vault_id = manager
            .create_vault(
                "Test Vault".to_string(),
                Some(temp_dir.path().join("test_vault.db")),
                VaultCategory::Personal,
                Some("Test description".to_string()),
                master_password,
            )
            .expect("Failed to create vault");

        // Verify vault was created
        assert!(manager.registry.vaults.contains_key(&vault_id));

        // Open vault
        let result = manager.open_vault(&vault_id, master_password);
        assert!(result.is_ok());
        assert!(manager.is_vault_open(&vault_id));

        // Close vault
        let result = manager.close_vault(&vault_id);
        assert!(result.is_ok());
        assert!(!manager.is_vault_open(&vault_id));
    }

    #[test]
    fn test_isolated_vault_creation_with_custom_path() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let custom_path = temp_dir.path().join("custom").join("path").join("vault.db");
        let master_password = "test_password";

        let vault_id = manager
            .create_vault(
                "Custom Path Vault".to_string(),
                Some(custom_path.clone()),
                VaultCategory::Work,
                None,
                master_password,
            )
            .expect("Failed to create vault with custom path");

        let vault_info = manager.registry.vaults.get(&vault_id).unwrap();
        assert_eq!(vault_info.path, custom_path);
        assert_eq!(vault_info.category, VaultCategory::Work);
    }

    #[test]
    fn test_isolated_open_vault_with_wrong_password() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let correct_password = "correct_password";
        let wrong_password = "wrong_password";

        // Create vault with correct password
        let vault_id = manager
            .create_vault(
                "Test Vault".to_string(),
                Some(temp_dir.path().join("test_vault.db")),
                VaultCategory::Personal,
                None,
                correct_password,
            )
            .expect("Failed to create vault");

        // Try to open with wrong password
        let result = manager.open_vault(&vault_id, wrong_password);
        assert!(result.is_err());
        assert!(!manager.is_vault_open(&vault_id));
    }

    #[test]
    fn test_isolated_open_nonexistent_vault() {
        let (mut manager, _temp_dir) = create_isolated_vault_manager();

        let result = manager.open_vault("nonexistent_vault", "password");
        assert!(result.is_err());
    }

    #[test]
    fn test_isolated_update_vault_info() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let vault_info = create_test_vault_info("test_vault", "Original Name", VaultCategory::Personal, &temp_dir);
        manager.registry.vaults.insert("test_vault".to_string(), vault_info);

        let result = manager.update_vault_info(
            "test_vault",
            Some("Updated Name".to_string()),
            Some("Updated description".to_string()),
            Some(VaultCategory::Work),
            Some(true),
        );

        assert!(result.is_ok());
        let updated_vault = manager.registry.vaults.get("test_vault").unwrap();
        assert_eq!(updated_vault.name, "Updated Name");
        assert_eq!(updated_vault.description, Some("Updated description".to_string()));
        assert_eq!(updated_vault.category, VaultCategory::Work);
        assert!(updated_vault.is_favorite);
    }

    #[test]
    fn test_isolated_update_nonexistent_vault() {
        let (mut manager, _temp_dir) = create_isolated_vault_manager();

        let result = manager.update_vault_info("nonexistent_vault", Some("New Name".to_string()), None, None, None);

        assert!(result.is_err());
    }

    #[test]
    fn test_isolated_delete_vault_without_file() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let vault_info = create_test_vault_info("test_vault", "Test Vault", VaultCategory::Personal, &temp_dir);
        manager.registry.vaults.insert("test_vault".to_string(), vault_info);

        // Delete vault but keep file
        let result = manager.delete_vault("test_vault", false);
        assert!(result.is_ok());
        assert!(!manager.registry.vaults.contains_key("test_vault"));
    }

    #[test]
    fn test_isolated_delete_vault_with_file() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let vault_path = temp_dir.path().join("test_vault.db");

        // Create an actual vault file
        create_test_vault_file(&vault_path).expect("Failed to create test vault file");

        // Create vault info with the SAME path as the file we created
        let mut vault_info = create_test_vault_info("test_vault", "Test Vault", VaultCategory::Personal, &temp_dir);
        vault_info.path = vault_path.clone(); // Override the path to match our test file

        manager.registry.vaults.insert("test_vault".to_string(), vault_info);

        assert!(vault_path.exists());

        // Delete vault and file
        let result = manager.delete_vault("test_vault", true);
        assert!(result.is_ok());
        assert!(!manager.registry.vaults.contains_key("test_vault"));
        assert!(!vault_path.exists());
    }

    #[test]
    fn test_isolated_delete_nonexistent_vault() {
        let (mut manager, _temp_dir) = create_isolated_vault_manager();

        let result = manager.delete_vault("nonexistent_vault", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_isolated_import_vault() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let source_vault_path = temp_dir.path().join("source_vault.db");

        // Create a source vault file
        create_test_vault_file(&source_vault_path).expect("Failed to create source vault file");

        // Store the initial vault count to detect new entries
        let initial_vault_count = manager.registry.vaults.len();

        let vault_id = manager
            .import_vault(
                &source_vault_path,
                "Imported Vault".to_string(),
                VaultCategory::Archive,
                true, // copy file
            )
            .expect("Failed to import vault");

        // Verify vault was imported
        assert!(manager.registry.vaults.contains_key(&vault_id));
        assert_eq!(manager.registry.vaults.len(), initial_vault_count + 1);

        let vault_info = manager.registry.vaults.get(&vault_id).unwrap();
        assert_eq!(vault_info.name, "Imported Vault");
        assert_eq!(vault_info.category, VaultCategory::Archive);

        // Store the imported vault file path for cleanup
        let imported_vault_path = vault_info.path.clone();

        // CLEANUP: Delete the vault entry and any files that may have been created outside temp_dir
        let cleanup_result = manager.delete_vault(&vault_id, true); // delete_file = true

        // Verify cleanup was successful
        if cleanup_result.is_ok() {
            assert!(!manager.registry.vaults.contains_key(&vault_id));
            // If the file was created outside temp_dir, it should now be deleted
            if !imported_vault_path.starts_with(temp_dir.path()) {
                assert!(
                    !imported_vault_path.exists(),
                    "Production file should be cleaned up: {imported_vault_path:?}"
                );
            }
        } else {
            // If delete_vault failed, try manual cleanup
            eprintln!("Warning: delete_vault failed, attempting manual cleanup: {cleanup_result:?}");

            // Remove from the registry manually
            manager.registry.vaults.remove(&vault_id);

            // Try to delete the file manually if it's outside our temp directory
            if !imported_vault_path.starts_with(temp_dir.path()) && imported_vault_path.exists() {
                if let Err(e) = std::fs::remove_file(&imported_vault_path) {
                    eprintln!("Warning: Failed to clean up test file {imported_vault_path:?}: {e}");
                }
            }
        }

        // Final verification that we cleaned up properly
        assert!(
            !manager.registry.vaults.contains_key(&vault_id),
            "Vault should be cleaned up from registry"
        );
    }

    #[test]
    fn test_isolated_import_vault_without_copy() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let source_vault_path = temp_dir.path().join("source_vault.db");

        // Create source vault file
        create_test_vault_file(&source_vault_path).expect("Failed to create source vault file");

        let vault_id = manager
            .import_vault(
                &source_vault_path,
                "Linked Vault".to_string(),
                VaultCategory::Project,
                false, // don't copy file, just reference
            )
            .expect("Failed to import vault");

        // Verify vault was imported
        assert!(manager.registry.vaults.contains_key(&vault_id));
        let vault_info = manager.registry.vaults.get(&vault_id).unwrap();
        assert_eq!(vault_info.path, source_vault_path);
    }

    #[test]
    fn test_isolated_import_nonexistent_vault() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let nonexistent_path = temp_dir.path().join("nonexistent.db");

        let result = manager.import_vault(
            &nonexistent_path,
            "Nonexistent Vault".to_string(),
            VaultCategory::Personal,
            true,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_isolated_multiple_vaults_management() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let master_password = "test_password";

        // Create multiple vaults
        let vault1_id = manager
            .create_vault(
                "Personal Vault".to_string(),
                Some(temp_dir.path().join("personal.db")),
                VaultCategory::Personal,
                None,
                master_password,
            )
            .expect("Failed to create personal vault");

        let vault2_id = manager
            .create_vault(
                "Work Vault".to_string(),
                Some(temp_dir.path().join("work.db")),
                VaultCategory::Work,
                None,
                master_password,
            )
            .expect("Failed to create work vault");

        // Verify both vaults exist
        assert_eq!(manager.list_vaults().len(), 2);

        // Open both vaults
        assert!(manager.open_vault(&vault1_id, master_password).is_ok());
        assert!(manager.open_vault(&vault2_id, master_password).is_ok());

        // Verify both are open
        assert!(manager.is_vault_open(&vault1_id));
        assert!(manager.is_vault_open(&vault2_id));

        // Switch active vault
        assert!(manager.switch_active_vault(&vault1_id).is_ok());
        assert_eq!(manager.registry.active_vault_id, Some(vault1_id.clone()));

        // Close all vaults
        manager.close_all_vaults();
        assert!(!manager.is_vault_open(&vault1_id));
        assert!(!manager.is_vault_open(&vault2_id));
    }

    #[test]
    fn test_isolated_active_vault_workflow() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let master_password = "test_password";

        // Create and open vault
        let vault_id = manager
            .create_vault(
                "Active Vault".to_string(),
                Some(temp_dir.path().join("active.db")),
                VaultCategory::Personal,
                None,
                master_password,
            )
            .expect("Failed to create vault");

        assert!(manager.open_vault(&vault_id, master_password).is_ok());
        assert!(manager.switch_active_vault(&vault_id).is_ok());

        // Should be able to get active vault
        let active_vault_result = manager.get_active_vault();
        assert!(active_vault_result.is_ok());
    }

    #[test]
    fn test_isolated_vault_categories() {
        let (mut manager, temp_dir) = create_isolated_vault_manager();
        let master_password = "test_password";

        // Test different categories
        let categories = [
            VaultCategory::Personal,
            VaultCategory::Work,
            VaultCategory::Team,
            VaultCategory::Project,
            VaultCategory::Testing,
            VaultCategory::Archive,
            VaultCategory::Custom("Custom Category".to_string()),
        ];

        for (i, category) in categories.iter().enumerate() {
            let vault_id = manager
                .create_vault(
                    format!("Vault {i}"),
                    Some(temp_dir.path().join(format!("vault_{i}.db"))),
                    category.clone(),
                    None,
                    master_password,
                )
                .expect("Failed to create vault");

            let vault_info = manager.registry.vaults.get(&vault_id).unwrap();
            assert_eq!(vault_info.category, *category);
        }

        assert_eq!(manager.list_vaults().len(), categories.len());
    }

    #[test]
    fn test_isolated_error_handling_consistency() {
        let (mut manager, _temp_dir) = create_isolated_vault_manager();

        // All operations on nonexistent vaults should return errors
        assert!(manager.open_vault("nonexistent", "password").is_err());
        assert!(manager.switch_active_vault("nonexistent").is_err());
        assert!(
            manager
                .update_vault_info("nonexistent", None, None, None, None)
                .is_err()
        );
        assert!(manager.delete_vault("nonexistent", false).is_err());

        // Operations that should succeed even with nonexistent vaults
        assert!(manager.close_vault("nonexistent").is_ok());
        assert!(!manager.is_vault_open("nonexistent"));
        assert!(manager.get_vault("nonexistent").is_none());
    }

    // BackupManager tests can also be isolated
    fn create_test_config(temp_dir: &TempDir) -> BackupConfig {
        BackupConfig {
            enabled: true,
            backup_dir: temp_dir.path().join("backups"),
            interval_hours: 24,
            max_backups: 5,
            format: String::from("Json"),
            compress: false,
            verify_after_backup: false,
        }
    }

    #[test]
    fn test_isolated_generic_backup_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let _items = [create_test_item(1, "test_item")];

        // Create a mock vault for testing (you'd need to implement this)
        // let vault = MockVault::new(items);
        // let manager = BackupManager::new(vault, config.clone());
        //
        // assert_eq!(manager.config.enabled, config.enabled);
        // assert_eq!(manager.config.format, config.format);
        // assert_eq!(manager.config.max_backups, config.max_backups);

        // For now, just test that config creation works
        assert!(config.enabled);
        assert_eq!(config.max_backups, 5);
        assert!(temp_dir.path().join("backups") == config.backup_dir);
    }
}
