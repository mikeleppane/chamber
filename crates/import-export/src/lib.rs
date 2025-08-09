use anyhow::{Result, anyhow};
use chamber_vault::{Item, ItemKind, NewItem};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use time::OffsetDateTime;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Csv,
    ChamberBackup,
}

impl FromStr for ExportFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(ExportFormat::Json),
            "csv" => Ok(ExportFormat::Csv),
            "backup" | "chamber" => Ok(ExportFormat::ChamberBackup),
            _ => Err(anyhow!(
                "Unsupported format: {}. Supported formats: json, csv, backup",
                s
            )),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedItem {
    pub name: String,
    pub kind: String,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChamberBackup {
    pub version: String,
    pub exported_at: String,
    pub item_count: usize,
    pub items: Vec<ExportedItem>,
}

impl From<&Item> for ExportedItem {
    fn from(item: &Item) -> Self {
        Self {
            name: item.name.clone(),
            kind: item.kind.as_str().to_string(),
            value: item.value.clone(),
            created_at: item
                .created_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "unknown".to_string()),
            updated_at: item
                .updated_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "unknown".to_string()),
            notes: None,
        }
    }
}

/// Exports a list of items to a specified file format and writes the output to a given file path.
///
/// # Arguments
///
/// * `items` - A slice of `Item` objects to be exported.
/// * `format` - The export format, represented as an `ExportFormat` enum. Possible formats include:
///     * `ExportFormat::Json` - Exports the data in JSON format.
///     * `ExportFormat::Csv` - Exports the data in CSV format.
///     * `ExportFormat::ChamberBackup` - Exports the data in a custom chamber backup format.
/// * `output_path` - The file path where the exported data will be saved.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the export operation succeeds, or an `Err` if an error occurs.
///
/// # Errors
///
/// This function will return an error in any of the following cases:
/// * The specified `output_path` is invalid or inaccessible.
/// * An I/O error occurs while writing to the file.
/// * Serialization to the chosen export format fails.
///
/// # Examples
///
/// ```
/// let items = vec![Item::new("Item1"), Item::new("Item2")];
/// let output_path = Path::new("output.json");
/// let result = export_items(&items, &ExportFormat::Json, output_path);
///
/// if result.is_err() {
///     eprintln!("Failed to export items");
/// }
/// ```
///
/// # Note
///
/// Ensure that the directory specified in `output_path` exists and has write permissions before calling this function.
pub fn export_items(items: &[Item], format: &ExportFormat, output_path: &Path) -> Result<()> {
    match format {
        ExportFormat::Json => export_json(items, output_path),
        ExportFormat::Csv => export_csv(items, output_path),
        ExportFormat::ChamberBackup => export_chamber_backup(items, output_path),
    }
}

fn export_json(items: &[Item], output_path: &Path) -> Result<()> {
    let exported_items: Vec<ExportedItem> = items.iter().map(ExportedItem::from).collect();
    let json = serde_json::to_string_pretty(&exported_items)?;
    fs::write(output_path, json)?;
    Ok(())
}

fn export_csv(items: &[Item], output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(output_path)?;
    writeln!(file, "name,kind,value,created_at,updated_at")?;

    for item in items {
        let exported = ExportedItem::from(item);
        let name = escape_csv_field(&exported.name);
        let kind = escape_csv_field(&exported.kind);
        let value = escape_csv_field(&exported.value);
        let created = escape_csv_field(&exported.created_at);
        let updated = escape_csv_field(&exported.updated_at);

        writeln!(file, "{name},{kind},{value},{created},{updated}")?;
    }
    Ok(())
}

fn export_chamber_backup(items: &[Item], output_path: &Path) -> Result<()> {
    let backup = ChamberBackup {
        version: "1.0".to_string(),
        exported_at: OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string()),
        item_count: items.len(),
        items: items.iter().map(ExportedItem::from).collect(),
    };

    let json = serde_json::to_string_pretty(&backup)?;
    fs::write(output_path, json)?;
    Ok(())
}

