mod api;
mod backup;
mod health;
mod stats;
mod utils;
mod vault;

use crate::api::handle_api_command;
use crate::backup::{BackupCommand, handle_backup_command};
use crate::health::{analyze_password_strength, handle_health_command};
use crate::stats::handle_stats_command;
use crate::utils::{filter_and_sort_items, format_relative_time};
use crate::vault::{VaultCommand, handle_vault_command};
use chamber_import_export::{ExportFormat, detect_format_from_extension, export_items, import_items};
use chamber_password_gen::{
    PasswordConfig, generate_complex_password, generate_memorable_password, generate_simple_password,
};
use chamber_vault::{Item, ItemKind, NewItem, Vault, VaultManager};
use clap::{Parser, Subcommand};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use std::path::PathBuf;
use std::str::FromStr;
use time::OffsetDateTime;

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
                  4. chamber                   # Launch terminal interface"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the REST API server
    Api {
        #[arg(
            short,
            long,
            default_value = "127.0.0.1:3000",
            help = "Address to bind the API server to"
        )]
        bind: String,
        #[arg(short, long, help = "Port to bind the API server to")]
        port: Option<u16>,
    },

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
        value: Option<String>,
        #[arg(long, help = "Generate a secure password automatically")]
        generate: bool,
        #[arg(long, default_value = "16", help = "Length of generated password")]
        length: Option<usize>,
        #[arg(long, help = "Generate simple password (alphanumeric only)")]
        simple: bool,
        #[arg(long, help = "Generate complex password (all character types)")]
        complex: bool,
        #[arg(long, help = "Generate memorable password")]
        memorable: bool,
    },

    /// Show vault health report
    Health {
        #[arg(long, help = "Show detailed health analysis")]
        detailed: bool,
    },

    /// List all secrets in the vault (names and types only)
    List {
        #[arg(long, help = "Filter by item type (password, apikey, envvar, etc.)")]
        r#type: Option<String>,
        #[arg(long, help = "Show items created since date (e.g., '1 week ago', '3 days ago')")]
        since: Option<String>,
        #[arg(long, help = "Show N most recent items")]
        recent: Option<usize>,
        #[arg(long, help = "Filter by name pattern (supports wildcards like GitHub*)")]
        name: Option<String>,
    },

    /// Retrieve and display a specific secret by name
    Get {
        /// Name of the secret to retrieve
        #[arg(short, long)]
        name: String,
        #[arg(long, help = "Copy value to clipboard instead of displaying it")]
        copy_value: bool,
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
    /// Show vault statistics
    Stats,

    /// Update an existing secret
    Update {
        #[arg(short, long, help = "Name of the item to update")]
        name: String,
        #[arg(short, long, help = "New value for the item")]
        value: Option<String>,
        #[arg(long, help = "Generate a new secure password automatically")]
        generate: bool,
        #[arg(long, default_value = "16", help = "Length of generated password")]
        length: Option<usize>,
        #[arg(long, help = "Generate simple password (alphanumeric only)")]
        simple: bool,
        #[arg(long, help = "Generate complex password (all character types)")]
        complex: bool,
        #[arg(long, help = "Generate memorable password")]
        memorable: bool,
        #[arg(long, help = "Copy updated value to clipboard")]
        copy: bool,
        #[arg(long, help = "Specify which vault to search in (otherwise searches all)")]
        vault: Option<String>,
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
/// # Panics
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
pub async fn handle_command(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Api { bind, port } => handle_api_command(bind, port).await?,

        Commands::Init => {
            let mut vault = Vault::open_or_create(None)?;
            if vault.is_initialized() {
                println!("Vault already initialized at {}", vault.db_path().display());
            } else {
                let master = prompt_secret("Create master key: ")?;
                let confirm = prompt_secret("Confirm master key: ")?;
                if master != confirm {
                    return Err(eyre!("Master keys do not match"));
                }
                vault.initialize(&master)?;
                println!("Initialized vault at {}", vault.db_path().display());
            }
        }
        Commands::Add {
            name,
            kind,
            value,
            generate,
            length,
            simple,
            complex,
            memorable,
        } => {
            let mut vault = Vault::open_or_create(None)?;
            let master = prompt_secret("Enter master key: ")?;
            vault.unlock(&master)?;

            // Determine the value to use
            let item_value = if generate {
                // Validate that value wasn't also provided
                if value.is_some() {
                    return Err(eyre!("Cannot use both --value and --generate options together"));
                }

                // Generate password based on options
                let password_length = length.unwrap_or(16);
                let generated_password = if memorable {
                    generate_memorable_password()
                } else if simple {
                    generate_simple_password(password_length)?
                } else if complex {
                    generate_complex_password(password_length)?
                } else {
                    // Default: secure password with good character mix
                    PasswordConfig::new()
                        .with_length(password_length)
                        .with_uppercase(true)
                        .with_lowercase(true)
                        .with_digits(true)
                        .with_symbols(true)
                        .with_exclude_ambiguous(true)
                        .generate()?
                };

                // Show the generated password to the user
                println!("Generated password: {generated_password}");
                generated_password
            } else {
                match value {
                    Some(v) => v,
                    None => prompt_secret("Enter value: ")?,
                }
            };

            let kind = match kind.as_str() {
                "pass" | "password" => ItemKind::Password,
                "env" | "envvar" => ItemKind::EnvVar,
                "ssh" | "sshkey" => ItemKind::SshKey,
                "apikey" | "api_key" | "api-key" | "token" => ItemKind::ApiKey,
                "certificate" | "cert" | "ssl" | "tls" => ItemKind::Certificate,
                "database" | "db" | "connection" => ItemKind::Database,
                _ => ItemKind::Note,
            };

            let item = NewItem {
                name: name.clone(),
                kind,
                value: item_value,
            };

            match vault.create_item(&item) {
                Ok(()) => {
                    println!("✅ Item '{name}' added successfully.");
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("already exists") {
                        println!("❌ Item '{name}' already exists. Use a different name or update it.");
                        return Ok(());
                    }
                    // Other errors should still bubble up
                    return Err(e);
                }
            }
        }

        Commands::Health { detailed } => {
            let mut vault = match Vault::open_active() {
                Ok(vault) => vault,
                Err(_) => {
                    // If no active vault, try default
                    Vault::open_default()?
                }
            };

            let master = prompt_secret("Enter master key: ")?;
            vault.unlock(&master)?;
            if !vault.is_unlocked() {
                eprintln!("❌ Vault is locked. Please unlock it first.");
                std::process::exit(1);
            }
            handle_health_command(&vault, detailed)?;
        }

        Commands::List {
            r#type,
            since,
            recent,
            name,
        } => {
            let mut vault = Vault::open_or_create(None)?;
            let master = prompt_secret("Enter master key: ")?;
            vault.unlock(&master)?;

            let all_items = vault.list_items()?;
            let filtered_items =
                match filter_and_sort_items(all_items, r#type.as_deref(), since.as_deref(), recent, name.as_deref()) {
                    Ok(items) => items,
                    Err(e) => {
                        return Err(eyre!("Error while filtering items: {e}"));
                    }
                };

            if filtered_items.is_empty() {
                println!("No items found matching the criteria.");
                return Ok(());
            }

            // Display results with enhanced formatting
            println!("Found {} item(s):", filtered_items.len());
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            for item in filtered_items {
                let age = format_relative_time(item.created_at);
                println!(
                    "• {} [{}] - created {} ({})",
                    item.name,
                    item.kind.display_name(),
                    age,
                    item.created_at
                        .format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_else(|_| "unknown".to_string())
                );
            }
        }

        Commands::Get { name, copy_value } => {
            let mut vault = Vault::open_or_create(None)?;
            let master = prompt_secret("Enter master key: ")?;
            vault.unlock(&master)?;
            if let Some(item) = vault.get_item_by_name(&name)? {
                if copy_value {
                    // Copy to clipboard instead of displaying
                    let mut clipboard =
                        arboard::Clipboard::new().map_err(|e| eyre!("Failed to access clipboard: {}", e))?;
                    clipboard
                        .set_text(&item.value)
                        .map_err(|e| eyre!("Failed to copy to clipboard: {}", e))?;
                    println!("✅ Value for '{name}' copied to clipboard");
                } else {
                    println!("{}", item.value);
                }
            } else {
                return Err(eyre!("Item not found"));
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
                return Err(eyre!("Input file does not exist: {}", input.display()));
            }

            // Determine format
            let import_format = if let Some(format_str) = format {
                ExportFormat::from_str(&format_str)?
            } else {
                // Try to detect from file extension
                detect_format_from_extension(&input)
                    .ok_or_else(|| eyre!("Could not detect file format. Please specify with --format"))?
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
        Commands::Stats => {
            let mut vault = match Vault::open_active() {
                Ok(vault) => vault,
                Err(_) => {
                    // If no active vault, try default
                    Vault::open_default()?
                }
            };

            let master = prompt_secret("Enter master key: ")?;
            vault.unlock(&master)?;
            if !vault.is_unlocked() {
                eprintln!("❌ Vault is locked. Please unlock it first.");
                std::process::exit(1);
            }
            handle_stats_command(&vault)?;
        }

        Commands::Update {
            name,
            value,
            generate,
            length,
            simple,
            complex,
            memorable,
            copy,
            vault: vault_name,
        } => {
            // First, try to find the item across all vaults
            let (vault, existing_item, found_vault_name) = if let Some(specific_vault) = vault_name.clone() {
                // Search only in the specified vault
                find_item_in_specific_vault(&name, &specific_vault)?
            } else {
                // Search across all vaults
                find_item_across_vaults(&name)?
            };

            if let (mut vault, Some(item), vault_name) = (vault, existing_item, found_vault_name) {
                println!("🔍 Found '{name}' in vault: {vault_name}");
                println!("📝 Updating '{}' [{}]", item.name, item.kind.display_name());

                // Determine the new value to use
                let new_value = if generate {
                    // Validate that value wasn't also provided
                    if value.is_some() {
                        return Err(eyre!("Cannot use both --value and --generate options together"));
                    }

                    // Generate password based on options
                    let password_length = length.unwrap_or(16);
                    let generated_password = if memorable {
                        generate_memorable_password()
                    } else if simple {
                        generate_simple_password(password_length)?
                    } else if complex {
                        generate_complex_password(password_length)?
                    } else {
                        // Default: secure password with good character mix
                        PasswordConfig::new()
                            .with_length(password_length)
                            .with_uppercase(true)
                            .with_lowercase(true)
                            .with_digits(true)
                            .with_symbols(true)
                            .with_exclude_ambiguous(true)
                            .generate()?
                    };

                    println!("🔐 Generated new password: {generated_password}");
                    generated_password
                } else if let Some(v) = value {
                    println!("📝 Using provided value");
                    v
                } else {
                    // Interactive mode - show current value (masked for passwords)
                    let display_current = if matches!(item.kind, ItemKind::Password | ItemKind::ApiKey) {
                        format!("{}***", &item.value.chars().take(3).collect::<String>())
                    } else {
                        #[allow(clippy::redundant_clone)]
                        item.value.clone()
                    };

                    println!("Current value: {display_current}");
                    prompt_secret("Enter new value (or press Ctrl+C to cancel): ")?
                };

                // Confirm if values are the same
                if new_value == item.value {
                    println!("⚠️  New value is the same as the current value. No changes made.");
                    return Ok(());
                }

                // Update the item
                match vault.update_item(item.id, &new_value) {
                    Ok(()) => {
                        println!("✅ Item '{name}' updated successfully in vault '{vault_name}'.");

                        // Show password strength for password-type items
                        if matches!(item.kind, ItemKind::Password | ItemKind::ApiKey) {
                            let strength = analyze_password_strength(&new_value);
                            println!("🔒 Password strength: {strength}");
                        }

                        // Copy to clipboard if requested
                        if copy {
                            match arboard::Clipboard::new() {
                                Ok(mut clipboard) => {
                                    if let Err(e) = clipboard.set_text(&new_value) {
                                        println!("⚠️  Warning: Failed to copy to clipboard: {e}");
                                    } else {
                                        println!("📋 New value copied to clipboard.");
                                    }
                                }
                                Err(e) => {
                                    println!("⚠️  Warning: Failed to access clipboard: {e}");
                                }
                            }
                        }

                        // Show update timestamp
                        println!(
                            "🕐 Updated at: {}",
                            OffsetDateTime::now_utc()
                                .format(&time::format_description::well_known::Rfc3339)
                                .unwrap_or_else(|_| "unknown".to_string())
                        );
                    }
                    Err(e) => {
                        return Err(eyre!("Failed to update item: {}", e));
                    }
                }
            } else {
                println!(
                    "❌ Item '{}' not found{}.",
                    name,
                    if vault_name.is_some() {
                        format!(" in vault '{}'", vault_name.unwrap())
                    } else {
                        " in any vault".to_string()
                    }
                );

                // Show suggestions from all vaults
                suggest_similar_items_across_vaults(&name)?;

                println!();
                println!("💡 Use 'chamber list' to see items in current vault");
                println!("💡 Use 'chamber registry list' to see all vaults");
                println!("💡 Use 'chamber add' to create a new item");
            }
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

fn is_weak_password(password: &str) -> bool {
    // Strong password criteria:
    // - At least 10 characters
    // - Contains lowercase letter
    // - Contains uppercase letter
    // - Contains number
    // - Contains special character

    if password.len() < 10 {
        return true;
    }

    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_ascii_alphanumeric());

    !(has_lower && has_upper && has_digit && has_special)
}

const fn get_type_emoji(item_type: ItemKind) -> &'static str {
    match item_type {
        ItemKind::Password => "🔑",
        ItemKind::EnvVar => "🌐",
        ItemKind::Note => "📝",
        ItemKind::ApiKey | ItemKind::SshKey => "🔐",
        ItemKind::Certificate => "📜",
        ItemKind::Database => "🗄️",
        ItemKind::CreditCard => "💳",
        ItemKind::SecureNote => "🔒",
        ItemKind::Identity => "🆔",
        ItemKind::Server => "🖥️",
        ItemKind::WifiPassword => "📶",
        ItemKind::License => "📄",
        ItemKind::BankAccount => "🏦",
        ItemKind::Document => "📋",
        ItemKind::Recovery => "🔄",
        ItemKind::OAuth => "🎫",
    }
}

fn prompt_secret(prompt: &str) -> Result<String> {
    use std::io::{Write, stdout};
    print!("{prompt}");
    stdout().flush()?;
    // Read without echo on Windows/Linux/macOS
    let pass = rpassword::prompt_password("")?;
    Ok(pass)
}

/// Searches for an item by name across all available vaults
/// Returns (vault, item, `vault_name`) if found, or (_, None, _) if not found
fn find_item_across_vaults(item_name: &str) -> Result<(Vault, Option<Item>, String)> {
    let manager = VaultManager::new()?;
    let vaults = manager.list_vaults();

    // First try the active vault
    if let Ok(mut vault) = Vault::open_active() {
        if let Ok(Some(vault_id)) = vault.get_vault_id() {
            let master = prompt_secret(&format!(
                "Enter master key for active vault ({}): ",
                get_vault_display_name(&manager, &vault_id)
            ))?;

            if vault.unlock(&master).is_ok() {
                if let Ok(Some(item)) = vault.get_item_by_name(item_name) {
                    let vault_name = get_vault_display_name(&manager, &vault_id);
                    return Ok((vault, Some(item), vault_name));
                }
            }
        }
    }

    // If not found in active vault, search all other vaults
    let mut tried_vaults = Vec::new();

    for vault_info in vaults {
        // Skip if it's the active vault (already tried)
        if let Ok(active_vault) = Vault::open_active() {
            if let Ok(Some(active_id)) = active_vault.get_vault_id() {
                if vault_info.id == active_id {
                    continue;
                }
            }
        }

        println!("🔍 Searching in vault: {} ({})", vault_info.name, vault_info.category);

        let master = prompt_secret(&format!("Enter master key for '{}': ", vault_info.name))?;

        match Vault::open_by_id(&vault_info.id) {
            Ok(mut vault) => match vault.unlock(&master) {
                Ok(()) => {
                    tried_vaults.push(vault_info.name.clone());
                    if let Ok(Some(item)) = vault.get_item_by_name(item_name) {
                        return Ok((vault, Some(item), vault_info.name.clone()));
                    }
                }
                Err(_) => {
                    println!("❌ Failed to unlock vault '{}' (incorrect password?)", vault_info.name);
                }
            },
            Err(e) => {
                println!("❌ Failed to open vault '{}': {}", vault_info.name, e);
            }
        }
    }

    if !tried_vaults.is_empty() {
        println!(
            "🔍 Searched in {} vault(s): {}",
            tried_vaults.len(),
            tried_vaults.join(", ")
        );
    }

    // Return empty result if not found anywhere
    Ok((Vault::open_default()?, None, String::new()))
}

/// Gets a user-friendly display name for a vault
fn get_vault_display_name(manager: &VaultManager, vault_id: &str) -> String {
    if let Some(vault_info) = manager.list_vaults().iter().find(|v| v.id == vault_id) {
        format!("{} ({})", vault_info.name, vault_info.category)
    } else {
        vault_id.to_string()
    }
}

/// Suggests similar items from all vaults
fn suggest_similar_items_across_vaults(item_name: &str) -> Result<()> {
    let manager = VaultManager::new()?;
    let vaults = manager.list_vaults();
    let mut suggestions = Vec::new();

    for vault_info in vaults {
        // Try to open and search each vault (but don't prompt for password for suggestions)
        if let Ok(vault) = Vault::open_by_id(&vault_info.id) {
            // Only suggest from already unlocked vaults to avoid password prompts
            if vault.is_unlocked() {
                if let Ok(items) = vault.list_items() {
                    for item in items {
                        if is_similar_name(&item.name, item_name) {
                            suggestions.push((item.name, item.kind, vault_info.name.clone()));
                        }
                    }
                }
            }
        }
    }

    if !suggestions.is_empty() {
        println!("💡 Did you mean one of these?");
        for (name, kind, vault_name) in suggestions.into_iter().take(5) {
            println!("   • {} [{}] in vault '{}'", name, kind.display_name(), vault_name);
        }
    }

    Ok(())
}

/// Simple fuzzy matching for item names
fn is_similar_name(item_name: &str, search_name: &str) -> bool {
    let item_lower = item_name.to_lowercase();
    let search_lower = search_name.to_lowercase();

    // Check if names contain each other
    if item_lower.contains(&search_lower) || search_lower.contains(&item_lower) {
        return true;
    }

    // Check for common substrings of length 3+
    if search_lower.len() >= 3 {
        search_lower.chars().collect::<Vec<_>>().windows(3).any(|window| {
            let substr: String = window.iter().collect();
            item_lower.contains(&substr)
        })
    } else {
        false
    }
}

fn find_item_in_specific_vault(item_name: &str, vault_identifier: &str) -> Result<(Vault, Option<Item>, String)> {
    let manager = VaultManager::new()?;
    let vaults = manager.list_vaults();

    // Find the vault by name or ID
    let vault_info = vaults
        .iter()
        .find(|v| v.name.eq_ignore_ascii_case(vault_identifier) || v.id == vault_identifier)
        .ok_or_else(|| eyre!("Vault '{}' not found", vault_identifier))?;

    let mut vault = Vault::open_by_id(&vault_info.id)?;
    let master = prompt_secret(&format!("Enter master key for '{}': ", vault_info.name))?;
    vault.unlock(&master)?;

    let item = vault.get_item_by_name(item_name)?;
    Ok((vault, item, vault_info.name.clone()))
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

    #[tokio::test]
    async fn test_generate_simple_ok() {
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
        assert!(res.await.is_ok());
    }

    #[tokio::test]
    async fn test_generate_complex_ok() {
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
        assert!(res.await.is_ok());
    }

    #[tokio::test]
    async fn test_generate_memorable_ok() {
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
        assert!(res.await.is_ok());
    }

    #[tokio::test]
    async fn test_generate_count_multiple_ok() {
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
        assert!(res.await.is_ok());
    }

    #[tokio::test]
    async fn test_generate_custom_no_character_sets_err() {
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
        let err = handle_command(cmd).await.unwrap_err().to_string();
        println!("{err}");
        assert!(err.contains("At least one character set must be enabled"));
    }

    #[tokio::test]
    async fn test_import_missing_file_err() {
        // This hits the early error path for missing an input file without prompting.
        let missing = tmp_path("missing").with_extension("json");
        let cmd = Commands::Import {
            input: missing,
            format: Some("json".to_string()),
            dry_run: false,
            skip_duplicates: false,
        };
        let err = handle_command(cmd).await.unwrap_err().to_string();
        assert!(err.contains("Input file does not exist"));
    }
}
