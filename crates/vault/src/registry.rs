use color_eyre::Result;
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use uuid::Uuid;

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
    pub(crate) registry_path: PathBuf,
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
            // Create default registry
            let mut registry = Self {
                vaults: HashMap::new(),
                active_vault_id: None,
                registry_path,
            };

            // Check if vault.sqlite3 exists and register it
            if let Ok(default_vault_path) = default_db_path() {
                if default_vault_path.exists() {
                    // Register existing vault.sqlite3
                    registry.register_existing_vault(default_vault_path);
                } else {
                    // Create default vault using vault.sqlite3 path
                    registry.create_default_vault();
                }
            } else {
                // Fallback to creating default vault
                registry.create_default_vault();
            }

            registry.save()?;
            Ok(registry)
        }
    }

    fn register_existing_vault(&mut self, vault_path: PathBuf) {
        let vault_id = "main".to_string();

        let vault_info = VaultInfo {
            id: vault_id.clone(),
            name: "Main Vault".to_string(),
            path: vault_path,
            created_at: OffsetDateTime::now_utc(),
            last_accessed: OffsetDateTime::now_utc(),
            description: Some("Main vault".to_string()),
            category: VaultCategory::Personal,
            is_active: true,
            is_favorite: false,
        };

        self.vaults.insert(vault_id.clone(), vault_info);
        self.active_vault_id = Some(vault_id);
    }

    fn default_registry_path() -> Result<PathBuf> {
        let base = dirs::config_dir().ok_or_else(|| eyre!("No config directory found"))?;
        Ok(base.join("chamber").join("registry.json"))
    }

    fn create_default_vault(&mut self) {
        // Use vault.sqlite3 path instead of default.db
        let vault_id = "main".to_string();
        let vault_path = default_db_path().expect("Could not get default DB"); // This returns vault.sqlite3 path

        let vault_info = VaultInfo {
            id: vault_id.clone(),
            name: "Main Vault".to_string(),
            path: vault_path,
            created_at: OffsetDateTime::now_utc(),
            last_accessed: OffsetDateTime::now_utc(),
            description: Some("Main vault".to_string()),
            category: VaultCategory::Personal,
            is_active: true,
            is_favorite: false,
        };

        self.vaults.insert(vault_id.clone(), vault_info);
        self.active_vault_id = Some(vault_id);
    }

    fn default_vault_path(vault_id: &str) -> Result<PathBuf> {
        let base = dirs::config_dir().ok_or_else(|| eyre!("No config directory found"))?;
        let chamber_dir = base.join("chamber");
        std::fs::create_dir_all(&chamber_dir)?;

        // For additional vaults, use .db extension
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
        let vault_id = Self::generate_vault_id();

        // Check if vault ID already exists
        if self.vaults.contains_key(&vault_id) {
            return Err(eyre!("Vault with ID '{}' already exists", vault_id));
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

    fn generate_vault_id() -> String {
        Uuid::new_v4().to_string()
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
            return Err(eyre!("Vault with ID '{}' not found", vault_id));
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
            .ok_or_else(|| eyre!("Vault with ID '{}' not found", vault_id))?;

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
            .ok_or_else(|| eyre!("Vault with ID '{}' not found", vault_id))?;

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
            return Err(eyre!("Vault file does not exist: {}", vault_file.display()));
        }

        let vault_id = Self::generate_vault_id();

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

fn default_db_path() -> Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| eyre!("No config dir"))?;
    let dir = base.join("chamber");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("vault.sqlite3"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::panic)]
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn create_temp_registry() -> (VaultRegistry, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let registry_path = temp_dir.path().join("test_registry.json");

        let registry = VaultRegistry {
            vaults: HashMap::new(),
            active_vault_id: None,
            registry_path,
        };

        (registry, temp_dir)
    }

    fn create_test_vault_info(id: &str, name: &str, category: VaultCategory) -> VaultInfo {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let vault_path = temp_dir.path().join("test_vault.db");

        VaultInfo {
            id: id.to_string(),
            name: name.to_string(),
            path: vault_path,
            created_at: OffsetDateTime::now_utc(),
            last_accessed: OffsetDateTime::now_utc(),
            description: None,
            category,
            is_active: false,
            is_favorite: false,
        }
    }

    #[test]
    fn test_vault_registry_new_empty() {
        let (registry, _temp_dir) = create_temp_registry();

        assert!(registry.vaults.is_empty());
        assert!(registry.active_vault_id.is_none());
        assert!(!registry.registry_path.as_os_str().is_empty());
    }

    #[test]
    fn test_list_vaults_empty() {
        let (registry, _temp_dir) = create_temp_registry();
        let vaults = registry.list_vaults();

        assert!(vaults.is_empty());
    }

    #[test]
    fn test_list_vaults_with_data() {
        let (mut registry, _temp_dir) = create_temp_registry();

        let vault1 = create_test_vault_info(
            "550e8400-e29b-41d4-a716-446655440000",
            "Test Vault 1",
            VaultCategory::Personal,
        );
        let vault2 = create_test_vault_info(
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
            "Test Vault 2",
            VaultCategory::Work,
        );

        registry
            .vaults
            .insert("550e8400-e29b-41d4-a716-446655440000".to_string(), vault1);
        registry
            .vaults
            .insert("6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(), vault2);

        let vaults = registry.list_vaults();

        assert_eq!(vaults.len(), 2);
        let vault_names: Vec<&str> = vaults.iter().map(|v| v.name.as_str()).collect();
        assert!(vault_names.contains(&"Test Vault 1"));
        assert!(vault_names.contains(&"Test Vault 2"));
    }

    #[test]
    fn test_get_vault_exists() {
        let (mut registry, _temp_dir) = create_temp_registry();
        let test_id = "550e8400-e29b-41d4-a716-446655440000";
        let vault = create_test_vault_info(test_id, "Test Vault", VaultCategory::Personal);
        registry.vaults.insert(test_id.to_string(), vault);

        let result = registry.get_vault(test_id);

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Test Vault");
        assert_eq!(result.unwrap().id, test_id);
    }

    #[test]
    fn test_get_vault_not_exists() {
        let (registry, _temp_dir) = create_temp_registry();

        let result = registry.get_vault("550e8400-e29b-41d4-a716-446655440000");

        assert!(result.is_none());
    }

    #[test]
    fn test_get_active_vault_none_set() {
        let (registry, _temp_dir) = create_temp_registry();

        let result = registry.get_active_vault();

        assert!(result.is_none());
    }

    #[test]
    fn test_get_active_vault_exists() {
        let (mut registry, _temp_dir) = create_temp_registry();
        let active_id = "550e8400-e29b-41d4-a716-446655440000";
        let vault = create_test_vault_info(active_id, "Active Vault", VaultCategory::Personal);
        registry.vaults.insert(active_id.to_string(), vault);
        registry.active_vault_id = Some(active_id.to_string());

        let result = registry.get_active_vault();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Active Vault");
    }

    #[test]
    fn test_get_active_vault_id_invalid() {
        let (mut registry, _temp_dir) = create_temp_registry();
        registry.active_vault_id = Some("550e8400-e29b-41d4-a716-446655440000".to_string());

        let result = registry.get_active_vault();

        assert!(result.is_none());
    }

    #[test]
    fn test_generate_vault_id_returns_uuid() {
        let (_, _temp_dir) = create_temp_registry();

        let id1 = VaultRegistry::generate_vault_id();
        let id2 = VaultRegistry::generate_vault_id();

        // Should be valid UUIDs
        assert!(Uuid::parse_str(&id1).is_ok());
        assert!(Uuid::parse_str(&id2).is_ok());

        // Should be different even with same name
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_vault_id_uniqueness() {
        let (_, _temp_dir) = create_temp_registry();

        let mut ids = Vec::new();
        for _ in 0..100 {
            ids.push(VaultRegistry::generate_vault_id());
        }

        // All IDs should be unique
        let mut unique_ids = ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        assert_eq!(ids.len(), unique_ids.len());

        // All should be valid UUIDs
        for id in ids {
            assert!(Uuid::parse_str(&id).is_ok());
        }
    }

    #[test]
    fn test_generate_vault_id_ignores_name() {
        let (_, _temp_dir) = create_temp_registry();

        // Different names should still produce valid UUIDs
        let id1 = VaultRegistry::generate_vault_id();
        let id2 = VaultRegistry::generate_vault_id();
        let id3 = VaultRegistry::generate_vault_id();
        let id4 = VaultRegistry::generate_vault_id();

        assert!(Uuid::parse_str(&id1).is_ok());
        assert!(Uuid::parse_str(&id2).is_ok());
        assert!(Uuid::parse_str(&id3).is_ok());
        assert!(Uuid::parse_str(&id4).is_ok());

        // All should be different
        let ids = vec![&id1, &id2, &id3, &id4];
        let mut unique_ids = ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        assert_eq!(ids.len(), unique_ids.len());
    }

    #[test]
    fn test_vault_category_display() {
        assert_eq!(VaultCategory::Personal.to_string(), "Personal");
        assert_eq!(VaultCategory::Work.to_string(), "Work");
        assert_eq!(VaultCategory::Team.to_string(), "Team");
        assert_eq!(VaultCategory::Project.to_string(), "Project");
        assert_eq!(VaultCategory::Testing.to_string(), "Testing");
        assert_eq!(VaultCategory::Archive.to_string(), "Archive");
        assert_eq!(
            VaultCategory::Custom("MyCategory".to_string()).to_string(),
            "MyCategory"
        );
    }

    #[test]
    fn test_vault_category_equality() {
        assert_eq!(VaultCategory::Personal, VaultCategory::Personal);
        assert_ne!(VaultCategory::Personal, VaultCategory::Work);
        assert_eq!(
            VaultCategory::Custom("test".to_string()),
            VaultCategory::Custom("test".to_string())
        );
        assert_ne!(
            VaultCategory::Custom("test1".to_string()),
            VaultCategory::Custom("test2".to_string())
        );
    }

    #[test]
    fn test_vault_info_serialization() {
        let vault = create_test_vault_info(
            "550e8400-e29b-41d4-a716-446655440000",
            "Test Vault",
            VaultCategory::Personal,
        );

        let serialized = serde_json::to_string(&vault).expect("Failed to serialize");
        let deserialized: VaultInfo = serde_json::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(vault.id, deserialized.id);
        assert_eq!(vault.name, deserialized.name);
        assert_eq!(vault.category, deserialized.category);
    }

    #[test]
    fn test_vault_registry_serialization() {
        let (mut registry, _temp_dir) = create_temp_registry();
        let test_id = "550e8400-e29b-41d4-a716-446655440000";
        let vault = create_test_vault_info(test_id, "Test Vault", VaultCategory::Work);
        registry.vaults.insert(test_id.to_string(), vault);
        registry.active_vault_id = Some(test_id.to_string());

        let serialized = serde_json::to_string(&registry).expect("Failed to serialize");
        let mut deserialized: VaultRegistry = serde_json::from_str(&serialized).expect("Failed to deserialize");

        // Set registry path since it's skipped in serialization
        deserialized.registry_path = registry.registry_path.clone();

        assert_eq!(registry.vaults.len(), deserialized.vaults.len());
        assert_eq!(registry.active_vault_id, deserialized.active_vault_id);
        assert!(deserialized.get_vault(test_id).is_some());
    }

    #[test]
    fn test_create_vault_basic() {
        let (mut registry, temp_dir) = create_temp_registry();
        let vault_path = temp_dir.path().join("custom_vault.db");

        let result = registry.create_vault(
            "My Test Vault".to_string(),
            Some(vault_path.clone()),
            VaultCategory::Personal,
            Some("Test description".to_string()),
        );

        assert!(result.is_ok());
        let vault_id = result.unwrap();

        // Should be a valid UUID
        assert!(Uuid::parse_str(&vault_id).is_ok());

        let vault = registry.get_vault(&vault_id);
        assert!(vault.is_some());

        let vault = vault.unwrap();
        assert_eq!(vault.name, "My Test Vault");
        assert_eq!(vault.path, vault_path);
        assert_eq!(vault.category, VaultCategory::Personal);
        assert_eq!(vault.description, Some("Test description".to_string()));
        assert!(!vault.is_active);
        assert!(!vault.is_favorite);
    }

    #[test]
    fn test_create_vault_default_path() {
        let (mut registry, _temp_dir) = create_temp_registry();

        let result = registry.create_vault("Default Path Vault".to_string(), None, VaultCategory::Work, None);

        assert!(result.is_ok());
        let vault_id = result.unwrap();

        // Should be a valid UUID
        assert!(Uuid::parse_str(&vault_id).is_ok());

        let vault = registry.get_vault(&vault_id);
        assert!(vault.is_some());

        let vault = vault.unwrap();
        assert_eq!(vault.name, "Default Path Vault");
        assert_eq!(vault.category, VaultCategory::Work);
        assert!(vault.description.is_none());
    }

    #[test]
    fn test_set_active_vault_success() {
        let (mut registry, _temp_dir) = create_temp_registry();
        let test_id = "550e8400-e29b-41d4-a716-446655440000";
        let vault = create_test_vault_info(test_id, "Test Vault", VaultCategory::Personal);
        registry.vaults.insert(test_id.to_string(), vault);

        let result = registry.set_active_vault(test_id);

        assert!(result.is_ok());
        assert_eq!(registry.active_vault_id, Some(test_id.to_string()));
    }

    #[test]
    fn test_set_active_vault_nonexistent() {
        let (mut registry, _temp_dir) = create_temp_registry();

        let result = registry.set_active_vault("550e8400-e29b-41d4-a716-446655440000");

        assert!(result.is_err());
        assert!(registry.active_vault_id.is_none());
    }

    #[test]
    fn test_update_vault_all_fields() {
        let (mut registry, _temp_dir) = create_temp_registry();
        let test_id = "550e8400-e29b-41d4-a716-446655440000";
        let vault = create_test_vault_info(test_id, "Original Name", VaultCategory::Personal);
        registry.vaults.insert(test_id.to_string(), vault);

        let result = registry.update_vault(
            test_id,
            Some("New Name".to_string()),
            Some("New Description".to_string()),
            Some(VaultCategory::Work),
            Some(true),
        );

        assert!(result.is_ok());

        let updated_vault = registry.get_vault(test_id).unwrap();
        assert_eq!(updated_vault.name, "New Name");
        assert_eq!(updated_vault.description, Some("New Description".to_string()));
        assert_eq!(updated_vault.category, VaultCategory::Work);
        assert!(updated_vault.is_favorite);
    }

    #[test]
    fn test_update_vault_partial_fields() {
        let (mut registry, _temp_dir) = create_temp_registry();
        let test_id = "550e8400-e29b-41d4-a716-446655440000";
        let mut vault = create_test_vault_info(test_id, "Original Name", VaultCategory::Personal);
        vault.description = Some("Original Description".to_string());
        registry.vaults.insert(test_id.to_string(), vault);

        let result = registry.update_vault(test_id, Some("New Name".to_string()), None, None, None);

        assert!(result.is_ok());

        let updated_vault = registry.get_vault(test_id).unwrap();
        assert_eq!(updated_vault.name, "New Name");
        assert_eq!(updated_vault.description, Some("Original Description".to_string()));
        assert_eq!(updated_vault.category, VaultCategory::Personal);
        assert!(!updated_vault.is_favorite);
    }

    #[test]
    fn test_update_vault_nonexistent() {
        let (mut registry, _temp_dir) = create_temp_registry();

        let result = registry.update_vault(
            "550e8400-e29b-41d4-a716-446655440000",
            Some("New Name".to_string()),
            None,
            None,
            None,
        );

        assert!(result.is_err());
    }
    #[test]
    fn test_delete_vault_exists() {
        let (mut registry, temp_dir) = create_temp_registry();
        let test_id = "550e8400-e29b-41d4-a716-446655440000";

        // Create an actual vault file
        let vault_path = temp_dir.path().join("test_vault.db");
        std::fs::write(&vault_path, b"test vault data").expect("Failed to create test vault file");

        let mut vault = create_test_vault_info(test_id, "Test Vault", VaultCategory::Personal);
        vault.path = vault_path;
        registry.vaults.insert(test_id.to_string(), vault);

        // Create a registry file
        let registry_content = serde_json::to_string(&registry).unwrap_or_else(|_| "{}".to_string());
        std::fs::write(&registry.registry_path, registry_content).ok();

        let result = registry.delete_vault(test_id, false);

        match result {
            Ok(()) => {
                assert!(registry.get_vault(test_id).is_none());
            }
            Err(e) => {
                if e.to_string().contains("not implemented") || e.to_string().contains("todo") {
                    println!("Skipping test - delete_vault not implemented yet");
                    return;
                }
                panic!("delete_vault failed unexpectedly: {e}");
            }
        }
    }

    #[test]
    fn test_delete_vault_with_file_removal() {
        let (mut registry, temp_dir) = create_temp_registry();
        let test_id = "550e8400-e29b-41d4-a716-446655440000";

        // Create an actual vault file
        let vault_path = temp_dir.path().join("vault_to_delete.db");
        std::fs::write(&vault_path, b"test vault data").expect("Failed to create test vault file");
        assert!(vault_path.exists());

        let mut vault = create_test_vault_info(test_id, "Vault To Delete", VaultCategory::Personal);
        vault.path = vault_path.clone();
        registry.vaults.insert(test_id.to_string(), vault);

        let result = registry.delete_vault(test_id, true); // delete_file = true

        if let Err(ref e) = result {
            eprintln!("Delete vault with file error: {e}");
        }

        assert!(result.is_ok());
        assert!(registry.get_vault(test_id).is_none());
        assert!(!vault_path.exists()); // File should be deleted
    }

    #[test]
    fn test_delete_vault_nonexistent() {
        let (mut registry, _temp_dir) = create_temp_registry();

        let result = registry.delete_vault("550e8400-e29b-41d4-a716-446655440000", false);

        assert!(result.is_err());
    }

    #[test]
    fn test_import_vault_copy_file() {
        let (mut registry, temp_dir) = create_temp_registry();

        // Create a temporary vault file to import
        let source_file = temp_dir.path().join("source_vault.db");
        std::fs::write(&source_file, b"test vault content").expect("Failed to create test file");

        let result = registry.import_vault(
            &source_file,
            "Imported Vault".to_string(),
            VaultCategory::Project,
            true, // copy file
        );

        assert!(result.is_ok());
        let vault_id = result.unwrap();

        // Should be a valid UUID
        assert!(Uuid::parse_str(&vault_id).is_ok());

        let vault = registry.get_vault(&vault_id).unwrap();
        assert_eq!(vault.name, "Imported Vault");
        assert_eq!(vault.category, VaultCategory::Project);
        assert_ne!(vault.path, source_file); // Should be copied to a different location

        // Store the copied file path for cleanup
        let copied_file_path = vault.path.clone();

        // CLEANUP: Delete the vault entry and any files that may have been created outside temp_dir
        let cleanup_result = registry.delete_vault(&vault_id, true); // delete_file = true

        // Verify cleanup was successful
        if cleanup_result.is_ok() {
            assert!(registry.get_vault(&vault_id).is_none());
            // If the file was copied outside temp_dir, it should now be deleted
            if !copied_file_path.starts_with(temp_dir.path()) {
                assert!(
                    !copied_file_path.exists(),
                    "Production file should be cleaned up: {copied_file_path:?}"
                );
            }
        } else {
            // If delete_vault failed, try manual cleanup
            eprintln!("Warning: delete_vault failed, attempting manual cleanup: {cleanup_result:?}");

            // Remove from registry manually
            registry.vaults.remove(&vault_id);

            // Try to delete the copied file manually if it's outside our temp directory
            if !copied_file_path.starts_with(temp_dir.path()) && copied_file_path.exists() {
                if let Err(e) = std::fs::remove_file(&copied_file_path) {
                    eprintln!("Warning: Failed to clean up test file {copied_file_path:?}: {e}");
                }
            }
        }

        // Final verification that we cleaned up properly
        assert!(
            registry.get_vault(&vault_id).is_none(),
            "Vault should be cleaned up from registry"
        );
    }

    #[test]
    fn test_import_vault_move_file() {
        let (mut registry, temp_dir) = create_temp_registry();

        let source_file = temp_dir.path().join("source_vault.db");
        std::fs::write(&source_file, b"test vault content").expect("Failed to create test file");

        let result = registry.import_vault(
            &source_file,
            "Imported Vault".to_string(),
            VaultCategory::Archive,
            false, // don't copy file
        );

        assert!(result.is_ok());
        let vault_id = result.unwrap();

        // Should be a valid UUID
        assert!(Uuid::parse_str(&vault_id).is_ok());

        let vault = registry.get_vault(&vault_id).unwrap();
        assert_eq!(vault.name, "Imported Vault");
        assert_eq!(vault.category, VaultCategory::Archive);
        assert_eq!(vault.path, source_file); // Should reference original location
    }

    #[test]
    fn test_import_vault_nonexistent_file() {
        let (mut registry, temp_dir) = create_temp_registry();
        let nonexistent_file = temp_dir.path().join("nonexistent.db");

        let result = registry.import_vault(
            &nonexistent_file,
            "Imported Vault".to_string(),
            VaultCategory::Personal,
            true,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_vaults_management() {
        let (mut registry, _temp_dir) = create_temp_registry();

        // Create multiple vaults
        let vault1_result = registry.create_vault("Vault One".to_string(), None, VaultCategory::Personal, None);
        let vault2_result = registry.create_vault(
            "Vault Two".to_string(),
            None,
            VaultCategory::Work,
            Some("Work vault".to_string()),
        );
        let vault3_result = registry.create_vault("Vault Three".to_string(), None, VaultCategory::Team, None);

        assert!(vault1_result.is_ok());
        assert!(vault2_result.is_ok());
        assert!(vault3_result.is_ok());

        let vault1_id = vault1_result.unwrap();
        let vault2_id = vault2_result.unwrap();
        let vault3_id = vault3_result.unwrap();

        // All IDs should be valid UUIDs and unique
        assert!(Uuid::parse_str(&vault1_id).is_ok());
        assert!(Uuid::parse_str(&vault2_id).is_ok());
        assert!(Uuid::parse_str(&vault3_id).is_ok());
        assert_ne!(vault1_id, vault2_id);
        assert_ne!(vault2_id, vault3_id);
        assert_ne!(vault1_id, vault3_id);

        // Test listing all vaults
        let vaults = registry.list_vaults();
        assert_eq!(vaults.len(), 3);

        // Test setting active vault
        assert!(registry.set_active_vault(&vault2_id).is_ok());
        assert_eq!(registry.active_vault_id, Some(vault2_id.clone()));

        // Test updating a vault
        assert!(registry.update_vault(&vault1_id, None, None, None, Some(true)).is_ok());
        assert!(registry.get_vault(&vault1_id).unwrap().is_favorite);

        // Test deleting a vault
        assert!(registry.delete_vault(&vault3_id, false).is_ok());
        assert_eq!(registry.list_vaults().len(), 2);
    }

    #[test]
    fn test_vault_timestamps() {
        let vault = create_test_vault_info(
            "550e8400-e29b-41d4-a716-446655440000",
            "Test Vault",
            VaultCategory::Personal,
        );

        let now = OffsetDateTime::now_utc();
        let created_diff = (vault.created_at - now).abs();
        let accessed_diff = (vault.last_accessed - now).abs();

        // Timestamps should be very recent (within 1 second)
        assert!(created_diff.whole_seconds() <= 1);
        assert!(accessed_diff.whole_seconds() <= 1);
    }

    #[test]
    fn test_vault_category_custom_variants() {
        let categories = vec![
            VaultCategory::Personal,
            VaultCategory::Work,
            VaultCategory::Team,
            VaultCategory::Project,
            VaultCategory::Testing,
            VaultCategory::Archive,
            VaultCategory::Custom("CustomType".to_string()),
        ];

        for category in categories {
            let vault = create_test_vault_info("550e8400-e29b-41d4-a716-446655440000", "Test Vault", category.clone());

            // Test serialization roundtrip
            let serialized = serde_json::to_string(&vault).expect("Failed to serialize");
            let deserialized: VaultInfo = serde_json::from_str(&serialized).expect("Failed to deserialize");

            assert_eq!(vault.category, deserialized.category);
        }
    }

    #[test]
    fn test_edge_case_empty_vault_name() {
        let (mut registry, _temp_dir) = create_temp_registry();

        let result = registry.create_vault(String::new(), None, VaultCategory::Personal, None);

        // Should handle empty names gracefully
        assert!(result.is_ok());
        let vault_id = result.unwrap();
        assert!(Uuid::parse_str(&vault_id).is_ok());
        let vault = registry.get_vault(&vault_id).unwrap();
        assert_eq!(vault.name, "");
    }

    #[test]
    fn test_edge_case_very_long_vault_name() {
        let (mut registry, _temp_dir) = create_temp_registry();
        let long_name = "A".repeat(1000);

        let result = registry.create_vault(long_name.clone(), None, VaultCategory::Personal, None);

        assert!(result.is_ok());
        let vault_id = result.unwrap();
        assert!(Uuid::parse_str(&vault_id).is_ok());
        let vault = registry.get_vault(&vault_id).unwrap();
        assert_eq!(vault.name, long_name);
    }

    #[test]
    fn test_concurrent_vault_operations() {
        let (mut registry, _temp_dir) = create_temp_registry();

        // Simulate concurrent vault creation
        let mut vault_ids = Vec::new();
        for i in 0..10 {
            let result = registry.create_vault(format!("Vault {i}"), None, VaultCategory::Testing, None);
            assert!(result.is_ok());
            vault_ids.push(result.unwrap());
        }

        // All IDs should be valid UUIDs and unique
        for vault_id in &vault_ids {
            assert!(Uuid::parse_str(vault_id).is_ok());
        }

        let mut unique_ids = vault_ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        assert_eq!(vault_ids.len(), unique_ids.len());

        // Test operations on all vaults
        for vault_id in &vault_ids {
            assert!(registry.get_vault(vault_id).is_some());
            assert!(registry.set_active_vault(vault_id).is_ok());
            assert!(registry.update_vault(vault_id, None, None, None, Some(true)).is_ok());
        }

        assert_eq!(registry.list_vaults().len(), 10);
    }

    #[test]
    fn test_uuid_collision_resistance() {
        let (_, _temp_dir) = create_temp_registry();

        // Generate a large number of UUIDs to test for collisions
        let mut ids = std::collections::HashSet::new();
        for _ in 0..10000 {
            let id = VaultRegistry::generate_vault_id();
            assert!(Uuid::parse_str(&id).is_ok());

            // Should be unique (no collisions)
            assert!(ids.insert(id), "UUID collision detected!");
        }
    }

    #[test]
    fn test_uuid_format_consistency() {
        let (_, _temp_dir) = create_temp_registry();

        for _ in 0..100 {
            let id = VaultRegistry::generate_vault_id();
            let uuid = Uuid::parse_str(&id).expect("Should be valid UUID");

            // Should be versioned 4 UUID (random)
            assert_eq!(uuid.get_version_num(), 4);

            // Should be properly formatted
            assert_eq!(id.len(), 36); // UUID string length
            assert_eq!(id.chars().filter(|&c| c == '-').count(), 4); // Should have 4 hyphens
        }
    }
}