/// Imports items from a specified file path and format.
///
/// # Arguments
///
/// * `input_path` - A reference to a `Path` that specifies the location of the file to be imported.
/// * `format` - The format of the file to be imported, which can be one of `ExportFormat::Json`,
///   `ExportFormat::Csv`, or `ExportFormat::ChamberBackup`.
///
/// # Returns
///
/// This function returns a `Result` containing:
/// * `Ok(Vec<NewItem>)` - A vector of `NewItem` if the import is successful.
/// * `Err` - An error if there is an issue with importing the items.
///
/// # Behavior
///
/// The function determines the appropriate import operation to execute based on the provided file
/// format:
/// * `ExportFormat::Json` - Calls the `import_json` function to handle JSON files.
/// * `ExportFormat::Csv` - Calls the `import_csv` function to handle CSV files.
/// * `ExportFormat::ChamberBackup` - Calls the `import_chamber_backup` function to handle Chamber
///   Backup files.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use my_crate::{import_items, ExportFormat};
///
/// let path = Path::new("items.json");
/// let format = ExportFormat::Json;
///
/// let result = import_items(path, &format);
/// match result {
///     Ok(items) => println!("Successfully imported {} items.", items.len()),
///     Err(e) => eprintln!("Failed to import items: {:?}", e),
/// }
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// * The file at `input_path` cannot be read.
/// * The file format is invalid or corrupted for the specified `ExportFormat`.
/// * Any other internal errors occur during the import process.
pub fn import_items(input_path: &Path, format: &ExportFormat) -> Result<Vec<NewItem>> {
    match format {
        ExportFormat::Json => import_json(input_path),
        ExportFormat::Csv => import_csv(input_path),
        ExportFormat::ChamberBackup => import_chamber_backup(input_path),
    }
}

fn import_json(input_path: &Path) -> Result<Vec<NewItem>> {
    let content = fs::read_to_string(input_path)?;
    let exported_items: Vec<ExportedItem> =
        serde_json::from_str(&content).map_err(|e| anyhow!("JSON parse error: {e}"))?;

    let mut items = Vec::new();
    for exported in exported_items {
        items.push(NewItem {
            name: exported.name,
            kind: ItemKind::from_str(&exported.kind)?,
            value: exported.value,
        });
    }

    Ok(items)
}

// Rust
fn record_complete(line: &str) -> bool {
    // Returns true if the line ends outside of quotes (i.e., unescaped quotes are balanced)
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            if in_quotes {
                // Escaped quote?
                if chars.peek() == Some(&'"') {
                    // consume the second quote and keep in quotes
                    chars.next();
                } else {
                    // closing quote
                    in_quotes = false;
                }
            } else {
                // opening quote
                in_quotes = true;
            }
        }
    }
    !in_quotes
}

fn import_csv(input_path: &Path) -> Result<Vec<NewItem>> {
    let content = fs::read_to_string(input_path)?;
    let mut lines = content.lines();

    // Handle header
    let Some(header) = lines.next() else {
        return Ok(Vec::new());
    };
    // Optional: validate header minimally (not strictly required)
    if header.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    let mut buf = String::new();
    // Track logical CSV line numbers for error reporting:
    // header is line 1; data records start from logical line 2
    let mut current_record_start_line: usize = 2;
    let mut physical_line_index: usize = 1; // already consumed header

    for raw in lines {
        physical_line_index += 1;
        if buf.is_empty() {
            buf.push_str(raw);
            current_record_start_line = physical_line_index;
        } else {
            buf.push('\n');
            buf.push_str(raw);
        }

        if !record_complete(&buf) {
            // Need more physical lines to complete a record
            continue;
        }

        if buf.trim().is_empty() {
            buf.clear();
            continue;
        }

        let fields = parse_csv_line(&buf);
        if fields.len() < 3 {
            return Err(anyhow!(
                "Invalid CSV format at line {}: expected at least 3 fields",
                current_record_start_line
            ));
        }

        items.push(NewItem {
            name: fields[0].clone(),
            kind: ItemKind::from_str(&fields[1])?,
            value: fields[2].clone(),
        });

        buf.clear();
    }

    // If leftover buffer remains, ensure it's a complete record and parse it
    if !buf.is_empty() {
        if !record_complete(&buf) {
            return Err(anyhow!(
                "Invalid CSV format at line {}: unterminated quoted field",
                current_record_start_line
            ));
        }
        let fields = parse_csv_line(&buf);
        if fields.len() < 3 {
            return Err(anyhow!(
                "Invalid CSV format at line {}: expected at least 3 fields",
                current_record_start_line
            ));
        }
        items.push(NewItem {
            name: fields[0].clone(),
            kind: ItemKind::from_str(&fields[1])?,
            value: fields[2].clone(),
        });
    }

    Ok(items)
}

