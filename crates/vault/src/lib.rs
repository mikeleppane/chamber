pub mod config;
pub mod crypto;
pub mod db;

// Re-export commonly used types and functions for easier access
pub use crypto::{
    aead_decrypt, aead_encrypt, derive_key, unwrap_vault_key, wrap_vault_key, HmacSha256, KdfParams, KeyMaterial,
    WrappedVaultKey,
};

pub use db::{Db, ItemRow};

pub use crate::config::BackupConfig;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ItemKind {
    Password,
    EnvVar,
    Note,
    ApiKey,
    SshKey,
    Certificate,
    Database,
}

impl std::str::FromStr for ItemKind {
    type Err = std::convert::Infallible; // Since your current implementation never fails

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result = match s.to_lowercase().as_str() {
            "password" | "pass" | "pwd" => ItemKind::Password,
            "env" | "envvar" | "environment" => ItemKind::EnvVar,
            "apikey" | "api_key" | "api-key" | "token" => ItemKind::ApiKey,
            "sshkey" | "ssh_key" | "ssh-key" | "ssh" => ItemKind::SshKey,
            "certificate" | "cert" | "ssl" | "tls" => ItemKind::Certificate,
            "database" | "db" | "connection" => ItemKind::Database,
            _ => ItemKind::Note,
        };
        Ok(result)
    }
}

impl ItemKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ItemKind::Password => "password",
            ItemKind::EnvVar => "env",
            ItemKind::Note => "note",
            ItemKind::ApiKey => "apikey",
            ItemKind::SshKey => "sshkey",
            ItemKind::Certificate => "certificate",
            ItemKind::Database => "database",
        }
    }

    #[must_use]
    pub const fn all() -> &'static [ItemKind] {
        &[
            ItemKind::Password,
            ItemKind::EnvVar,
            ItemKind::Note,
            ItemKind::ApiKey,
            ItemKind::SshKey,
            ItemKind::Certificate,
            ItemKind::Database,
        ]
    }

    /// Returns a user-friendly display name
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            ItemKind::Password => "Password",
            ItemKind::EnvVar => "Environment Variable",
            ItemKind::Note => "Note",
            ItemKind::ApiKey => "API Key",
            ItemKind::SshKey => "SSH Key",
            ItemKind::Certificate => "Certificate",
            ItemKind::Database => "Database Connection",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: i64,
    pub name: String,
    pub kind: ItemKind,
    pub value: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct NewItem {
    pub name: String,
    pub kind: ItemKind,
    pub value: String,
}

pub struct Vault {
    db: Db,
    key: Option<KeyMaterial>,
    db_path: PathBuf,
}

impl Vault {
    /// Open an existing database or create a new one if it doesn't exist.
    ///
    /// This function initializes a database instance from a given file system path.
    /// If a path is provided, it attempts to use that specific path.
    /// If no path is provided (`None` is passed), it will determine a default database path
    /// using the `default_db_path` function.
    ///
    /// # Parameters
    /// * `path` - An optional reference to a path (`Option<&Path>`). If `Some`, the database
    ///   will be opened or created at the specified path. If `None`, a default path is used.
    ///
    /// # Returns
    /// * `Result<Self>` - On success, returns an instance of `Self` with an open database.
    ///   If any operation fails (e.g., obtaining the default path or opening the database),
    ///   an error is returned.
    ///
    /// # Errors
    /// This function returns an error if:
    /// * Obtaining the default database path via `default_db_path` fails.
    /// * Opening the database at the specified or default path fails.
    pub fn open_or_create(path: Option<&Path>) -> Result<Self> {
        let db_path = match path {
            Some(p) => p.to_path_buf(),
            None => default_db_path()?,
        };
        let db = Db::open(&db_path)?;
        Ok(Self { db, key: None, db_path })
    }

