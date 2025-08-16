use crate::auth::AuthenticatedUser;
use crate::models::{ApiResponse, HealthResponse};
use crate::server::AppState;
use crate::{ApiError, ApiResult, HealthReportResponse, OldPasswordItem, ReusedPasswordGroup, StatsResponse};
use axum::Json;
use axum::extract::State;
use chamber_vault::ItemKind;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

/// # Errors
///
/// This function returns an error if there are issues accessing the vault state
pub async fn health(State(state): State<Arc<AppState>>) -> ApiResult<Json<ApiResponse<HealthResponse>>> {
    let vault_status = if state.auth.is_vault_unlocked() {
        "unlocked".to_string()
    } else {
        "locked".to_string()
    };

    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        vault_status,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'vault:health' scope
/// - The vault is locked
/// - There are issues accessing or listing vault items
pub async fn health_report(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
) -> ApiResult<Json<ApiResponse<HealthReportResponse>>> {
    // Check if user has permission to view health reports
    if !claims.has_scope("vault:health") {
        return Err(ApiError::Forbidden);
    }

    if !state.auth.is_vault_unlocked() {
        return Err(ApiError::BadRequest("Vault is locked".to_string()));
    }

    let items = state
        .vault
        .lock()
        .await
        .list_items()
        .map_err(|e| ApiError::VaultError(e.to_string()))?;

    let total_items = items.len();
    let password_items: Vec<_> = items
        .iter()
        .filter(|item| item.kind.as_str() == "password" || item.kind.as_str() == "login")
        .collect();

    // Analyze weak passwords (less than 8 characters or common patterns)
    let weak_passwords: Vec<String> = password_items
        .iter()
        .filter(|item| {
            let value = &item.value;
            value.len() < 8
                || value.chars().all(char::is_numeric)
                || value.to_lowercase().contains("password")
                || value.to_lowercase().contains("123")
        })
        .map(|item| item.name.clone())
        .collect();

    // Analyze reused passwords
    let mut password_groups: HashMap<String, Vec<String>> = HashMap::new();
    for item in &password_items {
        // Use a simple hash of the password value (in production, use proper hashing)
        let hash = format!("{:x}", md5::compute(&item.value));
        password_groups.entry(hash).or_default().push(item.name.clone());
    }

    let reused_passwords: Vec<ReusedPasswordGroup> = password_groups
        .into_iter()
        .filter(|(_, names)| names.len() > 1)
        .map(|(hash, names)| ReusedPasswordGroup {
            password_hash: hash,
            item_names: names,
        })
        .collect();

    // Analyze old passwords (older than 90 days)
    let now = Utc::now();
    let old_passwords: Vec<OldPasswordItem> = password_items
        .iter()
        .filter_map(|item| {
            let days_old = (now.timestamp() - item.updated_at.unix_timestamp()) / (24 * 60 * 60);
            if days_old > 90 {
                Some(OldPasswordItem {
                    item_name: item.name.clone(),
                    days_old,
                })
            } else {
                None
            }
        })
        .collect();

    // Analyze short passwords (less than 12 characters)
    let short_passwords: Vec<String> = password_items
        .iter()
        .filter(|item| item.value.len() < 12)
        .map(|item| item.name.clone())
        .collect();

    // Check for common passwords
    let common_password_list = ["password", "123456", "password123", "admin", "qwerty"];
    let common_passwords: Vec<String> = password_items
        .iter()
        .filter(|item| {
            common_password_list
                .iter()
                .any(|&common| item.value.to_lowercase().contains(common))
        })
        .map(|item| item.name.clone())
        .collect();

    // Calculate security score (0-100)
    let security_issues = weak_passwords.len()
        + reused_passwords.len()
        + old_passwords.len()
        + short_passwords.len()
        + common_passwords.len();

    #[allow(clippy::cast_precision_loss)]
    let security_score = if password_items.is_empty() {
        100.0
    } else {
        ((password_items.len() as f32 - security_issues as f32) / password_items.len() as f32 * 100.0).max(0.0)
    };

    let response = HealthReportResponse {
        weak_passwords,
        reused_passwords,
        old_passwords,
        short_passwords,
        common_passwords,
        total_items,
        password_items: password_items.len(),
        security_score,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'vault:read' scope
/// - The vault is locked
/// - There are issues accessing or listing vault items
pub async fn stats(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
) -> ApiResult<Json<ApiResponse<StatsResponse>>> {
    // Check if user has permission to view stats
    if !claims.has_scope("vault:read") {
        return Err(ApiError::Forbidden);
    }

    if !state.auth.is_vault_unlocked() {
        return Err(ApiError::BadRequest("Vault is locked".to_string()));
    }

    let vault = state.vault.lock().await;
    let items = vault.list_items().map_err(|e| ApiError::VaultError(e.to_string()))?;
    drop(vault);

    let total_items = items.len();

    // Count items by type using pattern matching
    let mut password_items = 0;
    let mut note_items = 0;
    let mut card_items = 0;
    let mut other_items = 0;
    let mut total_password_length = 0;
    let mut password_count = 0;

    for item in &items {
        match item.kind {
            // Password-related items
            ItemKind::Password => {
                password_items += 1;
                total_password_length += item.value.len();
                password_count += 1;
            }
            // Note-related items
            ItemKind::Note | ItemKind::SecureNote => {
                note_items += 1;
            }
            // Card-related items
            ItemKind::CreditCard | ItemKind::BankAccount => {
                card_items += 1;
            }
            // All other item types
            ItemKind::EnvVar
            | ItemKind::ApiKey
            | ItemKind::SshKey
            | ItemKind::Certificate
            | ItemKind::Database
            | ItemKind::Identity
            | ItemKind::Document
            | ItemKind::Recovery
            | ItemKind::OAuth
            | ItemKind::License
            | ItemKind::WifiPassword
            | ItemKind::Server => {
                other_items += 1;
            }
        }
    }

    // Calculate vault size (rough estimate)
    let vault_size_bytes: u64 = items
        .iter()
        .map(|item| (item.name.len() + item.value.len() + item.kind.as_str().len()) as u64)
        .sum();

    // Find oldest and newest items
    let now = Utc::now();
    let oldest_item_age_days = items
        .iter()
        .map(|item| (now.timestamp() - item.created_at.unix_timestamp()) / (24 * 60 * 60))
        .max();

    let newest_item_age_days = items
        .iter()
        .map(|item| (now.timestamp() - item.created_at.unix_timestamp()) / (24 * 60 * 60))
        .min();

    // Calculate average password length
    #[allow(clippy::cast_precision_loss)]
    let average_password_length = if password_count > 0 {
        Some(total_password_length as f32 / password_count as f32)
    } else {
        None
    };

    let response = StatsResponse {
        total_items,
        password_items,
        note_items,
        card_items,
        other_items,
        vault_size_bytes,
        oldest_item_age_days,
        newest_item_age_days,
        average_password_length,
    };

    Ok(Json(ApiResponse::new(response)))
}
