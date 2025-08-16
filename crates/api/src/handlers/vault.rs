#![allow(clippy::significant_drop_tightening)]

use axum::Json;
use axum::extract::{Path, State};
use std::path::PathBuf;
use std::sync::Arc;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiError, ApiResult};
use crate::models::ApiResponse;
use crate::server::AppState;
use chamber_vault::VaultCategory;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct VaultInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub favorite: bool,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateVaultRequest {
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub master_password: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVaultRequest {
    pub name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub favorite: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteVaultRequest {
    #[serde(default)]
    pub delete_file: bool,
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'read:vaults' or 'read:items' scope
/// - There are issues with the vault manager integration
pub async fn list_vaults(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
) -> ApiResult<Json<ApiResponse<Vec<VaultInfo>>>> {
    if !claims.has_scope("read:vaults") && !claims.has_scope("read:items") {
        return Err(ApiError::Forbidden);
    }

    let vault_manager = state.vault_manager.lock().await;
    let vault_infos = vault_manager.list_vaults();

    let vaults: Vec<VaultInfo> = vault_infos
        .iter()
        .map(|vault_info| VaultInfo {
            id: vault_info.id.clone(),
            name: vault_info.name.clone(),
            category: vault_info.category.to_string(),
            description: vault_info.description.clone(),
            favorite: vault_info.is_favorite,
            is_active: vault_info.is_active,
        })
        .collect();

    Ok(Json(ApiResponse::new(vaults)))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'manage:vaults' scope
/// - There are issues with the vault manager integration
/// - The vault creation operation fails
pub async fn create_vault(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Json(request): Json<CreateVaultRequest>,
) -> ApiResult<Json<ApiResponse<VaultInfo>>> {
    if !claims.has_scope("manage:vaults") {
        return Err(ApiError::Forbidden);
    }

    // Validate input
    if request.name.trim().is_empty() {
        return Err(ApiError::BadRequest("Vault name cannot be empty".to_string()));
    }

    if request.master_password.is_empty() {
        return Err(ApiError::BadRequest("Master password is required".to_string()));
    }

    // Parse category
    let vault_category = match request
        .category
        .as_deref()
        .unwrap_or("personal")
        .to_lowercase()
        .as_str()
    {
        "personal" => VaultCategory::Personal,
        "work" => VaultCategory::Work,
        "team" => VaultCategory::Team,
        "project" => VaultCategory::Project,
        "testing" => VaultCategory::Testing,
        "archive" => VaultCategory::Archive,
        custom => VaultCategory::Custom(custom.to_string()),
    };

    let vault_id = state
        .vault_manager
        .lock()
        .await
        .create_vault(
            request.name.clone(),
            request.path,
            vault_category,
            request.description.clone(),
            &request.master_password,
        )
        .map_err(|e| ApiError::InternalError(format!("Failed to create vault: {e}")))?;

    #[allow(clippy::significant_drop_tightening)]
    let vault_manager = state.vault_manager.lock().await;
    let vaults = vault_manager.list_vaults();
    // Get the created vault info
    let vault_info = vaults
        .iter()
        .find(|v| v.id == vault_id)
        .ok_or_else(|| ApiError::InternalError("Created vault not found".to_string()))?;

    let response = VaultInfo {
        id: vault_info.id.clone(),
        name: vault_info.name.clone(),
        category: vault_info.category.to_string(),
        description: vault_info.description.clone(),
        favorite: vault_info.is_favorite,
        is_active: vault_info.is_active,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'manage:vaults' scope
/// - There are issues with the vault manager integration
/// - The vault switching operation fails
pub async fn switch_vault(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Path(vault_id): Path<String>,
) -> ApiResult<Json<ApiResponse<String>>> {
    if !claims.has_scope("manage:vaults") {
        return Err(ApiError::Forbidden);
    }

    state
        .vault_manager
        .lock()
        .await
        .switch_active_vault(&vault_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to switch vault: {e}")))?;

    Ok(Json(ApiResponse::new(format!("Switched to vault: {vault_id}"))))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'manage:vaults' scope
/// - There are issues with the vault manager integration
/// - The vault update operation fails
pub async fn update_vault(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Path(vault_id): Path<String>,
    Json(request): Json<UpdateVaultRequest>,
) -> ApiResult<Json<ApiResponse<VaultInfo>>> {
    if !claims.has_scope("manage:vaults") {
        return Err(ApiError::Forbidden);
    }

    // Parse category if provided
    let vault_category = request
        .category
        .as_ref()
        .map(|category| match category.to_lowercase().as_str() {
            "personal" => VaultCategory::Personal,
            "work" => VaultCategory::Work,
            "team" => VaultCategory::Team,
            "project" => VaultCategory::Project,
            "testing" => VaultCategory::Testing,
            "archive" => VaultCategory::Archive,
            custom => VaultCategory::Custom(custom.to_string()),
        });

    let mut vault_manager = state.vault_manager.lock().await;
    vault_manager
        .update_vault_info(
            &vault_id,
            request.name.clone(),
            request.description.clone(),
            vault_category,
            request.favorite,
        )
        .map_err(|e| ApiError::InternalError(format!("Failed to update vault: {e}")))?;

    let vaults = vault_manager.list_vaults();
    // Get the updated vault info
    let vault_info = vaults
        .iter()
        .find(|v| v.id == vault_id)
        .ok_or_else(|| ApiError::InternalError("Created vault not found".to_string()))?;

    let response = VaultInfo {
        id: vault_info.id.clone(),
        name: vault_info.name.clone(),
        category: vault_info.category.to_string(),
        description: vault_info.description.clone(),
        favorite: vault_info.is_favorite,
        is_active: vault_info.is_active,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'manage:vaults' scope
/// - There are issues with the vault manager integration
/// - The vault deletion operation fails
pub async fn delete_vault(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Path(vault_id): Path<String>,
    Json(request): Json<DeleteVaultRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    if !claims.has_scope("manage:vaults") {
        return Err(ApiError::Forbidden);
    }

    state
        .vault_manager
        .lock()
        .await
        .delete_vault(&vault_id, request.delete_file)
        .map_err(|e| ApiError::InternalError(format!("Failed to delete vault: {e}")))?;

    Ok(Json(ApiResponse::new(format!("Deleted vault: {vault_id}"))))
}