fn import_chamber_backup(input_path: &Path) -> Result<Vec<NewItem>> {
    let content = fs::read_to_string(input_path)?;
    let backup: ChamberBackup = serde_json::from_str(&content).map_err(|e| anyhow!("JSON parse error: {e}"))?;

    let mut items = Vec::new();
    for exported in backup.items {
        items.push(NewItem {
            name: exported.name,
            kind: ItemKind::from_str(&exported.kind)?,
            value: exported.value,
        });
    }

    Ok(items)
}

// Helper functions for CSV handling
fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes {
                    // Check if this is an escaped quote
                    if chars.peek() == Some(&'"') {
                        current_field.push('"');
                        chars.next(); // consume the second quote
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            ',' if !in_quotes => {
                fields.push(current_field.trim().to_string());
                current_field.clear();
            }
            _ => {
                current_field.push(ch);
            }
        }
    }

    // Add the last field
    fields.push(current_field.trim().to_string());

    fields
}

// Additional import format support
#[must_use]
pub fn detect_format_from_extension(path: &Path) -> Option<ExportFormat> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext.to_lowercase().as_str() {
            "json" => {
                // Try to detect if it's a chamber backup by checking the filename
                if path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains("chamber") || name.contains("backup"))
                {
                    Some(ExportFormat::ChamberBackup)
                } else {
                    Some(ExportFormat::Json)
                }
            }
            "csv" => Some(ExportFormat::Csv),
            _ => None,
        })
}

// Rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use chamber_vault::{Item, ItemKind};
    use std::{fs, path::PathBuf};
    use time::OffsetDateTime;

    fn unique_path(ext: &str) -> PathBuf {
        let now = OffsetDateTime::now_utc().unix_timestamp_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("chamber_ie_test_{pid}_{now}.{ext}"))
    }

    fn mk_item(id: i64, name: &str, kind: ItemKind, value: &str) -> Item {
        let now = OffsetDateTime::now_utc();
        Item {
            id,
            name: name.to_string(),
            kind,
            value: value.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_items() -> Vec<Item> {
        vec![
            mk_item(1, "alpha", ItemKind::Password, "secret-α"),
            mk_item(2, "beta", ItemKind::EnvVar, "VALUE=1,2,3"),
            mk_item(3, "note,with,commas", ItemKind::Note, "Hello, \"World\""),
            mk_item(
                4,
                "multiline",
                ItemKind::ApiKey,
                "line1\nline2\nline3 with , and \"quotes\"",
            ),
        ]
    }

    #[test]
    fn test_exportformat_from_str() {
        assert!(matches!(ExportFormat::from_str("json").unwrap(), ExportFormat::Json));
        assert!(matches!(ExportFormat::from_str("csv").unwrap(), ExportFormat::Csv));
        assert!(matches!(
            ExportFormat::from_str("backup").unwrap(),
            ExportFormat::ChamberBackup
        ));
        assert!(matches!(
            ExportFormat::from_str("chamber").unwrap(),
            ExportFormat::ChamberBackup
        ));

        let err = ExportFormat::from_str("unknown").unwrap_err().to_string();
        assert!(err.contains("Unsupported format"));
        assert!(err.contains("json"));
        assert!(err.contains("csv"));
        assert!(err.contains("backup"));
    }

    #[test]
    fn test_detect_format_from_extension() {
        let json = Path::new("data/export.json");
        let csv = Path::new("data/export.CSV");
        let backup1 = Path::new("backup/chamber_backup.json");
        let backup2 = Path::new("backup/2024-12-01-backup.JSON");
        let unknown = Path::new("data/file.txt");

        assert!(matches!(detect_format_from_extension(json), Some(ExportFormat::Json)));
        assert!(matches!(detect_format_from_extension(csv), Some(ExportFormat::Csv)));
        assert!(matches!(
            detect_format_from_extension(backup1),
            Some(ExportFormat::ChamberBackup)
        ));
        assert!(matches!(
            detect_format_from_extension(backup2),
            Some(ExportFormat::ChamberBackup)
        ));
        assert!(detect_format_from_extension(unknown).is_none());
    }

    #[test]
    fn test_json_round_trip() {
        let items = sample_items();
        let path = unique_path("json");

        export_items(&items, &ExportFormat::Json, &path).unwrap();

        let imported = import_items(&path, &ExportFormat::Json).unwrap();
        fs::remove_file(&path).ok();

        // Only name/kind/value are imported (timestamps are not round-tripped)
        assert_eq!(imported.len(), items.len());
        for (i, ni) in imported.iter().enumerate() {
            assert_eq!(ni.name, items[i].name);
            assert_eq!(ni.kind.as_str(), items[i].kind.as_str());
            assert_eq!(ni.value, items[i].value);
        }
    }

    #[test]
    fn test_csv_round_trip_with_escaping() {
        let items = sample_items();
        let path = unique_path("csv");

        export_items(&items, &ExportFormat::Csv, &path).unwrap();

        let imported = import_items(&path, &ExportFormat::Csv).unwrap();
        fs::remove_file(&path).ok();

        // CSV import uses only the first three fields (name, kind, value)
        assert_eq!(imported.len(), items.len());
        for (i, ni) in imported.iter().enumerate() {
            assert_eq!(ni.name, items[i].name);
            assert_eq!(ni.kind.as_str(), items[i].kind.as_str());
            assert_eq!(ni.value, items[i].value);
        }
    }

    #[test]
    fn test_chamber_backup_round_trip() {
        let items = sample_items();
        let path = unique_path("json"); // backup uses JSON extension

        export_items(&items, &ExportFormat::ChamberBackup, &path).unwrap();

        let imported = import_items(&path, &ExportFormat::ChamberBackup).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(imported.len(), items.len());
        for (i, ni) in imported.iter().enumerate() {
            assert_eq!(ni.name, items[i].name);
            assert_eq!(ni.kind.as_str(), items[i].kind.as_str());
            assert_eq!(ni.value, items[i].value);
        }
    }

    #[test]
    fn test_import_csv_empty_file() {
        let path = unique_path("csv");
        fs::write(&path, "").unwrap();

        let imported = import_items(&path, &ExportFormat::Csv).unwrap();
        fs::remove_file(&path).ok();

        assert!(imported.is_empty());
    }

    #[test]
    fn test_import_csv_invalid_line() {
        let path = unique_path("csv");
        // Header + a malformed line with only two fields
        let content = "name,kind,value,created_at,updated_at\nbadline,password\n";
        fs::write(&path, content).unwrap();

        let err = import_items(&path, &ExportFormat::Csv).unwrap_err().to_string();
        fs::remove_file(&path).ok();

        assert!(err.contains("Invalid CSV format"));
        assert!(err.contains("expected at least 3 fields"));
    }

    #[test]
    fn test_import_json_invalid() {
        let path = unique_path("json");
        fs::write(&path, "{not valid json").unwrap();

        let err = import_items(&path, &ExportFormat::Json).unwrap_err().to_string();
        fs::remove_file(&path).ok();

        // serde_json error should bubble up
        assert!(err.to_lowercase().contains("json"));
    }

    #[test]
    fn test_import_chamber_backup_invalid() {
        let path = unique_path("json");
        // Not a valid ChamberBackup shape
        fs::write(&path, r#"{"foo":"bar"}"#).unwrap();

        let err = import_items(&path, &ExportFormat::ChamberBackup)
            .unwrap_err()
            .to_string();
        fs::remove_file(&path).ok();

        assert!(err.to_lowercase().contains("json"));
    }

    #[test]
    fn test_dispatch_import_items() {
        let items = sample_items();

        // JSON
        let path_json = unique_path("json");
        export_items(&items, &ExportFormat::Json, &path_json).unwrap();
        let j = import_items(&path_json, &ExportFormat::Json).unwrap();
        assert_eq!(j.len(), items.len());
        fs::remove_file(&path_json).ok();

        // CSV
        let path_csv = unique_path("csv");
        export_items(&items, &ExportFormat::Csv, &path_csv).unwrap();
        let c = import_items(&path_csv, &ExportFormat::Csv).unwrap();
        assert_eq!(c.len(), items.len());
        fs::remove_file(&path_csv).ok();

        // Backup
        let path_bak = unique_path("json");
        export_items(&items, &ExportFormat::ChamberBackup, &path_bak).unwrap();
        let b = import_items(&path_bak, &ExportFormat::ChamberBackup).unwrap();
        assert_eq!(b.len(), items.len());
        fs::remove_file(&path_bak).ok();
    }

    #[test]
    fn test_export_creates_parent_directories_csv() {
        let items = sample_items();
        let mut dir = unique_path("dir");
        // Ensure unique nested directory
        dir.set_extension("");
        let nested = dir.join("nested").join("export.csv");

        // Export_csv should create a Parent
        export_items(&items, &ExportFormat::Csv, &nested).unwrap();
        assert!(nested.exists());

        // Cleanup
        fs::remove_file(&nested).ok();
        // Remove dirs from deepest to top
        if let Some(parent) = nested.parent() {
            fs::remove_dir_all(parent.parent().unwrap_or(parent)).ok();
        }
    }
}