    /// Opens the default instance of a resource.
    ///
    /// This function attempts to open the default instance of a resource. If the default instance
    /// does not exist, it will create a new one. The specific behavior of this function depends
    /// on the implementation of `Self::open_or_create`.
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - If successful, returns an instance of the resource wrapped in `Ok`.
    ///   If an error occurs during the opening or creation process, it returns an `Err` containing
    ///   the corresponding error.
    ///
    /// # Errors
    ///
    /// This function will return an error if the resource cannot be opened or created for any
    /// reason, such as insufficient permissions or missing configuration.
    pub fn open_default() -> Result<Self> {
        Self::open_or_create(None)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Retrieves the backup configuration for the system.
    ///
    /// This function attempts to load the backup configuration from a JSON file
    /// stored in the directory containing the database. If the configuration file
    /// does not exist, a default `BackupConfig` instance is returned.
    ///
    /// # Behavior
    ///
    /// - The configuration file is named `backup_config.json`, and its location
    ///   is inferred based on the parent directory of the `db_path` provided.
    /// - If the parent directory cannot be determined, the current directory (`"."`)
    ///   is used as the fallback location to look for the configuration file.
    /// - The function reads the contents of the file and deserializes it into a
    ///   `BackupConfig` object using `serde_json`.
    /// - If the file does not exist, the function returns the default
    ///   `BackupConfig` object.
    ///
    /// # Errors
    ///
    /// This function may return an error in the following scenarios:
    /// - If the configuration file exists, but reading its contents fails,
    ///   an `std::io::Error` is returned.
    /// - If deserialization of the file content into a `BackupConfig` object fails,
    ///   a `serde_json::Error` is returned.
    ///
    /// # Returns
    ///
    /// A `Result` wrapping the backup configuration:
    /// - `Ok(BackupConfig)` if the configuration is successfully loaded or the
    ///   default configuration is returned.
    /// - `Err` if an error occurs while reading or deserializing the configuration file.
    pub fn get_backup_config(&self) -> Result<BackupConfig> {
        // Try to read backup config from the database
        // For now, we'll store it as a JSON string in a special meta-table or file
        let config_path = self
            .db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backup_config.json");

        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: BackupConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(BackupConfig::default())
        }
    }

