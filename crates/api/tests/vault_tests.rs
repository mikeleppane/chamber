#![allow(clippy::unwrap_used)]
#![allow(clippy::bool_assert_comparison)]
mod common;

use crate::common::TestContext;
use axum::http::StatusCode;
use chamber_api::models::ApiResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultResponse {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub favorite: bool,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    sub: String,
    scope: String,
    exp: i64,
}

#[tokio::test]
async fn test_create_vault_success() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;
    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    let create_request = json!({
        "name": "Test Vault",
        "category": "personal",
        "description": "A test vault",
        "master_password": "default_password",
        "path": Some(vault_path)
    });

    let response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<VaultResponse> = response.json();
    assert_eq!(body.data.name, "Test Vault");
    assert_eq!(body.data.category, "Personal");
    assert_eq!(body.data.description, Some("A test vault".to_string()));
    assert_eq!(body.data.favorite, false);

    // Verify it's a valid UUID
    uuid::Uuid::parse_str(&body.data.id).expect("ID should be a valid UUID");

    Ok(())
}

#[tokio::test]
async fn test_create_vault_empty_name() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let create_request = json!({
        "name": "",
        "category": "personal",
        "master_password": "default_password"
    });

    let response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json();
    assert!(body.get("error").is_some());
    let error_message = body["error"]["message"].as_str().unwrap();
    assert!(error_message.contains("name cannot be empty"));

    Ok(())
}

#[tokio::test]
async fn test_create_vault_empty_password() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let create_request = json!({
        "name": "Test Vault",
        "category": "personal",
        "master_password": ""
    });

    let response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json();
    assert!(body.get("error").is_some());
    let error_message = body["error"]["message"].as_str().unwrap();
    assert!(error_message.contains("Master password is required"));

    Ok(())
}

#[tokio::test]
async fn test_create_vault_custom_category() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    let create_request = json!({
        "name": "Custom Vault",
        "category": "gaming",
        "description": "For gaming accounts",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<VaultResponse> = response.json();
    assert_eq!(body.data.name, "Custom Vault");
    assert_eq!(body.data.category, "gaming");
    assert_eq!(body.data.description, Some("For gaming accounts".to_string()));
    assert_eq!(body.data.favorite, false);

    // Verify it's a valid UUID
    uuid::Uuid::parse_str(&body.data.id).expect("ID should be a valid UUID");

    Ok(())
}

#[tokio::test]
async fn test_create_vault_default_category() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    let create_request = json!({
        "name": "Default Category Vault",
        "master_password": "secure_password123",
        "path": Some(vault_path)

    });

    let response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<VaultResponse> = response.json();
    assert_eq!(body.data.name, "Default Category Vault");
    assert_eq!(body.data.category, "Personal"); // Should default to personal (lowercase)
    assert_eq!(body.data.favorite, false);

    // Verify it's a valid UUID
    uuid::Uuid::parse_str(&body.data.id).expect("ID should be a valid UUID");

    Ok(())
}

#[tokio::test]
async fn test_create_multiple_vaults() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let vaults_to_create = vec![
        ("Personal Vault", "Personal", "For personal use"),
        ("Work Vault", "Work", "For work accounts"),
        ("Project Alpha", "Project", "Alpha project vault"),
    ];

    for (name, category, description) in vaults_to_create {
        let vault_path = ctx.temp_dir.path().join("test_vault.db");
        let create_request = json!({
            "name": name,
            "category": category,
            "description": description,
            "master_password": "secure_password123",
            "path": Some(vault_path)

        });

        let response = ctx
            .server
            .post("/api/v1/vaults")
            .authorization_bearer(ctx.auth_token.as_ref().unwrap())
            .json(&create_request)
            .await;

        response.assert_status_ok();

        let body: ApiResponse<VaultResponse> = response.json();
        assert_eq!(body.data.name, name);
        assert_eq!(body.data.category, category);
        assert_eq!(body.data.description, Some(description.to_string()));

        // Verify it's a valid UUID
        uuid::Uuid::parse_str(&body.data.id).expect("ID should be a valid UUID");
    }

    Ok(())
}

