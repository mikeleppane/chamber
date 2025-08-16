use axum::Json;
use axum::extract::State;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiError, ApiResult};
use crate::models::ApiResponse;
use crate::server::AppState;
use chamber_import_export::{ExportFormat, export_items, import_items};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub format: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub format: String,
    pub path: String,
    #[serde(default)]
    pub filter: Option<ExportFilter>,
}

#[derive(Debug, Deserialize)]
pub struct ExportFilter {
    pub kind: Option<String>,
    pub query: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportResponse {
    pub imported: usize,
    pub skipped: usize,
    pub report: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub count: usize,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct DryRunResponse {
    pub would_import: usize,
    pub would_skip: usize,
    pub conflicts: Vec<String>,
    pub preview: Vec<ItemPreview>,
}

#[derive(Debug, Serialize)]
pub struct ItemPreview {
    pub name: String,
    pub kind: String,
    pub status: String, // "new", "conflict", "skip"
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'write:items' scope
/// - The vault is locked
/// - The import file does not exist
/// - No items are found in the import file
/// - There are issues with vault operations
/// - The import operation fails
pub async fn import_items_handler(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Json(request): Json<ImportRequest>,
) -> ApiResult<Json<ApiResponse<ImportResponse>>> {
    if !claims.has_scope("write:items") {
        return Err(ApiError::Forbidden);
    }

    if !state.auth.is_vault_unlocked() {
        return Err(ApiError::BadRequest("Vault is locked".to_string()));
    }

    let format = parse_export_format(&request.format)?;
    let path = PathBuf::from(&request.path);

    if !path.exists() {
        return Err(ApiError::BadRequest("File does not exist".to_string()));
    }

    // Import items from file
    let new_items = import_items(&path, &format).map_err(|e| ApiError::InternalError(format!("Import failed: {e}")))?;

    if new_items.is_empty() {
        return Err(ApiError::BadRequest("No items found in file".to_string()));
    }

    // Get existing items to check for conflicts
    let mut vault = state.vault.lock().await;
    let existing_items = vault.list_items().map_err(|e| ApiError::VaultError(e.to_string()))?;

    let existing_names: HashSet<String> = existing_items.iter().map(|item| item.name.clone()).collect();

    let mut imported = 0;
    let mut skipped = 0;
    let mut report = Vec::new();

    for item in new_items {
        if existing_names.contains(&item.name) {
            skipped += 1;
            report.push(format!("Skipped '{}': already exists", item.name));
            continue;
        }

        match vault.create_item(&item) {
            Ok(()) => {
                imported += 1;
                report.push(format!("Imported '{}'", item.name));
            }
            Err(e) => {
                skipped += 1;
                report.push(format!("Failed to import '{}': {}", item.name, e));
            }
        }
    }

    let response = ImportResponse {
        imported,
        skipped,
        report,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'read:items' scope
/// - The vault is locked
/// - Failed to create a directory for export
/// - The vault items cannot be listed
/// - The export operation fails
pub async fn export_items_handler(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Json(request): Json<ExportRequest>,
) -> ApiResult<Json<ApiResponse<ExportResponse>>> {
    if !claims.has_scope("read:items") {
        return Err(ApiError::Forbidden);
    }

    if !state.auth.is_vault_unlocked() {
        return Err(ApiError::BadRequest("Vault is locked".to_string()));
    }

    let format = parse_export_format(&request.format)?;
    let path = PathBuf::from(&request.path);

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ApiError::InternalError(format!("Failed to create directory: {e}")))?;
        }
    }

    let mut items = state
        .vault
        .lock()
        .await
        .list_items()
        .map_err(|e| ApiError::VaultError(e.to_string()))?;

    // Apply filters if provided
    if let Some(filter) = &request.filter {
        if let Some(kind) = &filter.kind {
            items.retain(|item| item.kind.as_str().eq_ignore_ascii_case(kind));
        }

        if let Some(query) = &filter.query {
            let query_lower = query.to_lowercase();
            items.retain(|item| {
                item.name.to_lowercase().contains(&query_lower) || item.value.to_lowercase().contains(&query_lower)
            });
        }
    }

    let count = items.len();

    export_items(&items, &format, &path).map_err(|e| ApiError::InternalError(format!("Export failed: {e}")))?;

    let response = ExportResponse {
        count,
        path: path.to_string_lossy().to_string(),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'read:items' scope
/// - The vault is locked
/// - The import file does not exist
/// - There are issues parsing the import file
/// - The vault items cannot be listed
pub async fn dry_run_import(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Json(request): Json<ImportRequest>,
) -> ApiResult<Json<ApiResponse<DryRunResponse>>> {
    if !claims.has_scope("read:items") {
        return Err(ApiError::Forbidden);
    }

    if !state.auth.is_vault_unlocked() {
        return Err(ApiError::BadRequest("Vault is locked".to_string()));
    }

    let format = parse_export_format(&request.format)?;
    let path = PathBuf::from(&request.path);

    if !path.exists() {
        return Err(ApiError::BadRequest("File does not exist".to_string()));
    }

    // Parse items from file
    let new_items = import_items(&path, &format).map_err(|e| ApiError::InternalError(format!("Import failed: {e}")))?;

    // Get existing items to check for conflicts
    let existing_items = state
        .vault
        .lock()
        .await
        .list_items()
        .map_err(|e| ApiError::VaultError(e.to_string()))?;

    let existing_names: HashSet<String> = existing_items.iter().map(|item| item.name.clone()).collect();

    let mut would_import = 0;
    let mut would_skip = 0;
    let mut conflicts = Vec::new();
    let mut preview = Vec::new();

    for item in new_items {
        let preview_item = if existing_names.contains(&item.name) {
            would_skip += 1;
            conflicts.push(item.name.clone());
            ItemPreview {
                name: item.name,
                kind: item.kind.as_str().to_string(),
                status: "conflict".to_string(),
            }
        } else {
            would_import += 1;
            ItemPreview {
                name: item.name,
                kind: item.kind.as_str().to_string(),
                status: "new".to_string(),
            }
        };

        preview.push(preview_item);
    }

    let response = DryRunResponse {
        would_import,
        would_skip,
        conflicts,
        preview,
    };

    Ok(Json(ApiResponse::new(response)))
}

fn parse_export_format(format_str: &str) -> ApiResult<ExportFormat> {
    match format_str.to_lowercase().as_str() {
        "json" => Ok(ExportFormat::Json),
        "csv" => Ok(ExportFormat::Csv),
        "backup" | "chamber" => Ok(ExportFormat::ChamberBackup),
        _ => Err(ApiError::BadRequest(
            "Invalid format. Supported: json, csv, backup".to_string(),
        )),
    }
}
