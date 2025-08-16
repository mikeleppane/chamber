use axum::Json;
use axum::extract::State;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiError, ApiResult};
use crate::models::{ApiResponse, LoginRequest, LoginResponse};
use crate::server::AppState;

/// # Errors
///
/// This function returns an error if:
/// - The provided master password is incorrect (Unauthorized)
/// - The vault cannot be unlocked due to internal errors
/// - Token generation fails due to internal errors
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<ApiResponse<LoginResponse>>> {
    // Try to unlock the vault with the provided password

    state
        .vault
        .lock()
        .await
        .unlock(&request.master_password)
        .map_err(|_| ApiError::Unauthorized)?;

    // Set vault as unlocked
    state.auth.set_vault_unlocked(true);

    // Generate token with default scopes
    let scopes = vec![
        "read:items".to_string(),
        "write:items".to_string(),
        "reveal:values".to_string(),
        "generate:passwords".to_string(),
        "vault:health".to_string(),
        "vault:read".to_string(),
        "vault:list".to_string(),
        "vault:create".to_string(),
        "vault:update".to_string(),
        "vault:delete".to_string(),
        "vault:switch".to_string(),
        "manage:vaults".to_string(),
    ];

    let token = state.auth.generate_token(scopes.clone())?;
    let expires_at: DateTime<Utc> = Utc::now() + Duration::hours(24);

    let response = LoginResponse {
        token,
        expires_at,
        scopes,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
///
/// This function returns an error if:
/// - The authentication token is invalid or expired
/// - The vault state cannot be updated due to internal errors
pub async fn logout(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(_claims): AuthenticatedUser,
) -> ApiResult<Json<ApiResponse<String>>> {
    // Lock the vault
    state.auth.set_vault_unlocked(false);

    Ok(Json(ApiResponse::new("Logged out successfully".to_string())))
}

/// # Errors
///
/// This function returns an error if:
/// - The authentication token is invalid or expired
/// - The vault state cannot be updated due to internal errors
pub async fn session_lock(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(_claims): AuthenticatedUser,
) -> ApiResult<Json<ApiResponse<String>>> {
    // Lock the vault and invalidate session
    state.auth.set_vault_unlocked(false);

    Ok(Json(ApiResponse::new("Session locked successfully".to_string())))
}

/// # Errors
///
/// This function returns an error if:
/// - The provided master password is incorrect (Unauthorized)
/// - The vault cannot be unlocked due to internal errors
pub async fn session_unlock(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(_claims): AuthenticatedUser, // Add this parameter to require auth
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    // Try to unlock the vault with the provided password

    state
        .vault
        .lock()
        .await
        .unlock(&request.master_password)
        .map_err(|_| ApiError::Unauthorized)?;

    // Set vault as unlocked
    state.auth.set_vault_unlocked(true);

    Ok(Json(ApiResponse::new("Session unlocked successfully".to_string())))
}
