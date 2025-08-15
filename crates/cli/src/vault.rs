use chamber_vault::{VaultCategory, VaultManager};
use clap::Subcommand;
use color_eyre::eyre::eyre;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum VaultCommand {
    /// List all available vaults with their status and information
    List,
    /// Create a new vault with separate encryption and organization
    Create {
        /// Name for the new vault (e.g., "work-secrets", "personal")
        name: String,
        /// Custom file path for vault storage (optional)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Vault category for organization: personal, work, team, project, testing, archive, or custom
        #[arg(long, default_value = "personal")]
        category: String,
        /// Optional description explaining the vault's purpose
        #[arg(long)]
        description: Option<String>,
    },
    /// Switch to a different vault (makes it the active vault for operations)
    Switch {
        /// Vault ID or name to switch to
        vault: String,
    },
    /// Show information about the currently active vault
    Active,
    /// Delete a vault and optionally remove its database file
    Delete {
        /// Vault ID or name to delete
        vault: String,
        /// Also delete the vault database file from disk (DESTRUCTIVE)
        #[arg(long)]
        delete_file: bool,
    },
    /// Import an existing vault database file into the vault registry
    Import {
        file: PathBuf,
        name: String,
        #[arg(long, default_value = "personal")]
        category: String,
        #[arg(long)]
        copy: bool,
    },
    /// Show vault information
    Info { vault: Option<String> },
    /// Update vault metadata
    Update {
        /// Path to the vault database file to import
        vault: String,
        /// Display name for the imported vault
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        /// Category for organizing the imported vault
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        favorite: Option<bool>,
    },
}

