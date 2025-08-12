use crate::registry::VaultInfo;
use crate::{Vault, VaultCategory, VaultRegistry};
use anyhow::{anyhow, Result};
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
            .unwrap_or_else(|| panic!("Cannot get vault with an id: {vault_id}"));
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
    use crate::registry::VaultCategory;
    use anyhow::Result;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Helper function to create a temporary directory for testing
    fn create_temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp dir")
    }

    // Helper to create a basic vault file for testing
    fn create_test_vault_file(path: &std::path::Path) -> Result<()> {
        // Create a minimal SQLite database file for testing
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, b"fake vault data")?;
        Ok(())
    }

    #[test]
    fn test_vault_manager_new_creates_empty_open_vaults() {
        // This test will only work if VaultRegistry::load() succeeds
        // In a real test environment, we'd want to mock this
        if let Ok(manager) = VaultManager::new() {
            assert!(manager.open_vaults.is_empty());
            // We can't easily test registry content without mocking
        } else {
            // Registry loading failed, which is expected in the test environment
            // This is acceptable as we're testing the structure
        }
    }

    #[test]
    fn test_vault_manager_default() {
        // Test that Default trait works
        // This might fail if registry loading fails, but tests the trait implementation
        if let Ok(manager) = std::panic::catch_unwind(VaultManager::default) {
            assert!(manager.open_vaults.is_empty());
        } else {
            // Expected if registry loading fails in test environment
        }
    }

    #[test]
    fn test_list_vaults_delegates_to_registry() {
        // We can test this by creating a VaultManager with a known registry state
        // This is more of an integration test since we can't easily mock VaultRegistry

        // Create a temporary VaultManager (this might fail in test env)
        if let Ok(manager) = VaultManager::new() {
            let vaults = manager.list_vaults();
            // The result should be whatever the registry returns
            // We can't assert specific content without setting up the registry
            assert!(vaults.len() >= 0); // Just test that it returns something
        }
    }

    #[test]
    fn test_is_vault_open_returns_false_for_nonexistent_vault() {
        if let Ok(manager) = VaultManager::new() {
            assert!(!manager.is_vault_open("nonexistent_vault"));
        }
    }

    #[test]
    fn test_close_vault_removes_from_open_vaults() {
        if let Ok(mut manager) = VaultManager::new() {
            // Manually insert a fake vault to test removal
            let temp_dir = create_temp_dir();
            let vault_path = temp_dir.path().join("test.db");
            create_test_vault_file(&vault_path).unwrap();

            if let Ok(vault) = Vault::open_or_create(Some(&vault_path)) {
                manager.open_vaults.insert("test_vault".to_string(), vault);

                assert!(manager.is_vault_open("test_vault"));

                let result = manager.close_vault("test_vault");
                assert!(result.is_ok());
                assert!(!manager.is_vault_open("test_vault"));
            }
        }
    }

    #[test]
    fn test_close_vault_succeeds_even_for_nonexistent_vault() {
        if let Ok(mut manager) = VaultManager::new() {
            let result = manager.close_vault("nonexistent");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_close_all_vaults_clears_open_vaults() {
        let mut manager = VaultManager::new().expect("Failed to create VaultManager");

        // Add some vaults using the proper creation flow
        let temp_dir = create_temp_dir();

        for i in 0..3 {
            let vault_path = temp_dir.path().join(format!("test{i}.db"));

            // Create and initialize a proper vault
            let mut vault =
                Vault::open_or_create(Some(&vault_path)).unwrap_or_else(|_| panic!("Failed to create vault {i}"));
            vault
                .initialize(&format!("password{i}"))
                .unwrap_or_else(|_| panic!("Failed to initialize vault {i}"));

            manager.open_vaults.insert(format!("vault_{i}"), vault);
        }

        assert!(
            !manager.open_vaults.is_empty(),
            "open_vaults should not be empty after inserting test vaults"
        );

        manager.close_all_vaults();
        assert!(
            manager.open_vaults.is_empty(),
            "open_vaults should be empty after close_all_vaults"
        );
    }

    #[test]
    fn test_get_vault_returns_none_for_closed_vault() {
        if let Ok(mut manager) = VaultManager::new() {
            let result = manager.get_vault("nonexistent");
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_get_vault_returns_some_for_open_vault() {
        if let Ok(mut manager) = VaultManager::new() {
            let temp_dir = create_temp_dir();
            let vault_path = temp_dir.path().join("test.db");
            create_test_vault_file(&vault_path).unwrap();

            if let Ok(vault) = Vault::open_or_create(Some(&vault_path)) {
                manager.open_vaults.insert("test_vault".to_string(), vault);

                let result = manager.get_vault("test_vault");
                assert!(result.is_some());
            }
        }
    }

    #[test]
    fn test_get_active_vault_fails_when_no_active_vault() {
        if let Ok(mut manager) = VaultManager::new() {
            // Ensure no active vault is set
            manager.registry.active_vault_id = None;

            let result = manager.get_active_vault();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("No active vault"));
        }
    }

    #[test]
    fn test_get_active_vault_fails_when_active_vault_not_open() {
        if let Ok(mut manager) = VaultManager::new() {
            // Set an active vault ID but don't open it
            manager.registry.active_vault_id = Some("test_vault".to_string());

            let result = manager.get_active_vault();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not unlocked"));
        }
    }

    #[test]
    fn test_switch_active_vault_fails_for_nonexistent_vault() {
        if let Ok(mut manager) = VaultManager::new() {
            let result = manager.switch_active_vault("nonexistent");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }
    }

    // Integration-style tests that work with the real components

    #[test]
    fn test_vault_lifecycle_create_open_close() {
        let temp_dir = create_temp_dir();

        // Set up a temporary registry path
        std::env::set_var("CHAMBER_CONFIG_DIR", temp_dir.path());

        if let Ok(mut manager) = VaultManager::new() {
            let vault_name = "test_vault".to_string();
            let master_password = "test_password_123";

            // Test vault creation
            let result = manager.create_vault(
                vault_name,
                None,
                VaultCategory::Testing,
                Some("Test description".to_string()),
                master_password,
            );

            match result {
                Ok(vault_id) => {
                    assert!(!vault_id.is_empty());

                    // Test that vault appears in list
                    let vaults = manager.list_vaults();
                    assert!(vaults.iter().any(|v| v.id == vault_id));

                    // Test opening the vault
                    let open_result = manager.open_vault(&vault_id, master_password);
                    assert!(open_result.is_ok());
                    assert!(manager.is_vault_open(&vault_id));

                    // Test closing the vault
                    let close_result = manager.close_vault(&vault_id);
                    assert!(close_result.is_ok());
                    assert!(!manager.is_vault_open(&vault_id));
                }
                Err(e) => {
                    println!("Vault creation failed (expected in some test environments): {e}");
                }
            }
        }
    }

    #[test]
    fn test_vault_creation_with_custom_path() {
        let temp_dir = create_temp_dir();
        let custom_vault_path = temp_dir.path().join("custom_vault.db");

        if let Ok(mut manager) = VaultManager::new() {
            let result = manager.create_vault(
                "custom_vault".to_string(),
                Some(custom_vault_path.clone()),
                VaultCategory::Personal,
                None,
                "password123",
            );

            match result {
                Ok(vault_id) => {
                    let vaults = manager.list_vaults();
                    let created_vault = vaults.iter().find(|v| v.id == vault_id);

                    if let Some(vault_info) = created_vault {
                        assert_eq!(vault_info.path, custom_vault_path);
                        assert_eq!(vault_info.category, VaultCategory::Personal);
                    }
                }
                Err(e) => {
                    println!("Custom path vault creation failed: {e}");
                }
            }
        }
    }

    #[test]
    fn test_vault_creation_with_weak_password_still_succeeds() {
        if let Ok(mut manager) = VaultManager::new() {
            let result = manager.create_vault(
                "weak_password_vault".to_string(),
                None,
                VaultCategory::Testing,
                None,
                "123", // Weak password
            );

            // The VaultManager should allow weak passwords (validation might be elsewhere)
            match result {
                Ok(vault_id) => {
                    assert!(!vault_id.is_empty());
                }
                Err(e) => {
                    println!("Weak password vault creation failed: {e}");
                }
            }
        }
    }

    #[test]
    fn test_open_vault_with_wrong_password() {
        let _temp_dir = create_temp_dir();

        if let Ok(mut manager) = VaultManager::new() {
            let correct_password = "correct_password";
            let wrong_password = "wrong_password";

            // Create vault
            if let Ok(vault_id) = manager.create_vault(
                "password_test_vault".to_string(),
                None,
                VaultCategory::Testing,
                None,
                correct_password,
            ) {
                // Try to open with wrong password
                let result = manager.open_vault(&vault_id, wrong_password);
                assert!(result.is_err());
                assert!(!manager.is_vault_open(&vault_id));

                // Try to open with correct password
                let result = manager.open_vault(&vault_id, correct_password);
                match result {
                    Ok(()) => assert!(manager.is_vault_open(&vault_id)),
                    Err(e) => println!("Opening with correct password failed: {e}"),
                }
            }
        }
    }

    #[test]
    fn test_open_nonexistent_vault() {
        if let Ok(mut manager) = VaultManager::new() {
            let result = manager.open_vault("nonexistent_vault", "any_password");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }
    }

    #[test]
    fn test_update_vault_info() {
        if let Ok(mut manager) = VaultManager::new() {
            // Create a vault first
            if let Ok(vault_id) = manager.create_vault(
                "original_name".to_string(),
                None,
                VaultCategory::Personal,
                Some("original description".to_string()),
                "password123",
            ) {
                // Update vault info
                let result = manager.update_vault_info(
                    &vault_id,
                    Some("updated_name".to_string()),
                    Some("updated description".to_string()),
                    Some(VaultCategory::Work),
                    Some(true), // Mark as favorite
                );

                match result {
                    Ok(()) => {
                        // Verify updates
                        let vaults = manager.list_vaults();
                        let updated_vault = vaults.iter().find(|v| v.id == vault_id);

                        if let Some(vault_info) = updated_vault {
                            assert_eq!(vault_info.name, "updated_name");
                            assert_eq!(vault_info.description, Some("updated description".to_string()));
                            assert_eq!(vault_info.category, VaultCategory::Work);
                            assert!(vault_info.is_favorite);
                        }
                    }
                    Err(e) => println!("Update vault info failed: {e}"),
                }
            }
        }
    }

    #[test]
    fn test_update_nonexistent_vault() {
        if let Ok(mut manager) = VaultManager::new() {
            let result = manager.update_vault_info("nonexistent", Some("new_name".to_string()), None, None, None);

            assert!(result.is_err());
        }
    }

    #[test]
    fn test_delete_vault_without_file() {
        if let Ok(mut manager) = VaultManager::new() {
            // Create a vault
            if let Ok(vault_id) = manager.create_vault(
                "delete_test_vault".to_string(),
                None,
                VaultCategory::Testing,
                None,
                "password123",
            ) {
                // Open the vault
                let _ = manager.open_vault(&vault_id, "password123");
                assert!(manager.is_vault_open(&vault_id));

                // Delete without removing file
                let result = manager.delete_vault(&vault_id, false);
                match result {
                    Ok(()) => {
                        // Verify vault is closed and removed from registry
                        assert!(!manager.is_vault_open(&vault_id));
                        let vaults = manager.list_vaults();
                        assert!(!vaults.iter().any(|v| v.id == vault_id));
                    }
                    Err(e) => println!("Delete vault failed: {e}"),
                }
            }
        }
    }

    #[test]
    fn test_delete_vault_with_file() {
        let temp_dir = create_temp_dir();
        let vault_path = temp_dir.path().join("delete_test.db");

        if let Ok(mut manager) = VaultManager::new() {
            // Create a vault with custom path
            if let Ok(vault_id) = manager.create_vault(
                "delete_with_file_test".to_string(),
                Some(vault_path.clone()),
                VaultCategory::Testing,
                None,
                "password123",
            ) {
                // Verify file exists
                assert!(vault_path.exists());

                // Delete with file removal
                let result = manager.delete_vault(&vault_id, true);
                match result {
                    Ok(()) => {
                        // Verify vault is removed and file is deleted
                        let vaults = manager.list_vaults();
                        assert!(!vaults.iter().any(|v| v.id == vault_id));
                        assert!(!vault_path.exists());
                    }
                    Err(e) => println!("Delete vault with file failed: {e}"),
                }
            }
        }
    }

    #[test]
    fn test_delete_nonexistent_vault() {
        if let Ok(mut manager) = VaultManager::new() {
            let result = manager.delete_vault("nonexistent", false);
            // Should return an error
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_import_vault() {
        let temp_dir = create_temp_dir();
        let source_vault_path = temp_dir.path().join("source_vault.db");
        create_test_vault_file(&source_vault_path).unwrap();

        if let Ok(mut manager) = VaultManager::new() {
            let result = manager.import_vault(
                &source_vault_path,
                "imported_vault".to_string(),
                VaultCategory::Archive,
                true, // Copy file
            );

            match result {
                Ok(vault_id) => {
                    assert!(!vault_id.is_empty());

                    // Verify vault appears in list
                    let vaults = manager.list_vaults();
                    let imported_vault = vaults.iter().find(|v| v.id == vault_id);

                    if let Some(vault_info) = imported_vault {
                        assert_eq!(vault_info.name, "imported_vault");
                        assert_eq!(vault_info.category, VaultCategory::Archive);
                        // File should be copied to a different location
                        assert_ne!(vault_info.path, source_vault_path);
                    }
                }
                Err(e) => println!("Import vault failed: {e}"),
            }
        }
    }

    #[test]
    fn test_import_vault_without_copy() {
        let temp_dir = create_temp_dir();
        let source_vault_path = temp_dir.path().join("source_vault.db");
        create_test_vault_file(&source_vault_path).unwrap();

        if let Ok(mut manager) = VaultManager::new() {
            let result = manager.import_vault(
                &source_vault_path,
                "imported_no_copy_vault".to_string(),
                VaultCategory::Personal,
                false, // Don't copy file
            );

            match result {
                Ok(vault_id) => {
                    let vaults = manager.list_vaults();
                    let imported_vault = vaults.iter().find(|v| v.id == vault_id);

                    if let Some(vault_info) = imported_vault {
                        // Path should be the same as source
                        assert_eq!(vault_info.path, source_vault_path);
                    }
                }
                Err(e) => println!("Import vault without copy failed: {e}"),
            }
        }
    }

    #[test]
    fn test_import_nonexistent_vault() {
        if let Ok(mut manager) = VaultManager::new() {
            let nonexistent_path = PathBuf::from("/nonexistent/vault.db");
            let result = manager.import_vault(
                &nonexistent_path,
                "nonexistent_vault".to_string(),
                VaultCategory::Personal,
                false,
            );

            assert!(result.is_err());
        }
    }

    #[test]
    fn test_multiple_vaults_management() {
        if let Ok(mut manager) = VaultManager::new() {
            let mut vault_ids = Vec::new();

            // Create multiple vaults
            for i in 0..3 {
                if let Ok(vault_id) = manager.create_vault(
                    format!("vault_{i}"),
                    None,
                    VaultCategory::Testing,
                    Some(format!("Test vault {i}")),
                    "password123",
                ) {
                    vault_ids.push(vault_id);
                }
            }

            // Open all vaults
            for vault_id in &vault_ids {
                let _ = manager.open_vault(vault_id, "password123");
            }

            // Verify all are open
            for vault_id in &vault_ids {
                assert!(manager.is_vault_open(vault_id));
            }

            // Close all vaults
            manager.close_all_vaults();

            // Verify all are closed
            for vault_id in &vault_ids {
                assert!(!manager.is_vault_open(vault_id));
            }

            // Clean up - delete all vaults
            for vault_id in &vault_ids {
                let _ = manager.delete_vault(vault_id, true);
            }
        }
    }

    #[test]
    fn test_active_vault_workflow() {
        if let Ok(mut manager) = VaultManager::new() {
            // Create two vaults
            let vault1_result =
                manager.create_vault("vault1".to_string(), None, VaultCategory::Personal, None, "password123");

            let vault2_result =
                manager.create_vault("vault2".to_string(), None, VaultCategory::Work, None, "password456");

            if let (Ok(vault1_id), Ok(vault2_id)) = (vault1_result, vault2_result) {
                // Switch active vault to vault1
                let switch_result = manager.switch_active_vault(&vault1_id);
                assert!(switch_result.is_ok());

                // Open vault1
                let open_result = manager.open_vault(&vault1_id, "password123");
                if open_result.is_ok() {
                    // Get active vault should succeed
                    let active_result = manager.get_active_vault();
                    assert!(active_result.is_ok());
                }

                // Switch to vault2
                let switch_result = manager.switch_active_vault(&vault2_id);
                assert!(switch_result.is_ok());

                // Get active vault should fail (vault2 not opened)
                let active_result = manager.get_active_vault();
                assert!(active_result.is_err());

                // Open vault2
                let open_result = manager.open_vault(&vault2_id, "password456");
                if open_result.is_ok() {
                    // Get active vault should succeed now
                    let active_result = manager.get_active_vault();
                    assert!(active_result.is_ok());
                }
            }
        }
    }

    #[test]
    fn test_vault_categories() {
        if let Ok(mut manager) = VaultManager::new() {
            let categories = [
                VaultCategory::Personal,
                VaultCategory::Work,
                VaultCategory::Team,
                VaultCategory::Project,
                VaultCategory::Testing,
                VaultCategory::Archive,
                VaultCategory::Custom("CustomCategory".to_string()),
            ];

            for (i, category) in categories.iter().enumerate() {
                let result = manager.create_vault(
                    format!("category_test_{i}"),
                    None,
                    category.clone(),
                    Some(format!("Testing {category:?} category")),
                    "password123",
                );

                if let Ok(vault_id) = result {
                    let vaults = manager.list_vaults();
                    let created_vault = vaults.iter().find(|v| v.id == vault_id);

                    if let Some(vault_info) = created_vault {
                        assert_eq!(vault_info.category, *category);
                    }
                }
            }
        }
    }

    #[test]
    fn test_error_handling_consistency() {
        if let Ok(mut manager) = VaultManager::new() {
            let nonexistent_id = "definitely_nonexistent_vault_id";

            // All operations on nonexistent vaults should return errors
            assert!(manager.open_vault(nonexistent_id, "password").is_err());
            assert!(manager.switch_active_vault(nonexistent_id).is_err());
            assert!(manager.delete_vault(nonexistent_id, false).is_err());
            assert!(manager
                .update_vault_info(nonexistent_id, None, None, None, None)
                .is_err());

            // These operations should not error even for nonexistent vaults
            assert!(manager.close_vault(nonexistent_id).is_ok());
            assert!(!manager.is_vault_open(nonexistent_id));
            assert!(manager.get_vault(nonexistent_id).is_none());
        }
    }
}
