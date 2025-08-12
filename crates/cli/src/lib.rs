use anyhow::{Result, anyhow};
use chamber_backup::BackupManager;
use chamber_import_export::{ExportFormat, detect_format_from_extension, export_items, import_items};
use chamber_password_gen::{
    PasswordConfig, generate_complex_password, generate_memorable_password, generate_simple_password,
};
use chamber_vault::{BackupConfig, ItemKind, NewItem, Vault, VaultCategory, VaultManager};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::str::FromStr;

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
        /// Vault category for organization: personal, work, team, project, testing, archive
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
pub fn handle_vault_command(manager: &mut VaultManager, cmd: VaultCommand) -> Result<()> {
    match cmd {
        VaultCommand::List => {
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

        VaultCommand::Create {
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

        VaultCommand::Switch { vault } => {
            // Try to find vault by ID or name
            let vault_id = find_vault_id(manager, &vault)?;
            manager.switch_active_vault(&vault_id)?;
            println!("Switched to vault: {vault_id}");
        }

        VaultCommand::Active => {
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

        VaultCommand::Delete { vault, delete_file } => {
            let vault_id = find_vault_id(manager, &vault)?;

            print!("Are you sure you want to delete vault '{vault_id}'? ");
            if delete_file {
                print!("This will also delete the vault file! ");
            }
            print!("(y/N): ");

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() == "y" {
                manager.delete_vault(&vault_id, delete_file)?;
                println!("Deleted vault: {vault_id}");
            } else {
                println!("Cancelled");
            }
        }

        VaultCommand::Import {
            file,
            name,
            category,
            copy,
        } => {
            let category = parse_category(&category);
            let vault_id = manager.import_vault(&file, name.clone(), category, copy)?;
            println!("Imported vault '{name}' with ID: {vault_id}");
        }

        VaultCommand::Info { vault } => {
            let vault_id = if let Some(v) = vault {
                find_vault_id(manager, &v)?
            } else {
                manager
                    .registry
                    .get_active_vault()
                    .ok_or_else(|| anyhow::anyhow!("No active vault"))?
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

        VaultCommand::Update {
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

fn find_vault_id(manager: &VaultManager, identifier: &str) -> Result<String> {
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

    Err(anyhow::anyhow!("Vault '{}' not found", identifier))
}

#[derive(Subcommand, Debug)]
pub enum BackupCommand {
    /// Configure automatic backups
    Configure {
        /// Enable or disable automatic backups
        #[arg(long)]
        enable: Option<bool>,
        /// Backup interval in hours
        #[arg(long)]
        interval: Option<u64>,
        /// Maximum number of backups to retain
        #[arg(long)]
        max_backups: Option<usize>,
        /// Backup directory path
        #[arg(long)]
        backup_dir: Option<PathBuf>,
        /// Backup format (json, csv, backup)
        #[arg(long)]
        format: Option<String>,
        /// Enable/disable compression
        #[arg(long)]
        compress: Option<bool>,
        /// Enable/disable backup verification
        #[arg(long)]
        verify: Option<bool>,
    },
    /// Create a backup immediately
    Now {
        /// Optional custom backup path
        #[arg(long)]
        output: Option<PathBuf>,
        /// Force backup even if one was recently created
        #[arg(long)]
        force: bool,
    },
    /// List all existing backups
    List {
        /// Show detailed information
        #[arg(long, short)]
        verbose: bool,
    },
    /// Restore from a backup file
    Restore {
        /// Path to the backup file
        backup_path: PathBuf,
        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },
    /// Verify backup integrity
    Verify {
        /// Path to the backup file to verify
        backup_path: PathBuf,
    },
    /// Show current backup configuration
    Status,
    /// Clean up old backups manually
    Cleanup {
        /// Number of backups to keep (overrides config)
        #[arg(long)]
        keep: Option<usize>,
        /// Perform a dry run (show what would be deleted)
        #[arg(long)]
        dry_run: bool,
    },
}

/// Handles backup-related commands issued by the user.
///
/// This function processes a given `BackupCommand` by delegating it to the appropriate
/// handler based on the command variant. The function works on the provided `Vault` instance
/// and executes the associated functionality.
///
/// # Arguments
///
/// * `vault` - A `Vault` instance representing the storage system or environment where backups
///   are managed.
/// * `cmd` - An instance of the `BackupCommand` enum, specifying the backup operation to be performed.
///
/// # Variants of `BackupCommand`
///
/// - `Configure`:
///   Configures backup settings such as enabling/disabling backup, setting backup intervals,
///   maximum number of backups to retain, backup directory, format, compression settings,
///   and verification options.
///
///   * `enable` - Whether to enable or disable backups.
///   * `interval` - The interval between automatic backups (if enabled).
///   * `max_backups` - Maximum number of backups to retain.
///   * `backup_dir` - Directory where backups are saved.
///   * `format` - Format of the backup files.
///   * `compress` - Whether to compress backups.
///   * `verify` - Whether to verify backups after creation.
///
/// - `Now`:
///   Immediately initiates a backup.
///   * `output` - Optional output location for the backup.
///   * `force` - Whether to force the backup operation, ignoring certain conditions.
///
/// - `List`:
///   Lists existing backups.
///   * `verbose` - If `true`, provides detailed information about each backup.
///
/// - `Restore`:
///   Restores a specific backup.
///   * `backup_path` - Path to the backup file to restore from.
///   * `yes` - If `true`, skips confirmation prompts.
///
/// - `Verify`:
///   Verifies the integrity of a specific backup file.
///   * `backup_path` - Path to the backup file to verify.
///
/// - `Status`:
///   Provides the current backup status, including information about recent backups
///   and configuration status.
///
/// - `Cleanup`:
///   Cleans up old or excess backups based on retention rules.
///   * `keep` - Number of recent backups to retain.
///   * `dry_run` - If `true`, performs a simulation of the cleanup process without deleting files.
///
/// # Returns
///
/// * `Ok(())` - If the specified command is successfully executed.
/// * `Err` - If an error occurs during processing. The specific error depends on the invoked operation.
///
/// # Errors
///
/// This function will return an error if:
/// - The supplied arguments are invalid.
/// - The `Vault` is inaccessible or improperly configured.
/// - File operations (e.g., reading/writing backups) encounter an issue.
/// - A requested backup file is not found, corrupted, or fails verification.
///
pub fn handle_backup_command(vault: Vault, cmd: BackupCommand) -> Result<()> {
    match cmd {
        BackupCommand::Configure {
            enable,
            interval,
            max_backups,
            backup_dir,
            format,
            compress,
            verify,
        } => handle_configure(
            &vault,
            enable,
            interval,
            max_backups,
            backup_dir,
            format,
            compress,
            verify,
        ),

        BackupCommand::Now { output, force } => handle_backup_now(vault, output, force),

        BackupCommand::List { verbose } => handle_list_backups(vault, verbose),

        BackupCommand::Restore { backup_path, yes } => handle_restore_backup(vault, &backup_path, yes),

        BackupCommand::Verify { backup_path } => handle_verify_backup(&backup_path),

        BackupCommand::Status => handle_backup_status(vault),

        BackupCommand::Cleanup { keep, dry_run } => handle_cleanup_backups(vault, keep, dry_run),
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_configure(
    vault: &Vault,
    enable: Option<bool>,
    interval: Option<u64>,
    max_backups: Option<usize>,
    backup_dir: Option<PathBuf>,
    format: Option<String>,
    compress: Option<bool>,
    verify: Option<bool>,
) -> Result<()> {
    let mut config = vault.get_backup_config().unwrap_or_default();
    let mut changed = false;

    if let Some(enabled) = enable {
        config.enabled = enabled;
        changed = true;
        println!("Automatic backups {}", if enabled { "enabled" } else { "disabled" });
    }

    if let Some(hours) = interval {
        if hours == 0 {
            return Err(anyhow!("Backup interval must be greater than 0"));
        }
        config.interval_hours = hours;
        changed = true;
        println!("Backup interval set to {hours} hours");
    }

    if let Some(max) = max_backups {
        if max == 0 {
            return Err(anyhow!("Max backups must be greater than 0"));
        }
        config.max_backups = max;
        changed = true;
        println!("Maximum backups set to {max}");
    }

    if let Some(dir) = backup_dir {
        config.backup_dir.clone_from(&dir);
        changed = true;
        println!("Backup directory set to: {}", dir.display());
    }

    if let Some(fmt) = format {
        match fmt.as_str() {
            "json" | "csv" | "backup" => {
                config.format.clone_from(&fmt);
                changed = true;
                println!("Backup format set to: {fmt}");
            }
            _ => return Err(anyhow!("Invalid format '{}'. Use: json, csv, or backup", fmt)),
        }
    }

    if let Some(comp) = compress {
        config.compress = comp;
        changed = true;
        println!("Compression {}", if comp { "enabled" } else { "disabled" });
    }

    if let Some(ver) = verify {
        config.verify_after_backup = ver;
        changed = true;
        println!("Backup verification {}", if ver { "enabled" } else { "disabled" });
    }

    if changed {
        vault.set_backup_config(&config)?;
        println!("Backup configuration updated successfully");
    } else {
        println!("No configuration changes specified");
        print_backup_config(&vault.get_backup_config().unwrap_or_default());
    }

    Ok(())
}

fn handle_backup_now(mut vault: Vault, output: Option<PathBuf>, force: bool) -> Result<()> {
    // Unlock vault if needed
    if !vault.is_unlocked() {
        let password = rpassword::prompt_password("Enter master password: ")?;
        vault.unlock(&password)?;
    }

    let config = if let Some(custom_path) = output {
        let mut config = vault.get_backup_config().unwrap_or_default();
        config.backup_dir = custom_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        config
    } else {
        vault.get_backup_config().unwrap_or_default()
    };

    let mut backup_manager = BackupManager::new(vault, config);

    println!("Creating backup...");
    let backup_path = if force {
        backup_manager.force_backup()?
    } else {
        backup_manager
            .backup_if_needed()?
            .ok_or_else(|| anyhow!("No backup needed (use --force to create anyway)"))?
    };

    println!("✅ Backup created successfully: {}", backup_path.display());

    // Show backup info
    let metadata = std::fs::metadata(&backup_path)?;
    println!("   Size: {} bytes", metadata.len());
    println!(
        "   Created: {}",
        chrono::DateTime::<chrono::Utc>::from(metadata.created()?).format("%Y-%m-%d %H:%M:%S UTC")
    );

    Ok(())
}

fn handle_list_backups(vault: Vault, verbose: bool) -> Result<()> {
    let config = vault.get_backup_config().unwrap_or_default();
    let backup_manager = BackupManager::new(vault, config);

    let backups = backup_manager.find_all_backups()?;

    if backups.is_empty() {
        println!("No backups found in: {}", backup_manager.config.backup_dir.display());
        return Ok(());
    }

    println!("Found {} backup(s):", backups.len());
    println!();

    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::expect_used)]
    for (i, backup_path) in backups.iter().enumerate() {
        let metadata = std::fs::metadata(backup_path)?;
        let size = metadata.len();
        let modified = metadata.modified()?;
        let datetime = chrono::DateTime::<chrono::Utc>::from(modified);

        println!(
            "{}. {}",
            i + 1,
            backup_path
                .file_name()
                .expect("Unable to get the file name")
                .to_string_lossy()
        );
        println!("   Path: {}", backup_path.display());
        println!("   Size: {} bytes ({:.2} MB)", size, size as f64 / 1_048_576.0);
        println!("   Date: {}", datetime.format("%Y-%m-%d %H:%M:%S UTC"));

        if verbose {
            // Try to extract additional info from the backup
            if let Some(timestamp) = backup_manager.extract_timestamp_from_filename(backup_path) {
                println!(
                    "   Timestamp: {}",
                    timestamp.format(&time::format_description::well_known::Rfc3339)?
                );
            }
        }
        println!();
    }

    Ok(())
}

fn handle_restore_backup(mut vault: Vault, backup_path: &Path, skip_confirmation: bool) -> Result<()> {
    if !backup_path.exists() {
        return Err(anyhow!("Backup file not found: {}", backup_path.display()));
    }

    // Verify backup first
    println!("Verifying backup integrity...");
    verify_backup_file(backup_path)?;
    println!("✅ Backup verification passed");

    if !skip_confirmation {
        println!("⚠️  WARNING: This will replace all current vault data!");
        print!("Are you sure you want to restore from backup? (y/N): ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Restore cancelled");
            return Ok(());
        }
    }

    // Unlock vault
    let password = rpassword::prompt_password("Enter master password: ")?;
    vault.unlock(&password)?;

    // Detect backup format
    let format = chamber_import_export::detect_format_from_extension(backup_path)
        .unwrap_or(chamber_import_export::ExportFormat::Json);

    println!("Importing backup data...");
    let items = chamber_import_export::import_items(backup_path, &format)?;

    println!("Found {} items in backup", items.len());

    // Clear existing items (if user confirmed)
    let existing_items = vault.list_items()?;
    for item in existing_items {
        vault.delete_item(item.id)?;
    }

    // Import new items
    let mut imported_count = 0;
    for item in items {
        match vault.create_item(&item) {
            Ok(()) => imported_count += 1,
            Err(e) => println!("⚠️  Failed to import '{}': {}", item.name, e),
        }
    }

    println!("✅ Successfully imported {imported_count} items from backup");
    Ok(())
}

fn handle_verify_backup(backup_path: &PathBuf) -> Result<()> {
    println!("Verifying backup: {}", backup_path.display());

    verify_backup_file(backup_path)?;

    println!("✅ Backup verification passed");

    // Show additional info
    let metadata = std::fs::metadata(backup_path)?;
    println!("   Size: {} bytes", metadata.len());

    // Try to get item count
    if let Some(format) = chamber_import_export::detect_format_from_extension(backup_path) {
        if let Ok(items) = chamber_import_export::import_items(backup_path, &format) {
            println!("   Items: {}", items.len());
        }
    }

    Ok(())
}

fn handle_backup_status(vault: Vault) -> Result<()> {
    let config = vault.get_backup_config().unwrap_or_default();

    println!("🔒 Backup Configuration Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    print_backup_config(&config);

    // Show last backup info
    let backup_manager = BackupManager::new(vault, config);
    if let Ok(Some(last_backup)) = backup_manager.find_most_recent_backup() {
        println!();
        println!("📁 Most Recent Backup:");
        println!("   File: {}", last_backup.display());

        if let Ok(metadata) = std::fs::metadata(&last_backup) {
            let modified = metadata.modified()?;
            let datetime = chrono::DateTime::<chrono::Utc>::from(modified);
            println!("   Date: {}", datetime.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("   Size: {} bytes", metadata.len());
        }
    } else {
        println!();
        println!("📁 No backups found");
    }

    Ok(())
}

fn handle_cleanup_backups(vault: Vault, keep: Option<usize>, dry_run: bool) -> Result<()> {
    let mut config = vault.get_backup_config().unwrap_or_default();

    if let Some(keep_count) = keep {
        config.max_backups = keep_count;
    }

    let backup_manager = BackupManager::new(vault, config.clone());
    let all_backups = backup_manager.find_all_backups()?;

    if all_backups.len() <= config.max_backups {
        println!(
            "No cleanup needed. Found {} backups, keeping {}",
            all_backups.len(),
            config.max_backups
        );
        return Ok(());
    }

    let to_delete = all_backups.len() - config.max_backups;
    println!(
        "Found {} backups, will {} {} oldest backup(s)",
        all_backups.len(),
        if dry_run { "identify" } else { "delete" },
        to_delete
    );

    // Sort by timestamp (the oldest first for deletion)
    let mut sorted_backups = all_backups;
    sorted_backups.sort_by(|a, b| {
        let time_a = backup_manager
            .extract_timestamp_from_filename(a)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        let time_b = backup_manager
            .extract_timestamp_from_filename(b)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        time_a.cmp(&time_b)
    });

    for (i, backup_path) in sorted_backups.iter().take(to_delete).enumerate() {
        if dry_run {
            println!("  [{}] Would delete: {}", i + 1, backup_path.display());
        } else {
            match std::fs::remove_file(backup_path) {
                Ok(()) => println!("  [{}] Deleted: {}", i + 1, backup_path.display()),
                Err(e) => println!("  [{}] Failed to delete {}: {}", i + 1, backup_path.display(), e),
            }
        }
    }

    if dry_run {
        println!("Dry run completed. Use without --dry-run to actually delete files.");
    } else {
        println!("✅ Cleanup completed");
    }

    Ok(())
}

fn print_backup_config(config: &BackupConfig) {
    println!(
        "   Status: {}",
        if config.enabled { "✅ Enabled" } else { "❌ Disabled" }
    );
    println!("   Directory: {}", config.backup_dir.display());
    println!("   Interval: {} hours", config.interval_hours);
    println!("   Max backups: {}", config.max_backups);
    println!("   Format: {}", config.format);
    println!(
        "   Compression: {}",
        if config.compress { "✅ Enabled" } else { "❌ Disabled" }
    );
    println!(
        "   Verification: {}",
        if config.verify_after_backup {
            "✅ Enabled"
        } else {
            "❌ Disabled"
        }
    );
}

fn verify_backup_file(backup_path: &std::path::Path) -> Result<()> {
    if !backup_path.exists() {
        return Err(anyhow!("Backup file does not exist"));
    }

    let metadata = std::fs::metadata(backup_path)?;
    if metadata.len() == 0 {
        return Err(anyhow!("Backup file is empty"));
    }

    // Try to parse the backup based on format
    if let Some(format) = chamber_import_export::detect_format_from_extension(backup_path) {
        chamber_import_export::import_items(backup_path, &format)?;
    } else {
        return Err(anyhow!("Unable to detect backup format"));
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[command(
    name = "chamber",
    about = "A secure, local-first secrets manager with encrypted storage and multiple vault support",
    long_about = "Chamber is a secure secrets manager that stores your passwords, API keys, and other \
                  sensitive information using strong encryption. All data is stored locally with \
                  zero-knowledge architecture.\n\n\
                  Features:\n\
                  • ChaCha20-Poly1305 authenticated encryption\n\
                  • Argon2 key derivation for master passwords\n\
                  • Multiple vault support for organization\n\
                  • Automatic backup system with retention policies\n\
                  • Import/export in JSON, CSV, and Chamber formats\n\
                  • Terminal UI and command-line interface\n\
                  • Secure password generation\n\n\
                  Quick start:\n\
                  1. chamber init              # Initialize your first vault\n\
                  2. chamber add -n \"github-token\" -k apikey -v \"your-token\"\n\
                  3. chamber list              # View your secrets\n\
                  4. chamber ui                # Launch terminal interface"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new Chamber vault with master password encryption
    Init,

    /// Add a new secret item to the vault
    Add {
        /// Name/identifier for the secret (e.g., "github-token", "database-password")
        #[arg(short, long)]
        name: String,
        /// Type of secret: password, apikey, envvar, sshkey, certificate, database, note
        #[arg(short, long, default_value = "note")]
        kind: String,
        /// The secret value to store (will be encrypted)
        #[arg(short, long)]
        value: String,
    },

    /// List all secrets in the vault (names and types only)
    List,

    /// Retrieve and display a specific secret by name
    Get {
        /// Name of the secret to retrieve
        #[arg(short, long)]
        name: String,
    },

    /// Generate secure passwords with customizable options
    Generate {
        /// Password length (default: 16 characters)
        #[arg(short, long, default_value = "16")]
        length: usize,
        /// Generate simple alphanumeric password
        #[arg(long)]
        simple: bool,
        /// Generate complex password with all character types
        #[arg(long)]
        complex: bool,
        /// Generate memorable password with words
        #[arg(long)]
        memorable: bool,
        /// Exclude uppercase letters (A-Z)
        #[arg(long)]
        no_uppercase: bool,
        /// Exclude lowercase letters (a-z)
        #[arg(long)]
        no_lowercase: bool,
        /// Exclude digits (0-9)
        #[arg(long)]
        no_digits: bool,
        /// Exclude symbols (!@#$%^&*)
        #[arg(long)]
        no_symbols: bool,
        /// Include ambiguous characters (0/O, 1/l/I)
        #[arg(long)]
        include_ambiguous: bool,
        /// Number of passwords to generate
        #[arg(short, long)]
        count: Option<usize>,
    },

    /// Export vault contents to a file for backup or migration
    Export {
        /// Output file path (e.g., backup.json, secrets.csv)
        #[arg(short, long)]
        output: PathBuf,
        /// Export format: json, csv, backup (auto-detected from file extension)
        #[arg(short, long)]
        format: Option<String>,
        /// Include creation/modification timestamps in export
        #[arg(long)]
        include_metadata: bool,
    },

    /// Import secrets from a file into the vault
    Import {
        /// Input file path containing secrets to import
        #[arg(short, long)]
        input: PathBuf,
        /// Import format: json, csv, backup (auto-detected from file extension)
        #[arg(short, long)]
        format: Option<String>,
        /// Preview import without making changes
        #[arg(long)]
        dry_run: bool,
        /// Skip items that already exist in the vault
        #[arg(long)]
        skip_duplicates: bool,
    },

    /// Backup management commands for automatic data protection
    #[command(subcommand)]
    Backup(BackupCommand),

    /// Multiple vault management commands for organizing secrets
    #[command(subcommand)]
    Registry(VaultCommand),
}

/// Handles various commands related to a vault system, including initialization,
/// adding items, listing items, retrieving items, generating passwords,
/// exporting items, and importing items.
///
/// # Arguments
/// * `cmd` - An instance of the `Commands` enumeration specifying the action to perform.
///
/// # Returns
/// * A `Result` that indicates success (`Ok`) or failure (`Err`) with an error message.
///
/// # Supported Commands
/// - `Commands::Init`:
///   Initializes a vault at a specified location. If the vault is already initialized,
///   it informs the user. Prompts the user to create and confirm a master key during initialization.
///
/// - `Commands::Add`:
///   Adds a new item to the vault.
///   - Prompts the user for the master key to unlock the vault.
///   - Accepts parameters `name`, `kind` (e.g., password, ssh key, note), and `value`.
///   - Returns an error if the item already exists or if any operation fails.
///
/// - `Commands::List`:
///   Lists all items in the vault.
///   - Prompts the user for the master key to unlock the vault.
///   - Outputs items in the format `- <name> [<item_kind>]`.
///
/// - `Commands::Get`:
///   Retrieves the value of an item by its name.
///   - Prompts the user for the master key to unlock the vault.
///   - Returns the item's value if found, or an error if the item is not found.
///
/// - `Commands::Generate`:
///   Generates passwords with customizable options.
///   - `length`: Specifies the password length.
///   - `simple`, `complex`, `memorable`: Password style preferences.
///   - `no_uppercase`, `no_lowercase`, `no_digits`, `no_symbols`: Exclusion flags for character sets.
///   - `include_ambiguous`: Includes or excludes ambiguous characters.
///   - `count`: Number of passwords to generate (if greater than 1, passwords are numbered in output).
///   - Outputs the generated password(s).
///
/// - `Commands::Export`:
///   Exports all items in the vault to a file.
///   - Prompts the user for the master key to unlock the vault.
///   - Accepts `output` (path to export file) and `format` (e.g., JSON, XML, etc.).
///   - Detects file format from the output file extension if not explicitly specified.
///   - Outputs the number of items exported and details about the export location and format.
///
/// - `Commands::Import`:
///   Imports items from a file into the vault.
///   - Checks if the input file exists.
///   - Detects file format from input file extension if not explicitly specified.
///   - Accepts options:
///     - `dry_run`: Only displays items to be imported without making changes.
///     - `skip_duplicates`: Skips importing items that already exist in the vault.
///   - Prompts the user for the master key to unlock the vault.
///   - Outputs the number of items imported, skipped, or failed to import.
///
/// # Errors
/// This function returns errors in the following cases:
/// - Failure to open or create a vault.
/// - Failure to unlock or initialize the vault.
/// - IO errors while exporting or importing.
/// - Validation errors during input or conflicts in item names.
/// - Password generation errors for invalid configurations.
///
/// # Examples
/// ```ignore
/// handle_command(Commands::Init);
/// handle_command(Commands::Add {
///     name: "example_credential".to_string(),
///     kind: "password".to_string(),
///     value: "securepassword123".to_string(),
/// });
/// handle_command(Commands::List);
/// handle_command(Commands::Get { name: "example_credential".to_string() });
/// handle_command(Commands::Generate {
///     length: 16,
///     complex: true,
///     simple: false,
///     memorable: false,
///     no_uppercase: false,
///     no_lowercase: false,
///     no_digits: false,
///     no_symbols: false,
///     include_ambiguous: true,
///     count: Some(3),
/// });
/// ```
#[allow(clippy::cognitive_complexity)]
#[allow(clippy::too_many_lines)]
pub fn handle_command(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Init => {
            let mut vault = Vault::open_or_create(None)?;
            if vault.is_initialized() {
                println!("Vault already initialized at {}", vault.db_path().display());
            } else {
                let master = prompt_secret("Create master key: ")?;
                let confirm = prompt_secret("Confirm master key: ")?;
                if master != confirm {
                    return Err(anyhow!("Master keys do not match"));
                }
                vault.initialize(&master)?;
                println!("Initialized vault at {}", vault.db_path().display());
            }
        }
        Commands::Add { name, kind, value } => {
            let mut vault = Vault::open_or_create(None)?;
            let master = prompt_secret("Enter master key: ")?;
            vault.unlock(&master)?;
            let kind = match kind.as_str() {
                "pass" | "password" => ItemKind::Password,
                "env" | "envvar" => ItemKind::EnvVar,
                "ssh" | "sshkey" => ItemKind::SshKey,
                _ => ItemKind::Note,
            };
            let item = NewItem {
                name: name.clone(),
                kind,
                value,
            };
            match vault.create_item(&item) {
                Ok(()) => {
                    println!("Item added.");
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("already exists") {
                        println!("Item '{name}' already exists. Use a different name or update it.");
                        return Ok(());
                    }
                    // Other errors should still bubble up
                    return Err(e);
                }
            }
        }

        Commands::List => {
            let mut vault = Vault::open_or_create(None)?;
            let master = prompt_secret("Enter master key: ")?;
            vault.unlock(&master)?;
            for item in vault.list_items()? {
                println!("- {} [{}]", item.name, item.kind.as_str());
            }
        }
        Commands::Get { name } => {
            let mut vault = Vault::open_or_create(None)?;
            let master = prompt_secret("Enter master key: ")?;
            vault.unlock(&master)?;
            if let Some(item) = vault.get_item_by_name(&name)? {
                println!("{}", item.value);
            } else {
                return Err(anyhow!("Item not found"));
            }
        }
        Commands::Generate {
            length,
            simple,
            complex,
            memorable,
            no_uppercase,
            no_lowercase,
            no_digits,
            no_symbols,
            include_ambiguous,
            count,
        } => {
            let count = count.unwrap_or(1);

            for i in 0..count {
                let password = if memorable {
                    generate_memorable_password()
                } else if simple {
                    generate_simple_password(length)?
                } else if complex {
                    generate_complex_password(length)?
                } else {
                    // Custom configuration
                    PasswordConfig::new()
                        .with_length(length)
                        .with_uppercase(!no_uppercase)
                        .with_lowercase(!no_lowercase)
                        .with_digits(!no_digits)
                        .with_symbols(!no_symbols)
                        .with_exclude_ambiguous(!include_ambiguous)
                        .generate()?
                };

                if count > 1 {
                    println!("{}: {password}", i + 1);
                } else {
                    println!("{password}");
                }
            }
        }
        Commands::Export {
            output,
            format,
            include_metadata: _,
        } => {
            let mut vault = Vault::open_or_create(None)?;
            let master = prompt_secret("Enter master key: ")?;
            vault.unlock(&master)?;

            let items = vault.list_items()?;
            if items.is_empty() {
                println!("No items to export.");
                return Ok(());
            }

            // Determine format
            let export_format = if let Some(format_str) = format {
                ExportFormat::from_str(&format_str)?
            } else {
                // Try to detect from file extension
                detect_format_from_extension(&output).unwrap_or(ExportFormat::Json)
            };

            export_items(&items, &export_format, &output)?;
            println!(
                "Exported {} items to {} (format: {:?})",
                items.len(),
                output.display(),
                export_format
            );
        }
        Commands::Import {
            input,
            format,
            dry_run,
            skip_duplicates,
        } => {
            if !input.exists() {
                return Err(anyhow!("Input file does not exist: {}", input.display()));
            }

            // Determine format
            let import_format = if let Some(format_str) = format {
                ExportFormat::from_str(&format_str)?
            } else {
                // Try to detect from file extension
                detect_format_from_extension(&input)
                    .ok_or_else(|| anyhow!("Could not detect file format. Please specify with --format"))?
            };

            let new_items = import_items(&input, &import_format)?;
            if new_items.is_empty() {
                println!("No items found in import file.");
                return Ok(());
            }

            println!("Found {} items to import", new_items.len());

            if dry_run {
                println!("DRY RUN - Items that would be imported:");
                for item in &new_items {
                    println!("  - {} [{}]", item.name, item.kind.as_str());
                }
                return Ok(());
            }

            let master = prompt_secret("Enter master key: ")?;
            let mut vault = Vault::open_or_create(None)?;
            vault.unlock(&master)?;

            let existing_items = vault.list_items()?;
            let existing_names: std::collections::HashSet<String> =
                existing_items.iter().map(|item| item.name.clone()).collect();

            let mut imported_count = 0;
            let mut skipped_count = 0;

            for item in new_items {
                if existing_names.contains(&item.name) {
                    if skip_duplicates {
                        println!("Skipping duplicate: {}", item.name);
                        skipped_count += 1;
                        continue;
                    }
                    println!(
                        "Warning: Item '{}' already exists and will be skipped. Use --skip-duplicates to suppress this warning.",
                        item.name
                    );
                    skipped_count += 1;
                    continue;
                }

                match vault.create_item(&item) {
                    Ok(()) => {
                        imported_count += 1;
                    }
                    Err(e) => {
                        println!("Failed to import item: {e}");
                    }
                }
            }

            println!("Import complete: {imported_count} imported, {skipped_count} skipped");
        }
        Commands::Backup(backup_cmd) => {
            let vault = Vault::open_or_create(None)?;
            let _ = handle_backup_command(vault, backup_cmd);
        }
        Commands::Registry(registry_cmd) => {
            let mut manager = VaultManager::new()?;
            handle_vault_command(&mut manager, registry_cmd)?;
        }
    }
    Ok(())
}

fn prompt_secret(prompt: &str) -> Result<String> {
    use std::io::{Write, stdout};
    print!("{prompt}");
    stdout().flush()?;
    // Read without echo on Windows/Linux/macOS
    let pass = rpassword::prompt_password("")?;
    Ok(pass)
}

// Rust
#[cfg(test)]
mod handle_command_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn tmp_path(name: &str) -> PathBuf {
        let now = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("chamber_cli_{name}_{pid}_{now}"))
    }

    #[test]
    fn test_generate_simple_ok() {
        // Runs the simple generator branch (no prompts involved)
        let cmd = Commands::Generate {
            length: 12,
            simple: true,
            complex: false,
            memorable: false,
            no_uppercase: false,
            no_lowercase: false,
            no_digits: false,
            no_symbols: false,
            include_ambiguous: false,
            count: None,
        };
        let res = handle_command(cmd);
        assert!(res.is_ok());
    }

    #[test]
    fn test_generate_complex_ok() {
        // Runs the complex generator branch (no prompts involved)
        let cmd = Commands::Generate {
            length: 24,
            simple: false,
            complex: true,
            memorable: false,
            no_uppercase: false,
            no_lowercase: false,
            no_digits: false,
            no_symbols: false,
            include_ambiguous: false,
            count: Some(1),
        };
        let res = handle_command(cmd);
        assert!(res.is_ok());
    }

    #[test]
    fn test_generate_memorable_ok() {
        // Runs the memorable generator branch (no prompts involved)
        let cmd = Commands::Generate {
            length: 16, // ignored by memorable branch but required by struct
            simple: false,
            complex: false,
            memorable: true,
            no_uppercase: false,
            no_lowercase: false,
            no_digits: false,
            no_symbols: false,
            include_ambiguous: false,
            count: None,
        };
        let res = handle_command(cmd);
        assert!(res.is_ok());
    }

    #[test]
    fn test_generate_count_multiple_ok() {
        // Ensure the loop over count works (no prompts involved)
        let cmd = Commands::Generate {
            length: 10,
            simple: true,
            complex: false,
            memorable: false,
            no_uppercase: false,
            no_lowercase: false,
            no_digits: false,
            no_symbols: false,
            include_ambiguous: false,
            count: Some(3),
        };
        let res = handle_command(cmd);
        assert!(res.is_ok());
    }

    #[test]
    fn test_generate_custom_no_character_sets_err() {
        // Triggers the custom branch and makes the generator return an error
        // by disabling all sets. The error should propagate out of handle_command.
        let cmd = Commands::Generate {
            length: 10,
            simple: false,
            complex: false,
            memorable: false,
            no_uppercase: true,
            no_lowercase: true,
            no_digits: true,
            no_symbols: true,
            include_ambiguous: false,
            count: None,
        };
        let err = handle_command(cmd).unwrap_err().to_string();
        println!("{err}");
        assert!(err.contains("At least one character set must be enabled"));
    }

    #[test]
    fn test_import_missing_file_err() {
        // This hits the early error path for missing an input file without prompting.
        let missing = tmp_path("missing").with_extension("json");
        let cmd = Commands::Import {
            input: missing,
            format: Some("json".to_string()),
            dry_run: false,
            skip_duplicates: false,
        };
        let err = handle_command(cmd).unwrap_err().to_string();
        assert!(err.contains("Input file does not exist"));
    }
}