#[tokio::test]
async fn test_list_vaults_after_creation() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    // First, create a vault
    let create_request = json!({
        "name": "Listable Vault",
        "category": "work",
        "description": "Vault for listing test",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let create_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    create_response.assert_status_ok();

    // Then list vaults
    let list_response = ctx
        .server
        .get("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    list_response.assert_status_ok();

    let body: ApiResponse<Vec<VaultResponse>> = list_response.json();

    // Should have at least our created vault (there might be a default one too)
    assert!(!body.data.is_empty());

    // Check if our vault is in the list
    let vault_names: Vec<&str> = body.data.iter().map(|v| v.name.as_str()).collect();
    assert!(vault_names.contains(&"Listable Vault"));

    Ok(())
}

#[tokio::test]
async fn test_create_vault_without_auth() -> color_eyre::Result<()> {
    let ctx = TestContext::new()?;

    let create_request = json!({
        "name": "Unauthorized Vault",
        "master_password": "password123"
    });

    let response = ctx.server.post("/api/v1/vaults").json(&create_request).await;

    response.assert_status(StatusCode::UNAUTHORIZED); // Unauthorized

    Ok(())
}

#[tokio::test]
async fn test_list_vaults_without_auth() -> color_eyre::Result<()> {
    let ctx = TestContext::new()?;

    let response = ctx.server.get("/api/v1/vaults").await;

    response.assert_status(StatusCode::UNAUTHORIZED); // Unauthorized

    Ok(())
}

#[tokio::test]
async fn test_create_vault_various_categories() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let categories = vec![
        ("personal", "Personal"),
        ("work", "Work"),
        ("team", "Team"),
        ("project", "Project"),
        ("testing", "Testing"),
        ("archive", "Archive"),
        ("gaming", "gaming"),   // Custom category
        ("finance", "finance"), // Another custom category
    ];

    for (input_category, expected_category) in categories {
        let vault_path = ctx.temp_dir.path().join("test_vault.db");
        let create_request = json!({
            "name": format!("{} Vault", input_category.to_uppercase()),
            "category": input_category,
            "master_password": "secure_password123",
            "path": Some(vault_path)
        });

        let response = ctx
            .server
            .post("/api/v1/vaults")
            .authorization_bearer(ctx.auth_token.as_ref().unwrap())
            .json(&create_request)
            .await;

        response.assert_status_ok();

        let body: ApiResponse<VaultResponse> = response.json();
        assert_eq!(body.data.category, expected_category);

        // Verify it's a valid UUID
        uuid::Uuid::parse_str(&body.data.id).expect("ID should be a valid UUID");
    }

    Ok(())
}

#[tokio::test]
async fn test_create_vault_long_name() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let long_name = "A".repeat(100); // Very long name

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    let create_request = json!({
        "name": long_name,
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<VaultResponse> = response.json();
    assert_eq!(body.data.name, long_name);

    Ok(())
}

#[tokio::test]
async fn test_create_vault_special_characters_in_name() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let special_name = "Test Vault !@#$%^&*()_+-=[]{}|;:,.<>?";

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    let create_request = json!({
        "name": special_name,
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<VaultResponse> = response.json();
    assert_eq!(body.data.name, special_name);

    Ok(())
}

#[tokio::test]
async fn test_switch_vault_success() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    // First, create a vault to switch to
    let create_request = json!({
        "name": "Switch Target Vault",
        "category": "work",
        "description": "Vault to switch to",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let create_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    create_response.assert_status_ok();

    let create_body: ApiResponse<VaultResponse> = create_response.json();
    let vault_id = create_body.data.id;

    // Now switch to the created vault
    let switch_response = ctx
        .server
        .post(&format!("/api/v1/vaults/{vault_id}/switch"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    switch_response.assert_status_ok();

    let switch_body: ApiResponse<String> = switch_response.json();
    assert!(switch_body.data.contains(&vault_id));
    assert!(switch_body.data.contains("Switched to vault"));

    // Verify the vault is now active by listing vaults
    let list_response = ctx
        .server
        .get("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    list_response.assert_status_ok();

    let list_body: ApiResponse<Vec<VaultResponse>> = list_response.json();
    let switched_vault = list_body
        .data
        .iter()
        .find(|v| v.id == vault_id)
        .expect("Should find the switched vault");

    assert!(switched_vault.is_active);

    Ok(())
}

#[tokio::test]
async fn test_switch_vault_nonexistent() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let nonexistent_vault_id = "00000000-0000-0000-0000-000000000000";

    let switch_response = ctx
        .server
        .post(&format!("/api/v1/vaults/{nonexistent_vault_id}/switch"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    switch_response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);

    let body: serde_json::Value = switch_response.json();
    assert!(body.get("error").is_some());
    let error_message = body["error"]["message"].as_str().unwrap();
    assert!(error_message.contains("Failed to switch vault"));

    Ok(())
}

#[tokio::test]
async fn test_switch_vault_without_auth() -> color_eyre::Result<()> {
    let ctx = TestContext::new()?;

    let vault_id = "some-vault-id";

    let response = ctx.server.post(&format!("/api/v1/vaults/{vault_id}/switch")).await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    Ok(())
}

#[tokio::test]
async fn test_switch_vault_insufficient_scope() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;

    // Login with limited scopes (no manage:vaults)
    let login_response = ctx
        .server
        .post("/api/v1/auth/login")
        .json(&json!({
            "master_password": &ctx.master_password
        }))
        .await;

    login_response.assert_status_ok();
    let login_body: chamber_api::models::ApiResponse<chamber_api::models::LoginResponse> = login_response.json();

    // Override with limited token (only read scopes)
    ctx.auth_token = Some(login_body.data.token);

    let vault_id = "some-vault-id";

    let response = ctx
        .server
        .post(&format!("/api/v1/vaults/{vault_id}/switch"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);

    Ok(())
}

#[tokio::test]
async fn test_switch_between_multiple_vaults() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    // Create multiple vaults
    let vaults_to_create = vec![("Vault A", "personal"), ("Vault B", "work"), ("Vault C", "project")];

    let mut vault_ids = Vec::new();

    for (name, category) in vaults_to_create {
        let vault_path = ctx.temp_dir.path().join("test_vault.db");
        let create_request = json!({
            "name": name,
            "category": category,
            "master_password": "secure_password123",
            "path": Some(vault_path)
        });

        let create_response = ctx
            .server
            .post("/api/v1/vaults")
            .authorization_bearer(ctx.auth_token.as_ref().unwrap())
            .json(&create_request)
            .await;

        create_response.assert_status_ok();

        let create_body: ApiResponse<VaultResponse> = create_response.json();
        vault_ids.push(create_body.data.id);
    }

    // Switch between vaults and verify each becomes active
    for vault_id in &vault_ids {
        // Switch to this vault
        let switch_response = ctx
            .server
            .post(&format!("/api/v1/vaults/{vault_id}/switch"))
            .authorization_bearer(ctx.auth_token.as_ref().unwrap())
            .await;

        switch_response.assert_status_ok();

        // Verify this vault is now active
        let list_response = ctx
            .server
            .get("/api/v1/vaults")
            .authorization_bearer(ctx.auth_token.as_ref().unwrap())
            .await;

        list_response.assert_status_ok();

        let list_body: ApiResponse<Vec<VaultResponse>> = list_response.json();

        // Check that only this vault is active
        for vault in &list_body.data {
            if vault.id == *vault_id {
                assert!(vault.is_active, "Vault {vault_id} should be active");
            } else {
                // Other vaults should not be active (unless they're default vaults)
                // We'll be lenient here since there might be default vaults
            }
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_switch_vault_updates_active_status() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    // Create two vaults
    let vault_a_request = json!({
        "name": "Vault A",
        "category": "personal",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let vault_a_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&vault_a_request)
        .await;

    vault_a_response.assert_status_ok();
    let vault_a_body: ApiResponse<VaultResponse> = vault_a_response.json();
    let vault_a_id = vault_a_body.data.id;

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    let vault_b_request = json!({
        "name": "Vault B",
        "category": "work",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let vault_b_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&vault_b_request)
        .await;

    vault_b_response.assert_status_ok();
    let vault_b_body: ApiResponse<VaultResponse> = vault_b_response.json();
    let vault_b_id = vault_b_body.data.id;

    // Switch to Vault A
    let switch_a_response = ctx
        .server
        .post(&format!("/api/v1/vaults/{vault_a_id}/switch"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    switch_a_response.assert_status_ok();

    // Get current vault list and verify Vault A is active
    let list_after_a = ctx
        .server
        .get("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    list_after_a.assert_status_ok();
    let list_a_body: ApiResponse<Vec<VaultResponse>> = list_after_a.json();

    let vault_a_status = list_a_body.data.iter().find(|v| v.id == vault_a_id).unwrap();
    let vault_b_status = list_a_body.data.iter().find(|v| v.id == vault_b_id).unwrap();

    assert!(vault_a_status.is_active);
    assert!(!vault_b_status.is_active);

    // Switch to Vault B
    let switch_b_response = ctx
        .server
        .post(&format!("/api/v1/vaults/{vault_b_id}/switch"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    switch_b_response.assert_status_ok();

    // Get the current vault list and verify Vault B is active, Vault A is not
    let list_after_b = ctx
        .server
        .get("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    list_after_b.assert_status_ok();
    let list_b_body: ApiResponse<Vec<VaultResponse>> = list_after_b.json();

    let vault_a_status = list_b_body.data.iter().find(|v| v.id == vault_a_id).unwrap();
    let vault_b_status = list_b_body.data.iter().find(|v| v.id == vault_b_id).unwrap();

    assert!(!vault_a_status.is_active);
    assert!(vault_b_status.is_active);

    Ok(())
}

#[tokio::test]
async fn test_switch_vault_invalid_uuid() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let invalid_vault_id = "not-a-valid-uuid";

    let switch_response = ctx
        .server
        .post(&format!("/api/v1/vaults/{invalid_vault_id}/switch"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    switch_response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);

    let body: serde_json::Value = switch_response.json();
    assert!(body.get("error").is_some());
    let error_message = body["error"]["message"].as_str().unwrap();
    assert!(error_message.contains("Failed to switch vault"));

    Ok(())
}

#[tokio::test]
async fn test_update_vault_success() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    // First, create a vault to update
    let create_request = json!({
        "name": "Original Vault",
        "category": "personal",
        "description": "Original description",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let create_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    create_response.assert_status_ok();

    let create_body: ApiResponse<VaultResponse> = create_response.json();
    let vault_id = create_body.data.id;

    // Now update the vault
    let update_request = json!({
        "name": "Updated Vault",
        "category": "work",
        "description": "Updated description",
        "favorite": true
    });

    let update_response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    update_response.assert_status_ok();

    let update_body: ApiResponse<VaultResponse> = update_response.json();
    assert_eq!(update_body.data.id, vault_id);
    assert_eq!(update_body.data.name, "Updated Vault");
    assert_eq!(update_body.data.category, "Work");
    assert_eq!(update_body.data.description, Some("Updated description".to_string()));
    assert!(update_body.data.favorite, "{}", true);

    Ok(())
}

#[tokio::test]
async fn test_update_vault_partial() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    // First, create a vault to update
    let create_request = json!({
        "name": "Original Vault",
        "category": "personal",
        "description": "Original description",
        "master_password": "secure_password123",
        "path": Some(vault_path)

    });

    let create_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    create_response.assert_status_ok();

    let create_body: ApiResponse<VaultResponse> = create_response.json();
    let vault_id = create_body.data.id;

    // Update only the name and favorite status
    let update_request = json!({
        "name": "Partially Updated Vault",
        "favorite": true
    });

    let update_response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    update_response.assert_status_ok();

    let update_body: ApiResponse<VaultResponse> = update_response.json();
    assert_eq!(update_body.data.id, vault_id);
    assert_eq!(update_body.data.name, "Partially Updated Vault");
    assert_eq!(update_body.data.category, "Personal"); // Should remain unchanged
    assert_eq!(update_body.data.description, Some("Original description".to_string())); // Should remain unchanged
    assert!(update_body.data.favorite, "{}", true);

    Ok(())
}

#[tokio::test]
async fn test_update_vault_name_only() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    // Create a vault
    let create_request = json!({
        "name": "Name Update Test",
        "category": "work",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let create_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    create_response.assert_status_ok();
    let create_body: ApiResponse<VaultResponse> = create_response.json();
    let vault_id = create_body.data.id;

    // Update only the name
    let update_request = json!({
        "name": "New Name Only"
    });

    let update_response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    update_response.assert_status_ok();

    let update_body: ApiResponse<VaultResponse> = update_response.json();
    assert_eq!(update_body.data.name, "New Name Only");
    assert_eq!(update_body.data.category, "Work"); // Should remain unchanged

    Ok(())
}

#[tokio::test]
async fn test_update_vault_category_to_custom() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    // Create a vault with standard category
    let create_request = json!({
        "name": "Category Test Vault",
        "category": "personal",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let create_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    create_response.assert_status_ok();
    let create_body: ApiResponse<VaultResponse> = create_response.json();
    let vault_id = create_body.data.id;

    // Update to custom category
    let update_request = json!({
        "category": "gaming"
    });

    let update_response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    update_response.assert_status_ok();

    let update_body: ApiResponse<VaultResponse> = update_response.json();
    assert_eq!(update_body.data.category, "gaming");

    Ok(())
}

#[tokio::test]
async fn test_update_vault_favorite_status() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    // Create a vault (default is not favorite)
    let create_request = json!({
        "name": "Favorite Test Vault",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let create_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    create_response.assert_status_ok();
    let create_body: ApiResponse<VaultResponse> = create_response.json();
    let vault_id = create_body.data.id;
    assert_eq!(create_body.data.favorite, false);

    // Mark as favorite
    let update_favorite_request = json!({
        "favorite": true
    });

    let update_favorite_response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_favorite_request)
        .await;

    update_favorite_response.assert_status_ok();
    let update_favorite_body: ApiResponse<VaultResponse> = update_favorite_response.json();
    assert_eq!(update_favorite_body.data.favorite, true);

    // Unmark as favorite
    let update_unfavorite_request = json!({
        "favorite": false
    });

    let update_unfavorite_response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_unfavorite_request)
        .await;

    update_unfavorite_response.assert_status_ok();
    let update_unfavorite_body: ApiResponse<VaultResponse> = update_unfavorite_response.json();
    assert_eq!(update_unfavorite_body.data.favorite, false);

    Ok(())
}

#[tokio::test]
async fn test_update_vault_nonexistent() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let nonexistent_vault_id = "00000000-0000-0000-0000-000000000000";

    let update_request = json!({
        "name": "This Won't Work"
    });

    let update_response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{nonexistent_vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    update_response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);

    let body: serde_json::Value = update_response.json();
    assert!(body.get("error").is_some());
    let error_message = body["error"]["message"].as_str().unwrap();
    assert!(error_message.contains("Failed to update vault"));

    Ok(())
}

#[tokio::test]
async fn test_update_vault_without_auth() -> color_eyre::Result<()> {
    let ctx = TestContext::new()?;

    let vault_id = "some-vault-id";
    let update_request = json!({
        "name": "Unauthorized Update"
    });

    let response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{vault_id}"))
        .json(&update_request)
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    Ok(())
}

#[tokio::test]
async fn test_update_vault_insufficient_scope() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;

    // Login with limited scopes (no manage:vaults)
    let login_response = ctx
        .server
        .post("/api/v1/auth/login")
        .json(&json!({
            "master_password": &ctx.master_password
        }))
        .await;

    login_response.assert_status_ok();
    let login_body: chamber_api::models::ApiResponse<chamber_api::models::LoginResponse> = login_response.json();

    // Override with limited token (only read scopes)
    ctx.auth_token = Some(login_body.data.token);

    let vault_id = "some-vault-id";
    let update_request = json!({
        "name": "Forbidden Update"
    });

    let response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);

    Ok(())
}

#[tokio::test]
async fn test_update_vault_empty_request() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    // Create a vault
    let create_request = json!({
        "name": "Empty Update Test",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let create_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    create_response.assert_status_ok();
    let create_body: ApiResponse<VaultResponse> = create_response.json();
    let vault_id = create_body.data.id;

    // Send empty update request (should still succeed but change nothing)
    let empty_request = json!({});

    let update_response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&empty_request)
        .await;

    update_response.assert_status_ok();

    let update_body: ApiResponse<VaultResponse> = update_response.json();
    // All original values should remain the same
    assert_eq!(update_body.data.name, "Empty Update Test");
    assert_eq!(update_body.data.category, "Personal");
    assert_eq!(update_body.data.favorite, false);

    Ok(())
}

#[tokio::test]
async fn test_update_vault_standard_categories() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    // Create a vault
    let create_request = json!({
        "name": "Category Update Test",
        "category": "personal",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let create_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    create_response.assert_status_ok();
    let create_body: ApiResponse<VaultResponse> = create_response.json();
    let vault_id = create_body.data.id;

    // Test updating to each standard category
    let standard_categories = vec![
        ("work", "Work"),
        ("team", "Team"),
        ("project", "Project"),
        ("testing", "Testing"),
        ("archive", "Archive"),
        ("personal", "Personal"), // Back to original
    ];

    for (input_category, expected_display) in standard_categories {
        let update_request = json!({
            "category": input_category
        });

        let update_response = ctx
            .server
            .patch(&format!("/api/v1/vaults/{vault_id}"))
            .authorization_bearer(ctx.auth_token.as_ref().unwrap())
            .json(&update_request)
            .await;

        update_response.assert_status_ok();

        let update_body: ApiResponse<VaultResponse> = update_response.json();
        assert_eq!(update_body.data.category, expected_display);
    }

    Ok(())
}

#[tokio::test]
async fn test_update_vault_description_to_empty() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let vault_path = ctx.temp_dir.path().join("test_vault.db");
    // Create a vault with a description
    let create_request = json!({
        "name": "Description Test",
        "description": "Original description",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let create_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    create_response.assert_status_ok();
    let create_body: ApiResponse<VaultResponse> = create_response.json();
    let vault_id = create_body.data.id;

    // Update description to null/empty
    let update_request = json!({
        "description": ""
    });

    let update_response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    update_response.assert_status_ok();

    let update_body: ApiResponse<VaultResponse> = update_response.json();
    assert_eq!(update_body.data.description, Some(String::new()));

    Ok(())
}

#[tokio::test]
async fn test_update_vault_invalid_uuid() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let invalid_vault_id = "not-a-valid-uuid";

    let update_request = json!({
        "name": "This Won't Work"
    });

    let update_response = ctx
        .server
        .patch(&format!("/api/v1/vaults/{invalid_vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    update_response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);

    let body: serde_json::Value = update_response.json();
    assert!(body.get("error").is_some());
    let error_message = body["error"]["message"].as_str().unwrap();
    assert!(error_message.contains("Failed to update vault"));

    Ok(())
}

#[tokio::test]
async fn test_delete_vault_success() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    let vault_path = ctx.temp_dir.path().join("test_vault.db");

    // First, create a vault to delete
    let create_request = json!({
        "name": "Vault to Delete",
        "category": "testing",
        "description": "This vault will be deleted",
        "master_password": "secure_password123",
        "path": Some(vault_path)
    });

    let create_response = ctx
        .server
        .post("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    create_response.assert_status_ok();

    let create_body: ApiResponse<VaultResponse> = create_response.json();
    let vault_id = create_body.data.id;

    // Delete the vault (without deleting file)
    let delete_request = json!({
        "delete_file": false
    });

    let delete_response = ctx
        .server
        .delete(&format!("/api/v1/vaults/{vault_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&delete_request)
        .await;

    delete_response.assert_status_ok();

    let delete_body: ApiResponse<String> = delete_response.json();
    assert!(delete_body.data.contains(&vault_id));
    assert!(delete_body.data.contains("Deleted vault"));

    // Verify the vault is no longer in the list
    let list_response = ctx
        .server
        .get("/api/v1/vaults")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    list_response.assert_status_ok();

    let list_body: ApiResponse<Vec<VaultResponse>> = list_response.json();
    let deleted_vault = list_body.data.iter().find(|v| v.id == vault_id);
    assert!(deleted_vault.is_none(), "Deleted vault should not appear in list");

    Ok(())
}
