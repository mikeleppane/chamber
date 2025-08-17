use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::server::AppState;

#[derive(Debug, Clone)]
pub struct AuthState {
    pub secret: Vec<u8>,
    pub vault_unlocked: Arc<std::sync::Mutex<bool>>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthState {
    #[must_use]
    pub fn new() -> Self {
        let mut secret = vec![0u8; 32];
        rand::Rng::fill(&mut rand::rng(), &mut secret[..]);

        Self {
            secret,
            vault_unlocked: Arc::new(std::sync::Mutex::new(false)),
        }
    }

    pub fn set_vault_unlocked(&self, unlocked: bool) {
        if let Ok(mut status) = self.vault_unlocked.lock() {
            *status = unlocked;
        }
    }

    /// # Errors
    ///
    /// This function does not return errors, but may return false if:
    /// - The mutex lock is poisoned or cannot be acquired
    #[must_use]
    pub fn is_vault_unlocked(&self) -> bool {
        self.vault_unlocked.lock().map(|status| *status).unwrap_or(false)
    }

    /// # Errors
    ///
    /// This function returns an error if:
    /// - Token expiration timestamp conversion fails
    /// - Token issue timestamp conversion fails
    /// - Token generation process fails
    pub fn generate_token(&self, scopes: Vec<String>) -> ApiResult<String> {
        let expiration = Utc::now() + Duration::hours(1);

        let claims = TokenClaims {
            sub: "api-user".to_string(),
            exp: usize::try_from(expiration.timestamp())
                .map_err(|_| ApiError::InternalError("Token expiration timestamp overflow".to_string()))?,
            iat: usize::try_from(Utc::now().timestamp())
                .map_err(|_| ApiError::InternalError("Token issue timestamp overflow".to_string()))?,
            jti: Uuid::new_v4().to_string(),
            scopes,
        };

        encode(&Header::default(), &claims, &EncodingKey::from_secret(&self.secret))
            .map_err(|e| ApiError::InternalError(format!("Token generation failed: {e}")))
    }

    /// # Errors
    ///
    /// This function returns an error if:
    /// - The token is invalid or expired
    /// - The token signature is invalid
    /// - The token format is incorrect
    pub fn verify_token(&self, token: &str) -> ApiResult<TokenClaims> {
        decode::<TokenClaims>(token, &DecodingKey::from_secret(&self.secret), &Validation::default())
            .map(|data| data.claims)
            .map_err(|_| ApiError::Unauthorized)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
    pub scopes: Vec<String>,
}

impl TokenClaims {
    #[must_use]
    pub fn has_scope(&self, required_scope: &str) -> bool {
        self.scopes.contains(&required_scope.to_string())
    }
}

// Simplified approach: Use the AuthState directly from extensions
#[derive(Debug)]
pub struct AuthenticatedUser(pub TokenClaims);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get the authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|header| header.to_str().ok())
            .and_then(|header| {
                // Make Bearer prefix case-insensitive
                if header.len() >= 7 && header[..7].eq_ignore_ascii_case("bearer ") {
                    Some(&header[7..])
                } else {
                    None
                }
            })
            .ok_or(ApiError::Unauthorized)?;

        // Get the auth state from request extensions (added by middleware)
        let auth_state = parts
            .extensions
            .get::<AuthState>()
            .ok_or(ApiError::InternalError("Auth state not found".to_string()))?;

        // Verify the token using the auth state
        let claims = auth_state.verify_token(auth_header)?;
        Ok(AuthenticatedUser(claims))
    }
}

// Simple middleware that just adds AuthState to extensions
use axum::body::Body;
use axum::{http::Request, middleware::Next, response::Response};

pub async fn auth_middleware(State(state): State<Arc<AppState>>, mut request: Request<Body>, next: Next) -> Response {
    // Add auth state to request extensions so extractors can access it
    request.extensions_mut().insert(state.auth.clone());
    next.run(request).await
}
