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
    #![allow(clippy::cast_possible_wrap)]
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

    #[test]
    fn test_csv_with_unterminated_quotes() {
        let path = unique_path("csv");
        let content = "name,kind,value\n\"unterminated,password,secret";
        fs::write(&path, content).unwrap();

        let err = import_items(&path, &ExportFormat::Csv).unwrap_err();
        fs::remove_file(&path).ok();

        assert!(err.to_string().contains("unterminated quoted field"));
    }

    #[test]
    fn test_csv_multiline_records() {
        let path = unique_path("csv");
        let content = "name,kind,value\n\"multi\nline\nname\",password,\"multi\nline\nvalue\"";
        fs::write(&path, content).unwrap();

        let imported = import_items(&path, &ExportFormat::Csv).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].name, "multi\nline\nname");
        assert_eq!(imported[0].value, "multi\nline\nvalue");
    }

    #[test]
    fn test_csv_with_only_headers() {
        let path = unique_path("csv");
        fs::write(&path, "name,kind,value,created_at,updated_at\n").unwrap();

        let imported = import_items(&path, &ExportFormat::Csv).unwrap();
        fs::remove_file(&path).ok();

        assert!(imported.is_empty());
    }

    #[test]
    fn test_csv_with_extra_fields() {
        let path = unique_path("csv");
        let content = "name,kind,value,extra1,extra2,extra3\ntest,password,secret,field4,field5,field6";
        fs::write(&path, content).unwrap();

        let imported = import_items(&path, &ExportFormat::Csv).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].name, "test");
        assert_eq!(imported[0].value, "secret");
    }

    #[test]
    fn test_csv_with_empty_fields() {
        let path = unique_path("csv");
        let content = "name,kind,value\n,password,\nempty_name,note,";
        fs::write(&path, content).unwrap();

        let imported = import_items(&path, &ExportFormat::Csv).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].name, "");
        assert_eq!(imported[0].value, "");
        assert_eq!(imported[1].name, "empty_name");
        assert_eq!(imported[1].value, "");
    }

    #[test]
    fn test_csv_with_whitespace_handling() {
        let path = unique_path("csv");
        let content = "name,kind,value\n  spaced name  ,  password  ,  spaced value  ";
        fs::write(&path, content).unwrap();

        let imported = import_items(&path, &ExportFormat::Csv).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].name, "spaced name");
        assert_eq!(imported[0].value, "spaced value");
    }

    #[test]
    fn test_import_nonexistent_file() {
        // Generate a path that definitely doesn't exist
        let nonexistent = {
            let mut path = std::env::temp_dir();
            path.push("chamber_test_nonexistent");
            path.push("deeply");
            path.push("nested");
            path.push("path");
            path.push("file.json");
            path
        };

        // Ensure the path doesn't exist
        assert!(!nonexistent.exists());

        // Test that import fails for all formats
        for format in [ExportFormat::Json, ExportFormat::Csv, ExportFormat::ChamberBackup] {
            let result = import_items(&nonexistent, &format);
            assert!(
                result.is_err(),
                "Import should fail for nonexistent file with format {format:?}"
            );
        }
    }

    #[test]
    fn test_chamber_backup_with_zero_items() {
        let path = unique_path("json");
        export_items(&[], &ExportFormat::ChamberBackup, &path).unwrap();

        // Verify the backup structure
        let content = fs::read_to_string(&path).unwrap();
        let backup: ChamberBackup = serde_json::from_str(&content).unwrap();

        assert_eq!(backup.version, "1.0");
        assert_eq!(backup.item_count, 0);
        assert!(backup.items.is_empty());

        let imported = import_items(&path, &ExportFormat::ChamberBackup).unwrap();
        fs::remove_file(&path).ok();

        assert!(imported.is_empty());
    }

    #[test]
    fn test_csv_with_unicode_content() {
        let items = vec![
            mk_item(1, "🔒 密码", ItemKind::Password, "测试密码123"),
            mk_item(2, "café", ItemKind::Note, "Héllö Wörld! 🌍"),
            mk_item(3, "русский", ItemKind::EnvVar, "значение=тест"),
        ];

        let path = unique_path("csv");
        export_items(&items, &ExportFormat::Csv, &path).unwrap();

        let imported = import_items(&path, &ExportFormat::Csv).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(imported.len(), 3);
        assert_eq!(imported[0].name, "🔒 密码");
        assert_eq!(imported[0].value, "测试密码123");
        assert_eq!(imported[1].name, "café");
        assert_eq!(imported[1].value, "Héllö Wörld! 🌍");
        assert_eq!(imported[2].name, "русский");
        assert_eq!(imported[2].value, "значение=тест");
    }

    #[test]
    fn test_export_to_readonly_directory() {
        // Create a temporary directory and make it read-only
        let temp_dir = std::env::temp_dir().join("readonly_test");
        fs::create_dir_all(&temp_dir).ok();

        // Try to make the directory read-only (this might not work on all systems)
        let _ = temp_dir.join("test.json");

        // On Windows, we can't easily make directories read-only, so we'll
        // test a different scenario - trying to write to a file that's a directory
        let dir_as_file = temp_dir.join("not_a_file");
        fs::create_dir_all(&dir_as_file).ok();

        let items = sample_items();
        let result = export_items(&items, &ExportFormat::Json, &dir_as_file);

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();

        // Should fail because we're trying to write to a directory
        assert!(result.is_err());
    }

    #[test]
    fn test_all_itemkind_variants_round_trip() {
        let all_kinds = [
            ItemKind::Password,
            ItemKind::EnvVar,
            ItemKind::Note,
            ItemKind::ApiKey,
            ItemKind::SshKey,
            ItemKind::Certificate,
            ItemKind::Database,
        ];

        let items: Vec<Item> = all_kinds
            .iter()
            .enumerate()
            .map(|(i, &kind)| mk_item(i as i64 + 1, &format!("item_{}", kind.as_str()), kind, "test_value"))
            .collect();

        // Test JSON round trip
        let json_path = unique_path("json");
        export_items(&items, &ExportFormat::Json, &json_path).unwrap();
        let json_imported = import_items(&json_path, &ExportFormat::Json).unwrap();
        fs::remove_file(&json_path).ok();

        // Test CSV round trip
        let csv_path = unique_path("csv");
        export_items(&items, &ExportFormat::Csv, &csv_path).unwrap();
        let csv_imported = import_items(&csv_path, &ExportFormat::Csv).unwrap();
        fs::remove_file(&csv_path).ok();

        // Test Chamber backup round trip
        let backup_path = unique_path("json");
        export_items(&items, &ExportFormat::ChamberBackup, &backup_path).unwrap();
        let backup_imported = import_items(&backup_path, &ExportFormat::ChamberBackup).unwrap();
        fs::remove_file(&backup_path).ok();

        // Verify all formats preserved all kinds
        for imported in [&json_imported, &csv_imported, &backup_imported] {
            assert_eq!(imported.len(), all_kinds.len());
            for (i, item) in imported.iter().enumerate() {
                assert_eq!(item.kind, all_kinds[i]);
            }
        }
    }

    #[test]
    fn test_csv_with_embedded_newlines() {
        let items = vec![
            mk_item(1, "line1\nline2", ItemKind::Note, "value1\nvalue2\nvalue3"),
            mk_item(2, "normal", ItemKind::Password, "no newlines"),
        ];

        let path = unique_path("csv");
        export_items(&items, &ExportFormat::Csv, &path).unwrap();

        let imported = import_items(&path, &ExportFormat::Csv).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].name, "line1\nline2");
        assert_eq!(imported[0].value, "value1\nvalue2\nvalue3");
        assert_eq!(imported[1].name, "normal");
        assert_eq!(imported[1].value, "no newlines");
    }

    #[test]
    fn test_chamber_backup_metadata_validation() {
        let items = sample_items();
        let path = unique_path("json");

        export_items(&items, &ExportFormat::ChamberBackup, &path).unwrap();

        // Read and verify the backup structure
        let content = fs::read_to_string(&path).unwrap();
        let backup: ChamberBackup = serde_json::from_str(&content).unwrap();

        assert_eq!(backup.version, "1.0");
        assert_eq!(backup.item_count, items.len());
        assert_eq!(backup.items.len(), items.len());
        assert!(!backup.exported_at.is_empty());

        // Verify timestamp format
        let _: OffsetDateTime =
            OffsetDateTime::parse(&backup.exported_at, &time::format_description::well_known::Rfc3339)
                .expect("exported_at should be valid RFC3339 timestamp");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_format_detection_edge_cases() {
        // Test files without extensions
        assert!(detect_format_from_extension(Path::new("noext")).is_none());

        // Test empty extension
        assert!(detect_format_from_extension(Path::new("file.")).is_none());

        // Test case variations
        assert!(matches!(
            detect_format_from_extension(Path::new("FILE.JSON")),
            Some(ExportFormat::Json)
        ));
        assert!(matches!(
            detect_format_from_extension(Path::new("file.Csv")),
            Some(ExportFormat::Csv)
        ));

        // Test backup detection heuristics
        assert!(matches!(
            detect_format_from_extension(Path::new("chamber_export.json")),
            Some(ExportFormat::ChamberBackup)
        ));
        assert!(matches!(
            detect_format_from_extension(Path::new("my_backup_file.json")),
            Some(ExportFormat::ChamberBackup)
        ));
        assert!(matches!(
            detect_format_from_extension(Path::new("regular_data.json")),
            Some(ExportFormat::Json)
        ));

        // Test complex paths
        assert!(matches!(
            detect_format_from_extension(Path::new("/path/to/chamber_backup_2024.json")),
            Some(ExportFormat::ChamberBackup)
        ));
    }

    #[test]
    fn test_very_large_field_values() {
        let large_value = "x".repeat(10_000); // 10KB string
        let large_name = "n".repeat(1000); // 1KB name

        let items = vec![
            mk_item(1, &large_name, ItemKind::Note, &large_value),
            mk_item(2, "normal", ItemKind::Password, "small"),
        ];

        // Test with all formats
        for (format, ext) in [
            (ExportFormat::Json, "json"),
            (ExportFormat::Csv, "csv"),
            (ExportFormat::ChamberBackup, "json"),
        ] {
            let path = unique_path(ext);
            export_items(&items, &format, &path).unwrap();

            let imported = import_items(&path, &format).unwrap();
            fs::remove_file(&path).ok();

            assert_eq!(imported.len(), 2);
            assert_eq!(imported[0].name, large_name);
            assert_eq!(imported[0].value, large_value);
            assert_eq!(imported[1].name, "normal");
            assert_eq!(imported[1].value, "small");
        }
    }

    #[test]
    fn test_exported_item_timestamp_fallback() {
        // Create an item with a timestamp that might cause formatting issues
        let far_future = OffsetDateTime::from_unix_timestamp(253_402_300_799).unwrap_or(OffsetDateTime::now_utc()); // Year 9999

        let item = Item {
            id: 1,
            name: "test".to_string(),
            kind: ItemKind::Password,
            value: "value".to_string(),
            created_at: far_future,
            updated_at: far_future,
        };

        let exported = ExportedItem::from(&item);

        // Should either have a valid timestamp or fallback to "unknown"
        assert!(!exported.created_at.is_empty());
        assert!(!exported.updated_at.is_empty());
    }

    #[test]
    fn test_csv_parsing_complex_quoting() {
        let path = unique_path("csv");
        // Complex CSV with nested quotes and commas
        let content = r#"name,kind,value
"item ""with"" quotes",password,"value, with ""quotes"" and, commas"
simple,note,no_quotes
"trailing comma,",envvar,"value,"
"#;
        fs::write(&path, content).unwrap();

        let imported = import_items(&path, &ExportFormat::Csv).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(imported.len(), 3);
        assert_eq!(imported[0].name, r#"item "with" quotes"#);
        assert_eq!(imported[0].value, r#"value, with "quotes" and, commas"#);
        assert_eq!(imported[1].name, "simple");
        assert_eq!(imported[2].name, "trailing comma,");
        assert_eq!(imported[2].value, "value,");
    }

    #[test]
    fn test_json_empty_array() {
        let path = unique_path("json");
        fs::write(&path, "[]").unwrap();

        let imported = import_items(&path, &ExportFormat::Json).unwrap();
        fs::remove_file(&path).ok();

        assert!(imported.is_empty());
    }

    #[test]
    fn test_invalid_itemkind_conversion() {
        let path = unique_path("json");
        let invalid_json = r#"[{"name":"test","kind":"invalid_kind","value":"test","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}]"#;
        fs::write(&path, invalid_json).unwrap();

        let imported = import_items(&path, &ExportFormat::Json);
        fs::remove_file(&path).ok();

        assert!(imported.is_err());
    }

    #[test]
    fn test_json_parse_error_missing_required_fields() {
        let path = unique_path("json");
        // Missing required fields like "name" or "value"
        let invalid_json = r#"[{"kind":"password","created_at":"2024-01-01T00:00:00Z"}]"#;
        fs::write(&path, invalid_json).unwrap();

        let err = import_items(&path, &ExportFormat::Json).unwrap_err();
        fs::remove_file(&path).ok();

        // Should fail due to missing required fields during deserialization
        let error_msg = err.to_string().to_lowercase();
        assert!(error_msg.contains("json") || error_msg.contains("missing") || error_msg.contains("error"));
    }

    #[test]
    fn test_nested_directory_creation_all_formats() {
        let base_dir = std::env::temp_dir().join(format!("chamber_test_{}", std::process::id()));
        let items = vec![mk_item(1, "test", ItemKind::Password, "value")];

        for (format, ext) in [
            (ExportFormat::Json, "json"),
            (ExportFormat::Csv, "csv"),
            (ExportFormat::ChamberBackup, "json"),
        ] {
            let nested_path = base_dir
                .join(format!("{format:?}"))
                .join("level1")
                .join("level2")
                .join(format!("export.{ext}"));

            // Ensure the parent directory exists for formats that don't create it automatically
            if let Some(parent) = nested_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            export_items(&items, &format, &nested_path).unwrap();
            assert!(nested_path.exists());

            let imported = import_items(&nested_path, &format).unwrap();
            assert_eq!(imported.len(), 1);
        }

        // Cleanup
        fs::remove_dir_all(&base_dir).ok();
    }

    #[test]
    fn test_csv_escape_field_function() {
        // Test the escape_csv_field function directly through export behavior
        let items = vec![
            mk_item(1, "no_escaping_needed", ItemKind::Password, "simple_value"),
            mk_item(2, "has,comma", ItemKind::Password, "has\"quote"),
            mk_item(3, "has\nnewline", ItemKind::Password, "normal"),
            mk_item(4, "has\"quote", ItemKind::Password, "has,comma"),
        ];

        let path = unique_path("csv");
        export_items(&items, &ExportFormat::Csv, &path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        fs::remove_file(&path).ok();

        // Verify escaping behavior in the CSV content
        assert!(content.contains("\"has,comma\"")); // Comma escaped
        assert!(content.contains("\"has\"\"quote\"")); // Quote escaped
        assert!(content.contains("\"has\nnewline\"")); // Newline escaped
        assert!(content.contains("no_escaping_needed")); // No escaping needed
    }

    #[test]
    fn test_performance_with_many_items() {
        // Test with a reasonable number of items (not too many to slow down tests)
        let items: Vec<Item> = (0..100)
            .map(|i| mk_item(i, &format!("item_{i}"), ItemKind::Password, &format!("value_{i}")))
            .collect();

        for format in [ExportFormat::Json, ExportFormat::Csv, ExportFormat::ChamberBackup] {
            let path = unique_path(match format {
                ExportFormat::Csv => "csv",
                _ => "json",
            });

            let start = std::time::Instant::now();
            export_items(&items, &format, &path).unwrap();
            let export_duration = start.elapsed();

            let start = std::time::Instant::now();
            let imported = import_items(&path, &format).unwrap();
            let import_duration = start.elapsed();

            fs::remove_file(&path).ok();

            assert_eq!(imported.len(), 100);

            // Should complete within reasonable time (adjust if needed)
            assert!(
                export_duration.as_secs() < 5,
                "Export took too long: {export_duration:?}"
            );
            assert!(
                import_duration.as_secs() < 5,
                "Import took too long: {import_duration:?}"
            );
        }
    }
}