/// Handles various vault-related commands by interacting with a `VaultManager` instance.
///
/// # Arguments
/// - `manager`: A mutable reference to the `VaultManager` instance responsible for managing vaults.
/// - `cmd`: A `VaultCommand` specifying the action to be performed (e.g., listing vaults, creating a vault).
///
/// # Returns
/// - `Result<()>`: Returns `Ok(())` if the command was processed successfully, or an error if something went wrong.
///
/// # Supported Commands
///
/// 1. **List (`VaultCommand::List`)**:
///    Lists all the vaults managed by the `VaultManager`. Each vault displays its ID, name, category, path, creation time,
///    description (if available), active status, and favorite status.
///
/// 2. **Create (`VaultCommand::Create`)**:
///    Creates a new vault with the specified name, path, category, and optional description.
///    It prompts the user for a master password and generates a new vault with a unique ID.
///
///    - `name`: The name of the new vault.
///    - `path`: The directory where the vault will be stored.
///    - `category`: A category to organize the vault.
///    - `description`: (Optional) Additional details about the vault.
///
/// 3. **Switch (`VaultCommand::Switch`)**:
///    Switches the active vault to the one specified by its ID or name. Updates the `VaultManager` to reflect this change.
///
/// 4. **Active (`VaultCommand::Active`)**:
///    Displays information about the currently active vault, including its name, ID, path, category, and description
///    (if set). If no vault is active, it notifies the user.
///
/// 5. **Delete (`VaultCommand::Delete`)**:
///    Deletes a specified vault either by its ID or name. If `delete_file` is true, the associated vault file is also
///    removed. The user is prompted for confirmation before deletion.
///
///    - `vault`: The ID or name of the vault to be deleted.
///    - `delete_file`: A flag indicating whether the file on disk should also be deleted.
///
/// 6. **Import (`VaultCommand::Import`)**:
///    Imports an existing vault from a specified file. The user can assign a new name and category, and optionally copy
///    the vault file to the vault directory.
///
///    - `file`: The path to the file to import.
///    - `name`: The name of the imported vault.
///    - `category`: A category to assign to the newly imported vault.
///    - `copy`: A flag indicating whether to copy the file or reference it directly.
///
/// 7. **Info (`VaultCommand::Info`)**:
///    Retrieves and displays detailed information about a vault. If no specific vault is specified, it uses the currently
///    active vault.
///
///    - `vault`: Optionally specify a vault by ID or name. If not provided, the active vault is used.
///
/// 8. **Update (`VaultCommand::Update`)**:
///    Updates the details of a specified vault, including name, description, category, and favorite status.
///
///    - `vault`: The ID or name of the vault to update.
///    - `name`: (Optional) A new name for the vault.
///    - `description`: (Optional) A new description for the vault.
///    - `category`: (Optional) A new category for the vault. Will be parsed to validate.
///    - `favorite`: (Optional) Updates the favorite status.
///
/// # Errors
/// This function may return an error in the following cases:
/// - Invalid input or data (e.g., an invalid category or vault ID).
/// - Operational issues such as file read/write errors, or issues updating the `VaultManager`.
/// - Failure to interact with the user for input or confirmation.
#[allow(clippy::too_many_lines)]
pub fn handle_vault_command(manager: &mut VaultManager, cmd: crate::VaultCommand) -> color_eyre::Result<()> {
    match cmd {
        crate::VaultCommand::List => {
            let manager = VaultManager::new()?;
            let vaults = manager.list_vaults();

            if vaults.is_empty() {
                println!("No vaults found. Create your first vault with:");
                println!("  chamber registry create <name>");
                return Ok(());
            }

            println!("📁 Available Vaults:");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            for vault in vaults {
                let status = if manager.is_vault_open(&vault.id) {
                    "Open"
                } else {
                    "Closed"
                };
                let active_indicator = if Some(&vault.id) == manager.registry.active_vault_id.as_ref() {
                    " ← Active"
                } else {
                    ""
                };

                println!("ID: {}           Name: {}{}", vault.id, vault.name, active_indicator);
                println!("Category: {}          Status: {}", vault.category, status);
                if let Some(desc) = &vault.description {
                    println!("Description: {desc}");
                }
                println!("Path: {}", vault.path.display());
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            }
        }

        crate::VaultCommand::Create {
            name,
            path,
            category,
            description,
        } => {
            let category = parse_category(&category);
            let password = rpassword::prompt_password("Enter master password for new vault: ")?;
            let vault_id = manager.create_vault(name.clone(), path, category, description, &password)?;
            println!("Created vault '{name}' with ID: {vault_id}");
        }

        crate::VaultCommand::Switch { vault } => {
            // Try to find vault by ID or name
            let vault_id = find_vault_id(manager, &vault)?;
            manager.switch_active_vault(&vault_id)?;
            println!("Switched to vault: {vault_id}");
        }

        crate::VaultCommand::Active => {
            if let Some(active) = manager.registry.get_active_vault() {
                println!("Active vault: {} ({})", active.name, active.id);
                println!("  Path: {}", active.path.display());
                println!("  Category: {}", active.category);
                if let Some(desc) = &active.description {
                    println!("  Description: {desc}");
                }
            } else {
                println!("No active vault");
            }
        }

        crate::VaultCommand::Delete { vault, delete_file } => {
            let vault_id = find_vault_id(manager, &vault)?;

            // Check if this is the active vault
            let is_active_vault = manager.registry.active_vault_id.as_ref() == Some(&vault_id);
            let vault_count = manager.registry.vaults.len();

            // Prevent deletion of active vault unless it's the only one
            if is_active_vault && vault_count > 1 {
                println!("❌ Cannot delete the active vault while other vaults exist.");
                println!("💡 Switch to another vault first using: chamber registry switch <vault-name>");
                println!("💡 Or list all vaults with: chamber registry list");
                return Ok(());
            }

            // Special message for deleting the last/only vault
            if vault_count == 1 {
                println!("⚠️  You are deleting the only remaining vault!");
            }

            print!("Are you sure you want to delete vault '{vault_id}'? ");
            if delete_file {
                print!("This will also delete the vault file! ");
            }
            print!("(y/N): ");

            // Flush stdout to ensure the prompt is displayed immediately
            std::io::Write::flush(&mut std::io::stdout())?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() == "y" {
                manager.delete_vault(&vault_id, delete_file)?;
                println!("✅ Deleted vault: {vault_id}");
            } else {
                println!("Cancelled");
            }
        }

        crate::VaultCommand::Import {
            file,
            name,
            category,
            copy,
        } => {
            let category = parse_category(&category);
            let vault_id = manager.import_vault(&file, name.clone(), category, copy)?;
            println!("Imported vault '{name}' with ID: {vault_id}");
        }

        crate::VaultCommand::Info { vault } => {
            let vault_id = if let Some(v) = vault {
                find_vault_id(manager, &v)?
            } else {
                manager
                    .registry
                    .get_active_vault()
                    .ok_or_else(|| eyre!("No active vault"))?
                    .id
                    .clone()
            };

            if let Some(info) = manager.registry.get_vault(&vault_id) {
                println!("Vault Information:");
                println!("  ID: {}", info.id);
                println!("  Name: {}", info.name);
                println!("  Category: {}", info.category);
                println!("  Path: {}", info.path.display());
                println!(
                    "  Created: {}",
                    info.created_at.format(&time::format_description::well_known::Rfc3339)?
                );
                println!(
                    "  Last accessed: {}",
                    info.last_accessed
                        .format(&time::format_description::well_known::Rfc3339)?
                );
                println!("  Active: {}", info.is_active);
                println!("  Favorite: {}", info.is_favorite);

                if let Some(desc) = &info.description {
                    println!("  Description: {desc}");
                }
            }
        }

        crate::VaultCommand::Update {
            vault,
            name,
            description,
            category,
            favorite,
        } => {
            let vault_id = find_vault_id(manager, &vault)?;
            let category = category.map(|cat| parse_category(&cat));

            manager.update_vault_info(&vault_id, name, description, category, favorite)?;
            println!("Updated vault: {vault_id}");
        }
    }

    Ok(())
}

fn parse_category(category: &str) -> VaultCategory {
    match category.to_lowercase().as_str() {
        "personal" => VaultCategory::Personal,
        "work" => VaultCategory::Work,
        "team" => VaultCategory::Team,
        "project" => VaultCategory::Project,
        "testing" => VaultCategory::Testing,
        "archive" => VaultCategory::Archive,
        custom => VaultCategory::Custom(custom.to_string()),
    }
}

fn find_vault_id(manager: &VaultManager, identifier: &str) -> color_eyre::Result<String> {
    // First try exact ID match
    if manager.registry.vaults.contains_key(identifier) {
        return Ok(identifier.to_string());
    }

    // Then try name match
    for (id, vault_info) in &manager.registry.vaults {
        if vault_info.name.to_lowercase() == identifier.to_lowercase() {
            return Ok(id.clone());
        }
    }

    Err(eyre!("Vault '{}' not found", identifier))
}
