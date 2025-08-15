use chamber_backup::BackupManager;
use chamber_import_export::{detect_format_from_extension, import_items};
use chamber_vault::{BackupConfig, Vault};
use clap::Subcommand;
use color_eyre::eyre::eyre;
use std::path::{Path, PathBuf};

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
pub fn handle_backup_command(vault: Vault, cmd: BackupCommand) -> color_eyre::Result<()> {
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
) -> color_eyre::Result<()> {
    let mut config = vault.get_backup_config().unwrap_or_default();
    let mut changed = false;

    if let Some(enabled) = enable {
        config.enabled = enabled;
        changed = true;
        println!("Automatic backups {}", if enabled { "enabled" } else { "disabled" });
    }

    if let Some(hours) = interval {
        if hours == 0 {
            return Err(eyre!("Backup interval must be greater than 0"));
        }
        config.interval_hours = hours;
        changed = true;
        println!("Backup interval set to {hours} hours");
    }

    if let Some(max) = max_backups {
        if max == 0 {
            return Err(eyre!("Max backups must be greater than 0"));
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
            _ => return Err(eyre!("Invalid format '{}'. Use: json, csv, or backup", fmt)),
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

fn handle_backup_now(mut vault: Vault, output: Option<PathBuf>, force: bool) -> color_eyre::Result<()> {
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
            .ok_or_else(|| eyre!("No backup needed (use --force to create anyway)"))?
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

fn handle_list_backups(vault: Vault, verbose: bool) -> color_eyre::Result<()> {
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

fn handle_restore_backup(mut vault: Vault, backup_path: &Path, skip_confirmation: bool) -> color_eyre::Result<()> {
    if !backup_path.exists() {
        return Err(eyre!("Backup file not found: {}", backup_path.display()));
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

fn handle_verify_backup(backup_path: &PathBuf) -> color_eyre::Result<()> {
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

fn handle_backup_status(vault: Vault) -> color_eyre::Result<()> {
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

fn handle_cleanup_backups(vault: Vault, keep: Option<usize>, dry_run: bool) -> color_eyre::Result<()> {
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

fn verify_backup_file(backup_path: &std::path::Path) -> color_eyre::Result<()> {
    if !backup_path.exists() {
        return Err(eyre!("Backup file does not exist"));
    }

    let metadata = std::fs::metadata(backup_path)?;
    if metadata.len() == 0 {
        return Err(eyre!("Backup file is empty"));
    }

    // Try to parse the backup based on format
    if let Some(format) = detect_format_from_extension(backup_path) {
        import_items(backup_path, &format)?;
    } else {
        return Err(eyre!("Unable to detect backup format"));
    }

    Ok(())
}
