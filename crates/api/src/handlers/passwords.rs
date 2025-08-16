use axum::Json;
use axum::extract::State;
use std::sync::Arc;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiError, ApiResult};
use crate::models::{ApiResponse, GeneratePasswordRequest, PasswordResponse};
use crate::server::AppState;
use chamber_password_gen::{PasswordConfig, generate_memorable_password};

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'generate:passwords' scope
/// - The password generation operation fails due to invalid configuration
pub async fn generate_password(
    State(_state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Json(request): Json<GeneratePasswordRequest>,
) -> ApiResult<Json<ApiResponse<PasswordResponse>>> {
    if !claims.has_scope("generate:passwords") {
        return Err(ApiError::Forbidden);
    }

    let config = PasswordConfig::new()
        .with_length(request.length)
        .with_uppercase(request.include_uppercase)
        .with_lowercase(request.include_lowercase)
        .with_digits(request.include_digits)
        .with_symbols(request.include_symbols)
        .with_exclude_ambiguous(request.exclude_ambiguous);

    let password = config.generate().map_err(|e| ApiError::InternalError(e.to_string()))?;

    let strength = if password.len() >= 16
        && password.chars().any(char::is_uppercase)
        && password.chars().any(char::is_lowercase)
        && password.chars().any(char::is_numeric)
        && password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c))
    {
        "Strong".to_string()
    } else if password.len() >= 12 {
        "Medium".to_string()
    } else {
        "Weak".to_string()
    };

    let response = PasswordResponse { password, strength };
    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'generate:passwords' scope
pub async fn generate_memorable_password_handler(
    State(_state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
) -> ApiResult<Json<ApiResponse<PasswordResponse>>> {
    if !claims.has_scope("generate:passwords") {
        return Err(ApiError::Forbidden);
    }

    let password = generate_memorable_password();
    let response = PasswordResponse {
        password,
        strength: "Medium".to_string(),
    };

    Ok(Json(ApiResponse::new(response)))
}
