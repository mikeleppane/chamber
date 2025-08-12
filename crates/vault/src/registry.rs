use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    /// Unique identifier for the vault
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Full path to the vault database file
    pub path: PathBuf,
    /// Creation timestamp
    pub created_at: OffsetDateTime,
    /// Last accessed timestamp
    pub last_accessed: OffsetDateTime,
    /// Vault description/notes
    pub description: Option<String>,
    /// Vault category (personal, work, team, etc.)
    pub category: VaultCategory,
    /// Whether this vault is currently active
    pub is_active: bool,
    /// Favorite/pinned status
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VaultCategory {
    Personal,
    Work,
    Team,
    Project,
    Testing,
    Archive,
    Custom(String),
}

impl std::fmt::Display for VaultCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultCategory::Personal => write!(f, "Personal"),
            VaultCategory::Work => write!(f, "Work"),
            VaultCategory::Team => write!(f, "Team"),
            VaultCategory::Project => write!(f, "Project"),
            VaultCategory::Testing => write!(f, "Testing"),
            VaultCategory::Archive => write!(f, "Archive"),
            VaultCategory::Custom(name) => write!(f, "{name}"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultRegistry {
    /// Map of vault ID to vault info
    pub vaults: HashMap<String, VaultInfo>,
    /// Currently active vault ID
    pub active_vault_id: Option<String>,
    /// Registry file path
    #[serde(skip)]
    registry_path: PathBuf,
}

impl VaultRegistry {
    /// Loads the `VaultRegistry` from the default registry path or creates a new one if it does not exist.
    ///
    /// This function attempts to load a `VaultRegistry` instance from a file at the default
    /// registry path. If the file exists, it is deserialized into a `VaultRegistry` object. If the
    /// file does not exist, a new `VaultRegistry` is created with default values, including the creation
    /// of a default vault if none exist. The new registry is then saved to disk.
    ///
    /// # Returns
    /// * `Ok(Self)` - A successfully loaded or newly created `VaultRegistry`.
    /// * `Err(Error)` - An error encountered during file operations, deserialization, or registry creation.
    ///
    /// # Errors
    /// - Returns an error if the default registry path cannot be determined.
    /// - Returns an error if reading the registry file fails.
    /// - Returns an error if deserialization of the registry content fails.
    /// - Returns an error if saving a newly created registry fails.
    /// - Returns an error if creating a default vault fails.
    pub fn load() -> Result<Self> {
        let registry_path = Self::default_registry_path()?;

        if registry_path.exists() {
            let content = fs::read_to_string(&registry_path)?;
            let mut registry: VaultRegistry = serde_json::from_str(&content)?;
            registry.registry_path = registry_path;
            Ok(registry)
        } else {
            // Create default registry with a default vault
            let mut registry = Self {
                vaults: HashMap::new(),
                active_vault_id: None,
                registry_path,
            };

            // Create default vault if none exists
            if registry.vaults.is_empty() {
                registry.create_default_vault()?;
            }

            registry.save()?;
            Ok(registry)
        }
    }

    fn default_registry_path() -> Result<PathBuf> {
        let base = dirs::config_dir().ok_or_else(|| anyhow!("No config directory found"))?;
        Ok(base.join("chamber").join("registry.json"))
    }

    fn create_default_vault(&mut self) -> Result<()> {
        let vault_id = "default".to_string();
        let vault_path = Self::default_vault_path(&vault_id)?;

        let vault_info = VaultInfo {
            id: vault_id.clone(),
            name: "Default Vault".to_string(),
            path: vault_path,
            created_at: OffsetDateTime::now_utc(),
            last_accessed: OffsetDateTime::now_utc(),
            description: Some("Default personal vault".to_string()),
            category: VaultCategory::Personal,
            is_active: true,
            is_favorite: false,
        };

        self.vaults.insert(vault_id.clone(), vault_info);
        self.active_vault_id = Some(vault_id);
        Ok(())
    }

    fn default_vault_path(vault_id: &str) -> Result<PathBuf> {
        let base = dirs::config_dir().ok_or_else(|| anyhow!("No config directory found"))?;
        let chamber_dir = base.join("chamber");

        // Ensure the chamber directory exists
        std::fs::create_dir_all(&chamber_dir)?;

        // Put vault files directly in the chamber directory, not in a vault subdirectory
        Ok(chamber_dir.join(format!("{vault_id}.db")))
    }

    /// Creates a new vault and stores its metadata.
    ///
    /// This method is used to create and register a new vault in the system. It generates a unique
    /// identifier for the vault, validates that the identifier does not already exist, and saves the
    /// vault's metadata after ensuring that the required directory structure exists.
    ///
    /// ### Parameters
    ///
    /// - `name`: The name of the vault to be created. This is used to generate a unique identifier for the vault.
    /// - `path`: An optional custom file system path for the vault. If not provided, a default path is used.
    /// - `category`: The category of the vault. Defines the type or purpose of the vault.
    /// - `description`: An optional description providing additional details about the vault's purpose or contents.
    ///
    /// ### Returns
    ///
    /// - `Ok(String)`: The unique identifier of the newly created vault if the operation is successful.
    /// - `Err(anyhow::Error)`: An error if the vault ID already exists, the path cannot be resolved, or any I/O operation fails.
    ///
    /// ### Behavior
    ///
    /// 1. A unique vault ID is generated using the provided `name`.
    /// 2. Checks if a vault with the generated ID already exists.
    ///    - If a duplicate exists, returns an error.
    /// 3. Computes the vault's file system path:
    ///    - If a custom `path` is provided, it is used.
    ///    - Otherwise, a default path is created based on the vault ID.
    /// 4. Ensures that the required directory structure exists by creating any missing parent directories.
    /// 5. Constructs a `VaultInfo` object with the vault's metadata, including timestamps for `created_at` and `last_accessed`.
    /// 6. Inserts the vault metadata into the internal vault registry.
    /// 7. Saves the updated vault registry to persistent storage.
    ///
    /// ### Errors
    ///
    /// - Returns an error if:
    ///   - A vault with the same ID already exists.
    ///   - There is an issue generating or resolving the vault's path.
    ///   - The file system operation (e.g., creating directories) fails.
    ///   - Saving the updated vault registry fails.
    pub fn create_vault(
        &mut self,
        name: String,
        path: Option<PathBuf>,
        category: VaultCategory,
        description: Option<String>,
    ) -> Result<String> {
        let vault_id = self.generate_vault_id(&name);

        // Check if vault ID already exists
        if self.vaults.contains_key(&vault_id) {
            return Err(anyhow!("Vault with ID '{}' already exists", vault_id));
        }

        let vault_path = if let Some(custom_path) = path {
            custom_path
        } else {
            Self::default_vault_path(&vault_id)?
        };

        // Ensure directory exists
        if let Some(parent) = vault_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let vault_info = VaultInfo {
            id: vault_id.clone(),
            name,
            path: vault_path,
            created_at: OffsetDateTime::now_utc(),
            last_accessed: OffsetDateTime::now_utc(),
            description,
            category,
            is_active: false,
            is_favorite: false,
        };

        self.vaults.insert(vault_id.clone(), vault_info);
        self.save()?;

        Ok(vault_id)
    }

    fn generate_vault_id(&self, name: &str) -> String {
        let base_id = name
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>()
            .replace(' ', "-");

        let mut counter = 0;
        let mut vault_id = base_id.clone();

        while self.vaults.contains_key(&vault_id) {
            counter += 1;
            vault_id = format!("{base_id}-{counter}");
        }

        vault_id
    }

    /// Get all vaults
    #[must_use]
    pub fn list_vaults(&self) -> Vec<&VaultInfo> {
        let mut vaults: Vec<&VaultInfo> = self.vaults.values().collect();
        vaults.sort_by(|a, b| {
            // Favorites first, then by last accessed
            match (a.is_favorite, b.is_favorite) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.last_accessed.cmp(&a.last_accessed),
            }
        });
        vaults
    }

    /// Get vault by ID
    #[must_use]
    pub fn get_vault(&self, vault_id: &str) -> Option<&VaultInfo> {
        self.vaults.get(vault_id)
    }

    /// Sets the active vault by its ID and updates the state of the previously active vault, if any.
    ///
    /// This function performs the following actions:
    /// - Checks if the provided `vault_id` exists in the `vaults` map. If it doesn't exist, returns
    ///   an error.
    /// - Deactivates the previously active vault, if one exists, by setting its `is_active` field
    ///   to `false`.
    /// - Activates the new vault by setting its `is_active` field to `true` and updating its
    ///   `last_accessed` timestamp to the current UTC time.
    /// - Updates the `active_vault_id` to the specified `vault_id`.
    /// - Saves the updated state to persistent storage by calling the `save` method.
    ///
    /// # Parameters
    /// - `vault_id`: A string slice representing the ID of the vault to be activated.
    ///
    /// # Returns
    /// - `Ok(())`: If the vault is successfully set as active, and the changes are saved.
    /// - `Err`: If the specified vault ID does not exist in the `vaults` map, or if saving the state fails.
    ///
    /// # Errors
    /// - Returns an error if:
    ///   - The provided vault ID does not match any keys in the `vaults` map.
    ///   - The `save` operation fails to persist in the updated state.
    pub fn set_active_vault(&mut self, vault_id: &str) -> Result<()> {
        if !self.vaults.contains_key(vault_id) {
            return Err(anyhow!("Vault with ID '{}' not found", vault_id));
        }

        // Update previous active vault
        if let Some(prev_id) = &self.active_vault_id {
            if let Some(prev_vault) = self.vaults.get_mut(prev_id) {
                prev_vault.is_active = false;
            }
        }

        // Set a new active vault
        if let Some(vault) = self.vaults.get_mut(vault_id) {
            vault.is_active = true;
            vault.last_accessed = OffsetDateTime::now_utc();
        }

        self.active_vault_id = Some(vault_id.to_string());
        self.save()?;
        Ok(())
    }

    /// Get currently active vault
    #[must_use]
    pub fn get_active_vault(&self) -> Option<&VaultInfo> {
        self.active_vault_id.as_ref().and_then(|id| self.vaults.get(id))
    }

    ///
    /// Updates the properties of a vault identified by its ID.
    ///
    /// This method allows modifying the name, description, category, and favorite status of a specified vault.
    /// If any of these properties are `None`, the corresponding field in the vault will remain unchanged.
    /// Additionally, the `last_accessed` timestamp of the vault is updated to the current UTC time.
    ///
    /// # Arguments
    ///
    /// * `vault_id` - A string slice representing the unique ID of the vault to be updated.
    /// * `name` - An optional string representing the new name of the vault.
    /// * `description` - An optional string representing the new description of the vault.
    /// * `category` - An optional `VaultCategory` value to set a new category for the vault.
    /// * `is_favorite` - An optional boolean indicating whether the vault is marked as a favorite.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the update succeeded.
    /// * `Err(anyhow::Error)` - If the vault ID is not found or if saving the vaults fails.
    ///
    /// # Errors
    ///
    /// This function returns an error in the following cases:
    /// - If the vault with the specified `vault_id` does not exist.
    /// - If there is an error while saving the updated vault data.
    pub fn update_vault(
        &mut self,
        vault_id: &str,
        name: Option<String>,
        description: Option<String>,
        category: Option<VaultCategory>,
        is_favorite: Option<bool>,
    ) -> Result<()> {
        let vault = self
            .vaults
            .get_mut(vault_id)
            .ok_or_else(|| anyhow!("Vault with ID '{}' not found", vault_id))?;

        if let Some(new_name) = name {
            vault.name = new_name;
        }
        if let Some(new_desc) = description {
            vault.description = Some(new_desc);
        }
        if let Some(new_category) = category {
            vault.category = new_category;
        }
        if let Some(favorite) = is_favorite {
            vault.is_favorite = favorite;
        }

        vault.last_accessed = OffsetDateTime::now_utc();
        self.save()?;
        Ok(())
    }

    /// Deletes a vault identified by `vault_id` from the vault manager.
    ///
    /// # Parameters
    ///
    /// - `vault_id`: A string slice that represents the ID of the vault to delete.
    /// - `delete_file`: A boolean indicating whether the physical file associated with the vault should be deleted.
    ///
    /// # Returns
    ///
    /// - `Result<()>`: Returns `Ok(())` if the operation is successful. Otherwise, returns an error.
    ///
    /// # Errors
    ///
    /// - Returns an error if a vault with the specified `vault_id` cannot be found.
    /// - Returns an error if the vault being deleted is the last remaining vault in the manager.
    /// - Returns an error if there is an issue deleting the associated physical file (if `delete_file` is `true`).
    /// - Returns an error if updating the active vault fails or if saving the state of the vault manager fails.
    ///
    /// # Panics
    ///
    /// # Behavior
    ///
    /// - If the vault with the specified `vault_id` is not found in the vault list, an error is returned.
    /// - If the vault manager contains only one vault, deletion is prohibited, and an error is returned.
    /// - If `delete_file` is `true` and the associated vault's physical file exists, the file will be deleted from the filesystem.
    /// - If the vault being deleted is the active vault, another vault is automatically selected as the active one.
    /// - The vault is removed from the internal list of vaults, and the manager's state is saved.
    ///
    /// # Note
    /// - Ensure at least one vault remains in the manager to avoid errors.
    /// - Consider the implications of allowing physical file deletion (`delete_file`) if there are no backups.
    pub fn delete_vault(&mut self, vault_id: &str, delete_file: bool) -> Result<()> {
        let vault = self
            .vaults
            .get(vault_id)
            .ok_or_else(|| anyhow!("Vault with ID '{}' not found", vault_id))?;

        // Don't allow deleting if it's the only vault
        if self.vaults.len() == 1 {
            return Err(anyhow!("Cannot delete the last remaining vault"));
        }

        // Delete physical file if requested
        if delete_file && vault.path.exists() {
            fs::remove_file(&vault.path)?;
        }

        // If this was the active vault, set another one as active
        if self.active_vault_id.as_ref() == Some(&String::from(vault_id)) {
            let next_vault_id = self.vaults.keys().find(|&id| id != vault_id).unwrap().clone();
            self.set_active_vault(&next_vault_id)?;
        }

        self.vaults.remove(vault_id);
        self.save()?;
        Ok(())
    }

    /// Save registry to file
    fn save(&self) -> Result<()> {
        if let Some(parent) = self.registry_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&self.registry_path, content)?;
        Ok(())
    }

    /// Imports a vault into the system.
    ///
    /// This function handles the import of a vault by accepting its file path and metadata, such as its
    /// name and category. It provides the option to copy the vault file into a default storage location
    /// or keep it in its original location. The imported vault is then added to the system's vault
    /// collection and saved.
    ///
    /// # Parameters
    ///
    /// * `vault_file`: The file path to the vault being imported. This must be a valid path to an
    ///   existing file.
    /// * `name`: A unique string representing the name of the vault. This is used as part of the
    ///   vault's unique identifier and in its metadata.
    /// * `category`: The category of the vault, represented by the `VaultCategory` type. This provides
    ///   additional classification for the vault.
    /// * `copy_file`: A boolean flag indicating whether the vault file should be copied to the default
    ///   vault storage location. If `true`, the file is copied to the default path; otherwise, the
    ///   imported vault references the original file location.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// * `Ok(String)`: The unique identifier (ID) of the imported vault, if the operation is successful.
    /// * `Err(anyhow::Error)`: An error if the file import fails. Some potential causes include:
    ///   - The file does not exist.
    ///   - Errors during file operations, such as copying or directory creation.
    ///   - Failures in the process of storing vault information persistently.
    ///
    /// # Errors
    ///
    /// This function will return an error in the following situations:
    /// * The `vault_file` does not exist at the given path.
    /// * Any I/O errors occur during file copying or directory creation (e.g., lack of permissions,
    ///   invalid paths, etc.).
    /// * Errors occur when generating the vault ID or saving the vault data after import.
    pub fn import_vault(
        &mut self,
        vault_file: &Path,
        name: String,
        category: VaultCategory,
        copy_file: bool,
    ) -> Result<String> {
        if !vault_file.exists() {
            return Err(anyhow!("Vault file does not exist: {}", vault_file.display()));
        }

        let vault_id = self.generate_vault_id(&name);

        let vault_path = if copy_file {
            let new_path = Self::default_vault_path(&vault_id)?;
            if let Some(parent) = new_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(vault_file, &new_path)?;
            new_path
        } else {
            vault_file.to_path_buf()
        };

        let vault_info = VaultInfo {
            id: vault_id.clone(),
            name,
            path: vault_path,
            created_at: OffsetDateTime::now_utc(),
            last_accessed: OffsetDateTime::now_utc(),
            description: Some("Imported vault".to_string()),
            category,
            is_active: false,
            is_favorite: false,
        };

        self.vaults.insert(vault_id.clone(), vault_info);
        self.save()?;

        Ok(vault_id)
    }
}
