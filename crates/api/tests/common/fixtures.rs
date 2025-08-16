#![allow(dead_code)]

use chamber_api::models::{
    CreateItemRequest, CreateVaultRequest, GeneratePasswordRequest, LoginRequest, UpdateItemRequest, VaultInfo,
};
use chamber_vault::{VaultCategory, VaultRegistry};
use chrono::DateTime;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;

pub fn sample_login_request() -> LoginRequest {
    LoginRequest {
        master_password: "test_master_password_123".to_string(),
    }
}

pub fn sample_create_item_request() -> CreateItemRequest {
    CreateItemRequest {
        name: "Test Password".to_string(),
        kind: "password".to_string(),
        value: "super_secret_password_123".to_string(),
    }
}

pub fn sample_update_item_request() -> UpdateItemRequest {
    UpdateItemRequest {
        name: Some("Updated Password".to_string()),
        kind: Some("password".to_string()),
        value: Some("new_super_secret_password_456".to_string()),
    }
}

pub const fn sample_generate_password_request() -> GeneratePasswordRequest {
    GeneratePasswordRequest {
        length: 16,
        include_uppercase: true,
        include_lowercase: true,
        include_digits: true,
        include_symbols: true,
        exclude_ambiguous: true,
    }
}

pub fn sample_create_vault_request() -> CreateVaultRequest {
    CreateVaultRequest {
        name: "Test Vault".to_string(),
        description: Some("A test vault".to_string()),
        category: Some("testing".to_string()),
        master_password: "test_vault_password_123".to_string(),
        path: None, // Default behavior for non-test usage
    }
}

pub fn sample_search_params() -> serde_json::Value {
    json!({
        "q": "test",
        "limit": 10,
        "offset": 0,
        "sort": "name",
        "order": "asc"
    })
}

fn create_temp_registry() -> (VaultRegistry, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let registry_path = temp_dir.path().join("test_registry.json");

    let registry = VaultRegistry {
        vaults: HashMap::new(),
        active_vault_id: None,
        registry_path,
    };

    (registry, temp_dir)
}

fn create_test_vault_info(id: &str, name: &str, _category: &VaultCategory, temp_dir: &TempDir) -> VaultInfo {
    let vault_path = temp_dir.path().join(format!("{id}.db"));

    VaultInfo {
        id: id.to_string(),
        name: name.to_string(),
        path: vault_path.to_str().unwrap().to_string(),
        created_at: DateTime::default(),
        updated_at: DateTime::default(),
        item_count: 0,
        description: None,
        is_favorite: false,
        is_current: false,
        category: String::from("Personal"),
    }
}