    /// Sets the backup configuration for the database by saving it to a file named `backup_config.json`
    /// in the parent directory of the database path.
    ///
    /// The method performs the following steps:
    /// 1. Resolves the `backup_config.json` file path in the parent directory of `self.db_path`.
    /// 2. Ensures the parent directory for the file exists, creating it if necessary.
    /// 3. Serializes the `BackupConfig` object to a pretty-printed JSON string.
    /// 4. Writes the serialized JSON string to the file.
    ///
    /// # Arguments
    /// * `config` - A reference to a `BackupConfig` object containing the backup configuration settings
    ///   that will be saved to the file.
    ///
    /// # Returns
    /// * `Ok(())` if the backup configuration is successfully saved.
    /// * `Err` if any file system or serialization operation fails.
    ///
    /// # Errors
    /// This function can return an error in the following cases:
    /// * Failure to create the parent directory for the config file.
    /// * Failure to serialize the `BackupConfig` object into a JSON string.
    /// * Failure to write the serialized JSON string to the file.
    pub fn set_backup_config(&self, config: &BackupConfig) -> Result<()> {
        let config_path = self
            .db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backup_config.json");

        // Ensure directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Get the database path for this vault
    pub fn get_db_path(&self) -> &std::path::Path {
        &self.db_path
    }

    pub fn is_initialized(&self) -> bool {
        !self.db.is_meta_empty().unwrap_or(false)
    }

    /// Initializes the storage system with a master key.
    ///
    /// This function is used to initialize the storage backend only if it has not been
    /// initialized already. It generates a vault key that is securely wrapped using the
    /// derived master key, and then stores the necessary metadata in the database.
    ///
    /// # Parameters
    /// - `master`: A reference to a string slice representing the master key. This key is
    ///   used as the input to derive a secure key for wrapping the vault key.
    ///
    /// # Returns
    /// - `Ok(())`: If the storage is successfully initialized or was already initialized.
    /// - `Err`: If any step in the initialization process fails (e.g., key derivation,
    ///   key wrapping, or database write).
    ///
    /// # Implementation Details
    /// - First, the function checks whether the storage system has already been initialized
    ///   using the `is_initialized()` method. If true, it immediately returns `Ok(())`.
    /// - The function then generates secure key derivation parameters (`KdfParams`) using
    ///   `KdfParams::default_secure()`.
    /// - Using the provided master key and the key derivation parameters, a derived key
    ///   is computed (`derive_key` function).
    /// - A new random vault key is generated (`KeyMaterial::random()`), which serves as the
    ///   key to encrypt secure data.
    /// - The vault key is securely wrapped using the derived master key, resulting in the
    ///   wrapped key and a verifier (`wrap_vault_key` function).
    /// - The derived key, wrapped vault key, and verifier are then persisted into the
    ///   database by calling `db.write_meta`.
    ///
    /// # Errors
    /// - Fails with an error if:
    ///   - The master key derivation fails.
    ///   - The vault key wrapping fails.
    ///   - Writing the required metadata to the database fails.
    /// - Errors are returned as a `Result::Err`, allowing the caller to handle the failure.
    pub fn initialize(&mut self, master: &str) -> Result<()> {
        if self.is_initialized() {
            return Ok(());
        }
        let kdf = KdfParams::default_secure();
        let master_derived = derive_key(master, &kdf)?;
        let vault_key = KeyMaterial::random();
        let (wrapped, verifier) = wrap_vault_key(&master_derived, &vault_key)?;
        self.db.write_meta(&kdf, &wrapped, &verifier)?;
        Ok(())
    }

    /// Unlocks the vault using the provided master key.
    ///
    /// This function attempts to unlock a vault instance by using the given `master` key.
    /// It reads the metadata from the vault, validates the provided master key against the verifier,
    /// and upon successful verification, derives and stores the corresponding vault key.
    ///
    /// # Arguments
    ///
    /// * `master` - A string slice representing the master key used to unlock the vault.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the vault is successfully unlocked and the vault key is derived.
    /// * `Err` - If the vault metadata is uninitialized, the derived master key is invalid,
    ///   or there is an issue during the unlocking process.
    ///
    /// # Errors
    ///
    /// This function can return the following errors wrapped in a `Result`:
    /// * `anyhow!("Vault not initialized")` - If the vault metadata is not present.
    /// * `anyhow!("Invalid master key")` - If the provided master key is invalid or fails verification.
    /// * Other errors arising from key derivation or vault key unwrapping.
    ///
    /// # Implementation Details
    ///
    /// 1. Reads metadata from the vault, including a key derivation function (KDF) configuration (`kdf`),
    ///    a wrapped vault key (`wrapped`), and a verifier.
    /// 2. Derives a key from the provided `master` key using the KDF.
    /// 3. Verifies the derived key against the verifier. If the verification fails, returns an error.
    /// 4. Attempts to unwrap the vault key using the derived key. Upon success, stores the derived
    ///    vault key (`vk`) within the instance's `key` field.
    ///
    pub fn unlock(&mut self, master: &str) -> Result<()> {
        let (kdf, wrapped, verifier) = self.db.read_meta()?.ok_or_else(|| anyhow!("Vault not initialized"))?;
        let master_derived = derive_key(master, &kdf)?;
        // Verify first
        unwrap_vault_key(&master_derived, &wrapped, Some(&verifier)).map_err(|_| anyhow!("Invalid master key"))?;
        let vk = unwrap_vault_key(&master_derived, &wrapped, None)?;
        self.key = Some(vk);
        Ok(())
    }

    pub const fn is_unlocked(&self) -> bool {
        self.key.is_some()
    }

    /// Retrieves a list of items from the database, decrypting their stored values.
    ///
    /// # Returns
    /// - `Ok(Vec<Item>)`: A vector of decrypted items if the operation is successful.
    /// - `Err(anyhow::Error)`: An error if the database is locked, decryption fails, or another issue occurs.
    ///
    /// # Implementation Details
    /// 1. The function attempts to retrieve the encryption key (`vk`) from the `self.key` field.
    ///    If the key is unavailable, an error is returned with the message `"Locked"`.
    /// 2. The database records are fetched using `self.db.list_items()`.
    /// 3. For each record:
    ///    - The ciphertext is decrypted using `aead_decrypt` with the provided nonce, ciphertext, and additional authentication data (AAD).
    ///    - The decrypted plaintext is converted to a UTF-8 string to derive the item's `value`.
    ///    - An `Item` is constructed with the decrypted and existing meta-information, such as `id`, `name`, `kind`, and timestamps.
    /// 4. The constructed items are aggregated into a `Vec<Item>` and returned.
    ///
    /// # Errors
    /// - Returns an error if:
    ///   - The encryption key is not available (e.g., when the data store is locked).
    ///   - The database query fails.
    ///   - The decryption operation fails (e.g., due to invalid data).
    ///   - The plaintext fails UTF-8 validation.
    ///   - An invalid `ItemKind` is provided.
    ///
    /// # Dependencies
    /// - `aead_decrypt`: A function used to decrypt the encrypted data.
    /// - `String::from_utf8`: Used to convert decrypted data into a String.
    /// - `ItemKind::from_str`: Parses the `kind` field into its corresponding `ItemKind` enum variant.
    ///
    /// # Notes
    /// - Ensure `self.key` is properly initialized before calling this method.
    /// - The database schema must provide the required fields for each item: `id`, `name`, `kind`, `ciphertext`, `nonce`, and timestamps.
    pub fn list_items(&self) -> Result<Vec<Item>> {
        let vk = self.key.as_ref().ok_or_else(|| anyhow!("Locked"))?;
        let rows = self.db.list_items()?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let plaintext = aead_decrypt(vk, &r.nonce, &r.ciphertext, &r.ad())?;
            let value = String::from_utf8(plaintext)?;
            out.push(Item {
                id: r.id,
                name: r.name,
                kind: ItemKind::from_str(&r.kind)?,
                value,
                created_at: r.created_at,
                updated_at: r.updated_at,
            });
        }
        Ok(out)
    }

