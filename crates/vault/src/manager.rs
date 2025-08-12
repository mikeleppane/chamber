use crate::registry::VaultInfo;
use crate::{Vault, VaultCategory, VaultRegistry};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;

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
