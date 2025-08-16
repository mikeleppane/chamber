use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl<T> ApiResponse<T> {
    pub const fn new(data: T) -> Self {
        Self { data, meta: None }
    }

    pub const fn with_meta(data: T, meta: serde_json::Value) -> Self {
        Self { data, meta: Some(meta) }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemResponse {
    pub id: u64,
    pub name: String,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub has_value: bool,
    pub value_length: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemWithValueResponse {
    pub id: u64,
    pub name: String,
    pub kind: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateItemRequest {
    pub name: String,
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateItemRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginRequest {
    pub master_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Deserialize, Serialize)]
pub struct GeneratePasswordRequest {
    #[serde(default = "default_length")]
    pub length: usize,
    #[serde(default = "default_true")]
    pub include_uppercase: bool,
    #[serde(default = "default_true")]
    pub include_lowercase: bool,
    #[serde(default = "default_true")]
    pub include_digits: bool,
    #[serde(default = "default_true")]
    pub include_symbols: bool,
    #[serde(default = "default_true")]
    pub exclude_ambiguous: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordResponse {
    pub password: String,
    pub strength: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub vault_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CountsResponse {
    pub total: usize,
    pub by_kind: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
}

const fn default_length() -> usize {
    16
}
const fn default_true() -> bool {
    true
}
const fn default_limit() -> usize {
    50
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthReportResponse {
    pub weak_passwords: Vec<String>,
    pub reused_passwords: Vec<ReusedPasswordGroup>,
    pub old_passwords: Vec<OldPasswordItem>,
    pub short_passwords: Vec<String>,
    pub common_passwords: Vec<String>,
    pub total_items: usize,
    pub password_items: usize,
    pub security_score: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReusedPasswordGroup {
    pub password_hash: String,
    pub item_names: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OldPasswordItem {
    pub item_name: String,
    pub days_old: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub total_items: usize,
    pub password_items: usize,
    pub note_items: usize,
    pub card_items: usize,
    pub other_items: usize,
    pub vault_size_bytes: u64,
    pub oldest_item_age_days: Option<i64>,
    pub newest_item_age_days: Option<i64>,
    pub average_password_length: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    #[serde(default)]
    pub q: Option<String>, // Search query (matches name, kind, or value preview)
    #[serde(default)]
    pub query: Option<String>, // Alias for 'q' for compatibility
    #[serde(default)]
    pub kind: Option<String>, // Filter by item type
    #[serde(default)]
    pub name: Option<String>, // Search in item names only
    #[serde(default = "default_limit")]
    pub limit: usize, // Maximum results to return
    #[serde(default)]
    pub offset: usize, // Pagination offset
    #[serde(default)]
    pub sort: Option<String>, // Sort field (name, created_at, updated_at, kind)
    #[serde(default)]
    pub order: Option<String>, // Sort order (asc, desc)
    #[serde(default)]
    pub fuzzy: Option<bool>, // Enable fuzzy matching
    #[serde(default)]
    pub case_sensitive: Option<bool>, // Case sensitive search
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub items: Vec<ItemResponse>,
    pub total_found: usize,
    pub total_available: usize,
    pub query_time_ms: u64,
    pub has_more: bool,
    pub next_offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CreateVaultRequest {
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub master_password: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVaultRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SwitchVaultRequest {
    pub master_password: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteVaultRequest {
    pub confirm_name: String,
    pub master_password: String,
}

#[derive(Debug, Serialize)]
pub struct VaultInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub path: String,
    pub is_favorite: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub item_count: usize,
    pub is_current: bool,
}

#[derive(Debug, Serialize)]
pub struct VaultListResponse {
    pub vaults: Vec<VaultInfo>,
    pub current_vault_id: Option<String>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct VaultOperationResponse {
    pub success: bool,
    pub message: String,
    pub vault_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListItemsResponse {
    pub items: Vec<ItemResponse>,
    pub total: usize,
}