    /// Retrieves an item by its name from the list of items.
    ///
    /// This method searches for an item in the collection of items maintained by the instance.
    /// It returns the first item that matches the provided name, if it exists.
    ///
    /// # Parameters
    /// - `name`: The name of the item to search for, provided as a string slice (`&str`).
    ///
    /// # Returns
    /// - `Ok(Some(Item))`: If an item with the given name is found.
    /// - `Ok(None)`: If no item with the given name exists in the collection.
    /// - `Err(Error)`: If an error occurs while retrieving the list of items.
    ///
    /// # Errors
    /// This function will return an error if the call to `self.list_items()` fails.
    pub fn get_item_by_name(&self, name: &str) -> Result<Option<Item>> {
        let items = self.list_items()?;
        Ok(items.into_iter().find(|i| i.name == name))
    }

    /// Creates a new item and inserts it into the database.
    ///
    /// This function encrypts the value of the provided item using an AEAD encryption
    /// scheme and then stores the encrypted data, along with additional metadata, in
    /// the database. The encryption process uses the key stored within the struct and
    /// associates the encrypted value with the provided item's name and kind.
    ///
    /// # Parameters
    /// - `item`: A reference to a `NewItem`, containing the data required to create the item.
    ///
    /// # Returns
    /// - `Ok(())` on successful encryption and insertion of the item.
    /// - `Err(anyhow::Error)` if any step of the process fails, including:
    ///   - The key is not available (when the struct is in a "Locked" state).
    ///   - Failure during the encryption process.
    ///   - Errors encountered during the database insertion.
    ///
    /// # Errors
    /// This function returns an error in the following scenarios:
    /// - If the struct's `key` field is `None`, indicating it is locked.
    /// - If encryption fails for any reason.
    /// - If the database insertion fails.
    pub fn create_item(&mut self, item: &NewItem) -> Result<()> {
        let vk = self.key.as_ref().ok_or_else(|| anyhow!("Locked"))?;
        let nonce_cipher = aead_encrypt(
            vk,
            item.value.as_bytes(),
            ItemRow::ad_for_name_kind(&item.name, item.kind.as_str()).as_ref(),
        )?;
        self.db
            .insert_item(&item.name, item.kind.as_str(), &nonce_cipher.0, &nonce_cipher.1)?;
        Ok(())
    }

