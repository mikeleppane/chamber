#![allow(dead_code)]
use axum::Router;
use axum_test::TestServer;
use chamber_api::{AuthState, models::*, server::AppState};
use chamber_vault::{Vault, VaultManager};
use std::sync::Arc;
use tempfile::TempDir;
use time::OffsetDateTime;

pub struct TestContext {
    pub server: TestServer,
    pub temp_dir: TempDir,
    pub master_password: String,
    pub auth_token: Option<String>,
}

impl TestContext {
    pub fn new() -> color_eyre::Result<Self> {
        // Create isolated temporary directory for all test resources
        let temp_dir = tempfile::tempdir()?;
        let vault_path = temp_dir.path().join("test_vault.db");

        // Ensure we're using a completely isolated vault manager that won't
        // touch production directories
        let vault_manager = create_isolated_vault_manager(&temp_dir);

        // Create and initialize a test vault in the temp directory
        let mut vault = Vault::open_or_create(Some(&vault_path))?;
        let master_password = "test_master_password_123";
        vault.initialize(master_password)?;

        // Create a test router with isolated state
        let app = create_test_router(vault, vault_manager)?;
        let server = TestServer::new(app).unwrap();

        Ok(Self {
            server,
            temp_dir,
            master_password: master_password.to_string(),
            auth_token: None,
        })
    }

    pub async fn login(&mut self) -> color_eyre::Result<String> {
        let login_request = LoginRequest {
            master_password: self.master_password.clone(),
        };

        let response = self.server.post("/api/v1/auth/login").json(&login_request).await;

        if response.status_code() != 200 {
            return Err(color_eyre::eyre::eyre!(
                "Login failed with status: {}",
                response.status_code()
            ));
        }

        let body: ApiResponse<LoginResponse> = response.json();
        let token = body.data.token.clone();
        self.auth_token = Some(body.data.token);

        Ok(token)
    }

    pub async fn unlock_session(&mut self) -> color_eyre::Result<()> {
        if self.auth_token.is_none() {
            self.login().await?;
        }

        let unlock_request = serde_json::json!({
            "master_password": self.master_password
        });

        let response = self
            .server
            .post("/api/v1/session/unlock")
            .authorization_bearer(self.auth_token.as_ref().unwrap())
            .json(&unlock_request)
            .await;

        if response.status_code() != 200 {
            return Err(color_eyre::eyre::eyre!("Session unlock failed"));
        }

        Ok(())
    }

    pub fn auth_header(&self) -> String {
        format!("Bearer {}", self.auth_token.as_ref().expect("Not authenticated"))
    }

    pub async fn create_test_item(&mut self, name: &str, kind: &str, value: &str) -> color_eyre::Result<u64> {
        if self.auth_token.is_none() {
            self.login().await?;
            self.unlock_session().await?;
        }

        let create_request = CreateItemRequest {
            name: name.to_string(),
            kind: kind.to_string(),
            value: value.to_string(),
        };

        let response = self
            .server
            .post("/api/v1/items")
            .authorization_bearer(self.auth_token.as_ref().unwrap())
            .json(&create_request)
            .await;

        response.assert_status_ok();

        let body: ApiResponse<ItemResponse> = response.json();
        Ok(body.data.id)
    }

    pub async fn create_multiple_test_items(&mut self, count: usize) -> color_eyre::Result<Vec<u64>> {
        let mut item_ids = Vec::new();

        for i in 0..count {
            let item_id = self
                .create_test_item(
                    &format!("test_item_{i}"),
                    if i % 3 == 0 {
                        "password"
                    } else if i % 3 == 1 {
                        "note"
                    } else {
                        "apikey"
                    },
                    &format!("test_value_{i}"),
                )
                .await?;
            item_ids.push(item_id);
        }

        Ok(item_ids)
    }
}

// Helper function to create a test router
fn create_test_router(vault: Vault, vault_manager: VaultManager) -> color_eyre::Result<Router> {
    let state = AppState {
        vault: Arc::new(tokio::sync::Mutex::new(vault)),
        vault_manager: Arc::new(tokio::sync::Mutex::new(vault_manager)),
        auth: AuthState::new(),
    };

    // Use the build_router function from your server
    chamber_api::server::build_router(Arc::new(state))
}

fn create_isolated_vault_manager(temp_dir: &TempDir) -> VaultManager {
    use chamber_vault::{VaultCategory, VaultInfo, VaultRegistry};
    use std::collections::HashMap;

    // Create a registry that only uses the temp directory
    let registry_path = temp_dir.path().join("isolated_registry.json");
    let vault_path = temp_dir.path().join("main_vault.db");

    // Create an isolated vault info
    let vault_info = VaultInfo {
        id: "main".to_string(),
        name: "Test Main Vault".to_string(),
        path: vault_path,
        created_at: OffsetDateTime::now_utc(),
        last_accessed: OffsetDateTime::now_utc(),
        description: Some("Isolated test vault".to_string()),
        category: VaultCategory::Testing,
        is_active: true,
        is_favorite: false,
    };

    // Create the registry with only our test vault
    let mut vaults = HashMap::new();
    vaults.insert("main".to_string(), vault_info);

    let registry = VaultRegistry {
        vaults,
        active_vault_id: Some("main".to_string()),
        registry_path,
    };

    // Create vault manager with isolated registry
    VaultManager {
        registry,
        open_vaults: HashMap::new(),
    }
}
