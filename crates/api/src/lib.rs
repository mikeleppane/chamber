pub mod auth;
pub mod error;
pub mod handlers;
pub mod models;
pub mod server;

pub use auth::{AuthState, TokenClaims};
pub use error::{ApiError, ApiResult};
pub use models::{
    HealthReportResponse, OldPasswordItem, ReusedPasswordGroup, SearchParams, SearchResponse, StatsResponse,
};
pub use server::ApiServer;

use chamber_vault::VaultManager;
use color_eyre::Result;

/// Initializes and starts the API server.
///
/// This asynchronous function takes a `chamber_vault::Vault` instance, which provides access
/// to sensitive configuration or secrets, and a bind address (e.g., "127.0.0.1:8080") that determines
/// where the API server will be hosted.
///
/// # Arguments
///
/// * `vault` - An instance of `chamber_vault::Vault` used to manage secrets or configuration needed by the server.
/// * `bind_address` - A `&str` that specifies the address and port the server will bind to (e.g., "0.0.0.0:8080").
///
/// # Returns
///
/// Returns a `Result` containing an `ApiServer` instance if the server is initialized successfully,
/// or an error if the initialization fails.
///
/// # Errors
///
/// This function may return an error if:
/// * The server fails to bind to the specified address and port.
/// * There is an issue initializing the server with the given vault.
pub async fn init_api_server(
    vault: chamber_vault::Vault,
    vault_manager: VaultManager,
    bind_address: &str,
) -> Result<ApiServer> {
    ApiServer::new(vault, vault_manager, bind_address).await
}