    /// Deletes an item from the database with the specified ID.
    ///
    /// # Parameters
    /// - `id` (i64): The unique identifier of the item to be deleted.
    ///
    /// # Returns
    /// - `Result<()>`: Returns `Ok(())` if the item is successfully deleted, or an error if the deletion fails.
    ///
    /// # Errors
    /// This function will return an error if:
    /// - The item with the specified ID does not exist.
    /// - There is a failure in the underlying database operation.
    pub fn delete_item(&mut self, id: i64) -> Result<()> {
        self.db.delete_item(id)
    }

    /// Changes the master key for the vault.
    ///
    /// This function updates the master key used to protect the vault by verifying the current
    /// master key and re-encrypting the vault key using the new master key. It also updates
    /// the key derivation function (KDF) parameters for the new master key and persists the
    /// updated metadata in the database.
    ///
    /// # Arguments
    ///
    /// * `current_master` - The currently active master key used to protect the vault. This must
    ///   be provided to verify the existing setup and decrypt the vault key.
    /// * `new_master` - The new master key to which the vault will be re-encrypted. It must conform
    ///   to the same security requirements as the old master key.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Returns `Ok(())` on successful master key change, or an error if the operation
    ///   fails. Possible error cases include:
    ///   - The vault is not initialized.
    ///   - The provided `current_master` key is invalid.
    ///   - Issues with deriving keys, unwrapping the vault key, or re-wrapping with the new key.
    ///   - Errors while persisting the updated metadata.
    ///
    /// # Behavior
    ///
    /// 1. Reads the current metadata (KDF parameters, wrapped key, and verifier) from the database.
    /// 2. Validates the `current_master` key using the stored KDF parameters and verifier.
    /// 3. Unwraps the vault key using the `current_master` key.
    /// 4. Generates new secure KDF parameters for the `new_master` key.
    /// 5. Derives a key from the `new_master` and wraps the vault key with it, generating a new verifier.
    /// 6. Writes the new KDF parameters, wrapped key, and verifier to the metadata in the database.
    /// 7. Updates the in-memory vault key if it is already unlocked.
    ///
    /// # Errors
    ///
    /// * Returns an error if the vault has not been initialized (e.g., no metadata exists yet).
    /// * Returns an error if the `current_master` key fails verification or cannot unwrap the vault key.
    /// * Returns an error for any issues in key operations (e.g., deriving, wrapping, or unwrapping).
    /// * Returns an error if writing the updated metadata to the database fails.
    ///
    /// # Notes
    ///
    /// The updated master key takes effect immediately upon successful execution of this function.
    /// Ensure that the `new_master` key is securely stored and managed to prevent loss of access
    /// to the vault.
    pub fn change_master_key(&mut self, current_master: &str, new_master: &str) -> Result<()> {
        let (kdf_old, wrapped_old, verifier_old) =
            self.db.read_meta()?.ok_or_else(|| anyhow!("Vault not initialized"))?;

        // Verify the current master and unwrap the existing vault key
        let current_derived = derive_key(current_master, &kdf_old)?;
        let _ = unwrap_vault_key(&current_derived, &wrapped_old, Some(&verifier_old))
            .map_err(|_| anyhow!("Invalid current master key"))?;
        let vault_key = unwrap_vault_key(&current_derived, &wrapped_old, None)?;

        // Generate fresh KDF params and wrap with a new master-derived key
        let kdf_new = KdfParams::default_secure();
        let new_derived = derive_key(new_master, &kdf_new)?;
        let (wrapped_new, verifier_new) = wrap_vault_key(&new_derived, &vault_key)?;

        // Persist new meta
        self.db.write_meta(&kdf_new, &wrapped_new, &verifier_new)?;

        // Keep the in-memory vault key usable if we were unlocked
        self.key = Some(vault_key);
        Ok(())
    }

