use anyhow::{Result, anyhow};
use chamber_import_export::{ExportFormat, detect_format_from_extension, export_items, import_items};
use chamber_password_gen::{
    PasswordConfig, generate_complex_password, generate_memorable_password, generate_simple_password,
};
use chamber_vault::{ItemKind, NewItem, Vault};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(name = "chamber", about = "A secure TUI/CLI secrets manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Init,
    Add {
        #[arg(short, long)]
        name: String,
        #[arg(short, long, default_value = "note")]
        kind: String,
        #[arg(short, long)]
        value: String,
    },
    List,
    Get {
        #[arg(short, long)]
        name: String,
    },
    Generate {
        #[arg(short, long, default_value = "16")]
        length: usize,
        #[arg(long)]
        simple: bool,
        #[arg(long)]
        complex: bool,
        #[arg(long)]
        memorable: bool,
        #[arg(long)]
        no_uppercase: bool,
        #[arg(long)]
        no_lowercase: bool,
        #[arg(long)]
        no_digits: bool,
        #[arg(long)]
        no_symbols: bool,
        #[arg(long)]
        include_ambiguous: bool,
        #[arg(short, long)]
        count: Option<usize>,
    },
    Export {
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long)]
        format: Option<String>,
        #[arg(long)]
        include_metadata: bool,
    },
    Import {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        format: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        skip_duplicates: bool,
    },
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