    /// Updates an item in the database with a new value, preserving the item's associated metadata.
    ///
    /// # Parameters
    /// - `id`: The unique identifier of the item to be updated.
    /// - `new_value`: A reference to the new string value that will replace the current value of the item.
    ///
    /// # Returns
    /// - `Ok(())`: If the operation is successful.
    /// - `Err`: Returns an error in the following scenarios:
    ///   - If the encryption key (`self.key`) is not available (locked).
    ///   - If the specified item with the given `id` is not found in the list.
    ///   - If there's any failure during the encryption or database update.
    ///
    /// # Behavior
    /// 1. Verifies that the encryption key (`self.key`) is available. If not, an error is returned.
    /// 2. Retrieves the list of items stored and searches for the item matching the provided `id`.
    /// 3. If the item is found, constructs authenticated data (AD) using the item's metadata (name and kind).
    /// 4. Encrypts the `new_value` using the encryption key (`vk`), the new value, and the constructed AD.
    /// 5. Updates the item in the database with the newly encrypted value and its corresponding nonce.
    ///
    /// # Errors
    /// - If the encryption key is missing, an `anyhow!("Locked")` error is returned.
    /// - If the item is not found, an `anyhow!("Item not found")` error is returned.
    /// - Any failures during encryption or database operations propagate as errors.
    pub fn update_item(&mut self, id: i64, new_value: &str) -> Result<()> {
        let vk = self.key.as_ref().ok_or_else(|| anyhow!("Locked"))?;

        // Get the item to preserve name and kind for AD
        let items = self.list_items()?;
        let item = items
            .iter()
            .find(|i| i.id == id)
            .ok_or_else(|| anyhow!("Item not found"))?;

        // Encrypt new value with same AD (name and kind)
        let nonce_cipher = aead_encrypt(
            vk,
            new_value.as_bytes(),
            ItemRow::ad_for_name_kind(&item.name, item.kind.as_str()).as_ref(),
        )?;

        self.db.update_item(id, &nonce_cipher.0, &nonce_cipher.1)?;
        Ok(())
    }
}

// Add Clone implementation for Vault if it doesn't exist
impl Clone for Vault {
    #[allow(clippy::expect_used)]
    fn clone(&self) -> Self {
        // Note: This creates a new database connection
        // The key material is not cloned for security reasons
        Self {
            db: Db::open(&self.db_path).expect("Failed to open database connection"),
            key: None, // Don't clone the key for security
            db_path: self.db_path.clone(),
        }
    }
}

fn default_db_path() -> Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("No config dir"))?;
    let dir = base.join("chamber");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("vault.sqlite3"))
}

// Rust
#[cfg(test)]
mod lib_module_tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::unwrap_in_result)]
    #![allow(clippy::panic)]
    #![allow(clippy::panic_in_result_fn)]
    #![allow(clippy::expect_used)]
    use super::*;
    use std::fs;
    use std::str::FromStr;

    fn tmp_db(name: &str) -> PathBuf {
        let now = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("chamber_vault_lib_{name}_{pid}_{now}.sqlite3"))
    }

    #[test]
    fn test_itemkind_as_str_and_from_str() {
        // Round-trip known variants
        for (kind, s) in [
            (ItemKind::Password, "password"),
            (ItemKind::EnvVar, "env"),
            (ItemKind::Note, "note"),
            (ItemKind::ApiKey, "apikey"),
            (ItemKind::SshKey, "sshkey"),
            (ItemKind::Certificate, "certificate"),
            (ItemKind::Database, "database"),
        ] {
            assert_eq!(kind.as_str(), s);
            // Fuzzy forms should still parse
            assert_eq!(ItemKind::from_str(s), Ok(kind));
            assert_eq!(ItemKind::from_str(&s.to_uppercase()), Ok(kind));
        }

        // Aliases
        assert_eq!(ItemKind::from_str("pass"), Ok(ItemKind::Password));
        assert_eq!(ItemKind::from_str("pwd"), Ok(ItemKind::Password));
        assert_eq!(ItemKind::from_str("envvar"), Ok(ItemKind::EnvVar));
        assert_eq!(ItemKind::from_str("environment"), Ok(ItemKind::EnvVar));
        assert_eq!(ItemKind::from_str("api_key"), Ok(ItemKind::ApiKey));
        assert_eq!(ItemKind::from_str("api-key"), Ok(ItemKind::ApiKey));
        assert_eq!(ItemKind::from_str("token"), Ok(ItemKind::ApiKey));
        assert_eq!(ItemKind::from_str("ssh_key"), Ok(ItemKind::SshKey));
        assert_eq!(ItemKind::from_str("ssh-key"), Ok(ItemKind::SshKey));
        assert_eq!(ItemKind::from_str("ssh"), Ok(ItemKind::SshKey));
        assert_eq!(ItemKind::from_str("cert"), Ok(ItemKind::Certificate));
        assert_eq!(ItemKind::from_str("ssl"), Ok(ItemKind::Certificate));
        assert_eq!(ItemKind::from_str("tls"), Ok(ItemKind::Certificate));
        assert_eq!(ItemKind::from_str("db"), Ok(ItemKind::Database));
        assert_eq!(ItemKind::from_str("connection"), Ok(ItemKind::Database));

        // Unknown -> Note
        assert_eq!(ItemKind::from_str("something-else"), Ok(ItemKind::Note));
    }

    #[test]
    fn test_itemkind_all_and_display_names() {
        let all = ItemKind::all();
        // Ensure all variants present
        assert!(all.contains(&ItemKind::Password));
        assert!(all.contains(&ItemKind::EnvVar));
        assert!(all.contains(&ItemKind::Note));
        assert!(all.contains(&ItemKind::ApiKey));
        assert!(all.contains(&ItemKind::SshKey));
        assert!(all.contains(&ItemKind::Certificate));
        assert!(all.contains(&ItemKind::Database));

        // Display names are human-friendly and stable
        assert_eq!(ItemKind::Password.display_name(), "Password");
        assert_eq!(ItemKind::EnvVar.display_name(), "Environment Variable");
        assert_eq!(ItemKind::Note.display_name(), "Note");
        assert_eq!(ItemKind::ApiKey.display_name(), "API Key");
        assert_eq!(ItemKind::SshKey.display_name(), "SSH Key");
        assert_eq!(ItemKind::Certificate.display_name(), "Certificate");
        assert_eq!(ItemKind::Database.display_name(), "Database Connection");
    }

    #[test]
    fn test_vault_initialize_and_is_initialized() -> Result<()> {
        let path = tmp_db("init");
        let mut v = Vault::open_or_create(Some(&path))?;
        assert!(!v.is_initialized());

        v.initialize("master-1")?;
        assert!(v.is_initialized());

        // Initialize again should be no-op and not fail
        v.initialize("master-1")?;
        assert!(v.is_initialized());

        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_vault_unlock_success_and_failure_paths() -> Result<()> {
        let path = tmp_db("unlock");
        let mut v = Vault::open_or_create(Some(&path))?;
        v.initialize("secret")?;

        // Good master unlocks
        v.unlock("secret")?;

        // Wrong master returns error
        let mut v2 = Vault::open_or_create(Some(&path))?;
        let err = v2.unlock("wrong").unwrap_err().to_string();
        assert!(!err.is_empty());

        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_vault_create_list_get_update_delete() -> Result<()> {
        let path = tmp_db("crud");
        let mut v = Vault::open_or_create(Some(&path))?;
        v.initialize("m")?;
        v.unlock("m")?;

        // Initially empty
        let items = v.list_items()?;
        assert!(items.is_empty());

        // Create a few items
        v.create_item(&NewItem {
            name: "alpha".into(),
            kind: ItemKind::Password,
            value: "A1".into(),
        })?;
        v.create_item(&NewItem {
            name: "beta".into(),
            kind: ItemKind::EnvVar,
            value: "B2".into(),
        })?;

        // List sorts by name
        let items = v.list_items()?;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "alpha");
        assert_eq!(items[1].name, "beta");

        // Get by name
        let got = v.get_item_by_name("alpha")?.expect("exists");
        assert_eq!(got.value, "A1");
        assert_eq!(got.kind, ItemKind::Password);

        // Update item value
        v.update_item(got.id, "A1-updated")?;
        let got2 = v.get_item_by_name("alpha")?.expect("exists");
        assert_eq!(got2.value, "A1-updated");
        assert!(got2.updated_at >= got.updated_at);

        // Delete beta
        let beta = v.get_item_by_name("beta")?.expect("exists");
        v.delete_item(beta.id)?;
        let items_after = v.list_items()?;
        assert_eq!(items_after.len(), 1);
        assert_eq!(items_after[0].name, "alpha");

        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_vault_persistence_across_reopen() -> Result<()> {
        let path = tmp_db("persist");
        {
            let mut v = Vault::open_or_create(Some(&path))?;
            v.initialize("k")?;
            v.unlock("k")?;
            v.create_item(&NewItem {
                name: "one".into(),
                kind: ItemKind::Note,
                value: "first".into(),
            })?;
        }

        // Reopen and unlock, item should be there
        {
            let mut v = Vault::open_or_create(Some(&path))?;
            v.unlock("k")?;
            let items = v.list_items()?;
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].name, "one");
            assert_eq!(items[0].value, "first");
        }

        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_change_master_key_preserves_items_and_allows_new_unlock() -> Result<()> {
        let path = tmp_db("change_key");

        // Initialize, add items
        let mut v = Vault::open_or_create(Some(&path))?;
        v.initialize("old-master")?;
        v.unlock("old-master")?;
        v.create_item(&NewItem {
            name: "svc".into(),
            kind: ItemKind::ApiKey,
            value: "token-123".into(),
        })?;

        // Change master key
        v.change_master_key("old-master", "new-master")?;

        // Old master should no longer unlock; new one should
        let mut v2 = Vault::open_or_create(Some(&path))?;
        let err = v2.unlock("old-master").unwrap_err().to_string();
        assert!(!err.is_empty());

        v2.unlock("new-master")?;
        let items = v2.list_items()?;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "svc");
        assert_eq!(items[0].value, "token-123");

        fs::remove_file(path).ok();
        Ok(())
    }

    #[test]
    fn test_get_item_by_name_not_found() -> Result<()> {
        let path = tmp_db("get_missing");
        let mut v = Vault::open_or_create(Some(&path))?;
        v.initialize("m")?;
        v.unlock("m")?;
        v.create_item(&NewItem {
            name: "exists".into(),
            kind: ItemKind::Note,
            value: "v".into(),
        })?;

        assert!(v.get_item_by_name("nope")?.is_none());

        fs::remove_file(path).ok();
        Ok(())
    }
}
