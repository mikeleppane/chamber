use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Instant;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiError, ApiResult};
use crate::models::{
    ApiResponse, CountsResponse, CreateItemRequest, ItemResponse, ItemWithValueResponse, QueryParams, UpdateItemRequest,
};
use crate::server::AppState;
use crate::{SearchParams, SearchResponse};
use chamber_vault::{ItemKind, NewItem};

/// # Errors
/// - `ApiError::Forbidden`: Returned if the authenticated user lacks the required `read:items` scope.
/// - `ApiError::BadRequest`: Returned if the vault is locked and cannot be accessed.
/// - `ApiError::VaultError`: Returned if there is an error accessing or listing items from the vault.
pub async fn list_items(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Query(params): Query<QueryParams>,
) -> ApiResult<Json<ApiResponse<crate::models::ListItemsResponse>>> {
    if !claims.has_scope("read:items") {
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

    // Apply filters
    let mut filtered_items = items;

    // Filter by kind
    if let Some(kind) = &params.kind {
        filtered_items.retain(|item| item.kind.as_str().eq_ignore_ascii_case(kind));
    }

    // Filter by search query
    if let Some(query) = &params.query {
        let query_lower = query.to_lowercase();
        filtered_items.retain(|item| item.name.to_lowercase().contains(&query_lower));
    }

    // Sort items
    match params.sort.as_deref() {
        Some("name") => filtered_items.sort_by(|a, b| a.name.cmp(&b.name)),
        Some("kind") => filtered_items.sort_by(|a, b| a.kind.as_str().cmp(b.kind.as_str())),
        Some("updated_at") => filtered_items.sort_by(|a, b| a.updated_at.cmp(&b.updated_at)),
        _ => filtered_items.sort_by(|a, b| a.name.cmp(&b.name)), // default
    }

    if params.order.as_deref() == Some("desc") {
        filtered_items.reverse();
    }

    let total = filtered_items.len();

    // Apply pagination
    let items: Vec<ItemResponse> = filtered_items
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .map(|item| ItemResponse {
            id: item.id,
            name: item.name,
            kind: item.kind.as_str().to_string(),
            created_at: DateTime::from_timestamp(item.created_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
            updated_at: DateTime::from_timestamp(item.updated_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
            has_value: !item.value.is_empty(),
            value_length: item.value.len(),
            preview: if item.value.len() > 20 {
                Some(format!("{}...", &item.value[..17]))
            } else {
                None
            },
        })
        .collect();

    let response = crate::models::ListItemsResponse { items, total };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
/// - `ApiError::Forbidden`: Returned if the authenticated user lacks the required `read:items` scope.
/// - `ApiError::BadRequest`: Returned if the vault is locked and cannot be accessed.
/// - `ApiError::VaultError`: Returned if there is an error accessing or listing items from the vault.
/// - `ApiError::NotFound`: Returned if the item with the specified ID is not found.
pub async fn get_item(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Path(id): Path<u64>,
) -> ApiResult<Json<ApiResponse<ItemResponse>>> {
    if !claims.has_scope("read:items") {
        return Err(ApiError::Forbidden);
    }

    if !state.auth.is_vault_unlocked() {
        return Err(ApiError::BadRequest("Vault is locked".to_string()));
    }

    let vault = state.vault.lock().await;
    let items = vault.list_items().map_err(|e| ApiError::VaultError(e.to_string()))?;
    drop(vault);

    let item = items
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| ApiError::NotFound("Item not found".to_string()))?;

    let response = ItemResponse {
        id: item.id,
        name: item.name,
        kind: item.kind.as_str().to_string(),
        created_at: DateTime::from_timestamp(item.created_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
        updated_at: DateTime::from_timestamp(item.updated_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
        has_value: !item.value.is_empty(),
        value_length: item.value.len(),
        preview: if item.value.len() > 20 {
            Some(format!("{}...", &item.value[..17]))
        } else {
            None
        },
    };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
/// - `ApiError::Forbidden`: Returned if the authenticated user lacks the required `reveal:values` scope.
/// - `ApiError::BadRequest`: Returned if the vault is locked and cannot be accessed.
/// - `ApiError::VaultError`: Returned if there is an error accessing or listing items from the vault.
/// - `ApiError::NotFound`: Returned if the item with the specified ID is not found.
pub async fn get_item_value(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Path(id): Path<u64>,
) -> ApiResult<Json<ApiResponse<ItemWithValueResponse>>> {
    if !claims.has_scope("reveal:values") {
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

    let item = items
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| ApiError::NotFound("Item not found".to_string()))?;

    let response = ItemWithValueResponse {
        id: item.id,
        name: item.name,
        kind: item.kind.as_str().to_string(),
        value: item.value,
        created_at: DateTime::from_timestamp(item.created_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
        updated_at: DateTime::from_timestamp(item.updated_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'write:items' scope
/// - The vault is locked
/// - The item name or value is empty
/// - The item kind is invalid
/// - There are issues with accessing or updating the vault
/// - The created item cannot be retrieved
pub async fn create_item(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Json(request): Json<CreateItemRequest>,
) -> ApiResult<Json<ApiResponse<ItemResponse>>> {
    if !claims.has_scope("write:items") {
        return Err(ApiError::Forbidden);
    }

    if !state.auth.is_vault_unlocked() {
        return Err(ApiError::BadRequest("Vault is locked".to_string()));
    }

    // Validate input
    if request.name.trim().is_empty() {
        return Err(ApiError::ValidationError("Name cannot be empty".to_string()));
    }

    if request.value.trim().is_empty() {
        return Err(ApiError::ValidationError("Value cannot be empty".to_string()));
    }

    // Parse item kind
    let kind = *ItemKind::all()
        .iter()
        .find(|&k| k.as_str().eq_ignore_ascii_case(&request.kind))
        .ok_or_else(|| ApiError::ValidationError("Invalid item kind".to_string()))?;

    let new_item = NewItem {
        name: request.name.trim().to_string(),
        kind,
        value: request.value,
    };

    state
        .vault
        .lock()
        .await
        .create_item(&new_item)
        .map_err(|e| ApiError::VaultError(e.to_string()))?;

    // Get the created item to return
    let items = state
        .vault
        .lock()
        .await
        .list_items()
        .map_err(|e| ApiError::VaultError(e.to_string()))?;

    let created_item = items
        .into_iter()
        .find(|item| item.name == new_item.name && item.kind == new_item.kind)
        .ok_or_else(|| ApiError::InternalError("Failed to retrieve created item".to_string()))?;

    let response = ItemResponse {
        id: created_item.id,
        name: created_item.name,
        kind: created_item.kind.as_str().to_string(),
        created_at: DateTime::from_timestamp(created_item.created_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
        updated_at: DateTime::from_timestamp(created_item.updated_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
        has_value: !created_item.value.is_empty(),
        value_length: created_item.value.len(),
        preview: None,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'write:items' scope
/// - The vault is locked
/// - The item value is empty
/// - No fields are provided for update
/// - The item is not found
/// - There are issues with accessing or updating the vault
pub async fn update_item(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Path(id): Path<u64>,
    Json(request): Json<UpdateItemRequest>,
) -> ApiResult<Json<ApiResponse<ItemResponse>>> {
    if !claims.has_scope("write:items") {
        return Err(ApiError::Forbidden);
    }

    if !state.auth.is_vault_unlocked() {
        return Err(ApiError::BadRequest("Vault is locked".to_string()));
    }

    // For now, we can only update the value
    if let Some(value) = request.value {
        if value.trim().is_empty() {
            return Err(ApiError::ValidationError("Value cannot be empty".to_string()));
        }

        state.vault.lock().await.update_item(id, &value).map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("Item not found") || error_msg.contains("not found") {
                ApiError::NotFound("Item not found".to_string())
            } else {
                ApiError::VaultError(error_msg)
            }
        })?;

        // Get the updated item
        let items = state
            .vault
            .lock()
            .await
            .list_items()
            .map_err(|e| ApiError::VaultError(e.to_string()))?;

        let updated_item = items
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| ApiError::NotFound("Item not found".to_string()))?;

        let response = ItemResponse {
            id: updated_item.id,
            name: updated_item.name,
            kind: updated_item.kind.as_str().to_string(),
            created_at: DateTime::from_timestamp(updated_item.created_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
            updated_at: DateTime::from_timestamp(updated_item.updated_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
            has_value: !updated_item.value.is_empty(),
            value_length: updated_item.value.len(),
            preview: None,
        };

        Ok(Json(ApiResponse::new(response)))
    } else {
        Err(ApiError::ValidationError("No fields to update".to_string()))
    }
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'write:items' scope
/// - The vault is locked
/// - There are issues with accessing the vault or deleting the item
pub async fn delete_item(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Path(id): Path<u64>,
) -> ApiResult<Json<ApiResponse<String>>> {
    if !claims.has_scope("write:items") {
        return Err(ApiError::Forbidden);
    }

    if !state.auth.is_vault_unlocked() {
        return Err(ApiError::BadRequest("Vault is locked".to_string()));
    }

    state
        .vault
        .lock()
        .await
        .delete_item(id)
        .map_err(|e| ApiError::VaultError(e.to_string()))?;

    Ok(Json(ApiResponse::new("Item deleted successfully".to_string())))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'read:items' scope
/// - The vault is locked
/// - There are issues with accessing the vault or retrieving items
pub async fn get_counts(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
) -> ApiResult<Json<ApiResponse<CountsResponse>>> {
    if !claims.has_scope("read:items") {
        return Err(ApiError::Forbidden);
    }

    if !state.auth.is_vault_unlocked() {
        return Err(ApiError::BadRequest("Vault is locked".to_string()));
    }

    let vault = state.vault.lock().await;
    let items = vault.list_items().map_err(|e| ApiError::VaultError(e.to_string()))?;
    drop(vault);

    let mut by_kind: HashMap<String, usize> = HashMap::new();
    for item in &items {
        *by_kind.entry(item.kind.as_str().to_string()).or_insert(0) += 1;
    }

    let response = CountsResponse {
        total: items.len(),
        by_kind,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// # Errors
///
/// This function returns an error if:
/// - The user does not have the required 'reveal:values' scope
/// - The vault is locked
/// - There are issues with accessing the vault or retrieving items
/// - The clipboard access or copy operation fails
/// - The requested item is not found
pub async fn copy_item_to_clipboard(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Path(id): Path<u64>,
) -> ApiResult<Json<ApiResponse<String>>> {
    if !claims.has_scope("reveal:values") {
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

    let item = items
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| ApiError::NotFound("Item not found".to_string()))?;

    // Copy to clipboard using arboard
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| ApiError::InternalError(format!("Failed to access clipboard: {e}")))?;

    clipboard
        .set_text(&item.value)
        .map_err(|e| ApiError::InternalError(format!("Failed to copy to clipboard: {e}")))?;

    Ok(Json(ApiResponse::new(format!("Copied '{}' to clipboard", item.name))))
}

/// # Errors
/// - Returns `ApiError::Forbidden` if the user does not have the `vault:read` permission.
/// - Returns `ApiError::BadRequest` if the vault is locked.
/// - Returns `ApiError::VaultError` if an issue occurs during item retrieval from the vault.
/// - Returns `ApiError::InternalError` if querying time cannot be calculated.
pub async fn search_items(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Query(params): Query<SearchParams>,
) -> ApiResult<Json<ApiResponse<SearchResponse>>> {
    let start_time = Instant::now();

    // Check permissions
    if !claims.has_scope("vault:read") {
        return Err(ApiError::Forbidden);
    }

    if !state.auth.is_vault_unlocked() {
        return Err(ApiError::BadRequest("Vault is locked".to_string()));
    }

    let vault = state.vault.lock().await;
    let all_items = vault.list_items().map_err(|e| ApiError::VaultError(e.to_string()))?;
    drop(vault);

    // Convert to ItemResponse format
    let mut items: Vec<ItemResponse> = all_items
        .into_iter()
        .map(|item| {
            let preview = if item.value.len() > 50 {
                Some(format!("{}...", &item.value[..47]))
            } else if !item.value.is_empty() {
                Some(item.value.clone())
            } else {
                None
            };

            ItemResponse {
                id: item.id,
                name: item.name,
                kind: item.kind.as_str().to_string(),
                created_at: DateTime::from_timestamp(item.created_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
                updated_at: DateTime::from_timestamp(item.updated_at.unix_timestamp(), 0).unwrap_or(Utc::now()),
                has_value: !item.value.is_empty(),
                value_length: item.value.len(),
                preview,
            }
        })
        .collect();

    let total_available = items.len();

    // Apply filtering
    items = apply_search_filters(items, &params);
    let total_found = items.len();

    // Apply sorting
    items = apply_search_sorting(items, &params)?;

    // Apply pagination
    let has_more = params.offset + params.limit < total_found;
    let next_offset = if has_more {
        Some(params.offset + params.limit)
    } else {
        None
    };

    let paginated_items: Vec<ItemResponse> = items.into_iter().skip(params.offset).take(params.limit).collect();

    let query_time_ms = u64::try_from(start_time.elapsed().as_millis())
        .map_err(|_| ApiError::InternalError("Failed to calculate query time".to_string()))?
        .max(1);

    let response = SearchResponse {
        items: paginated_items,
        total_found,
        total_available,
        query_time_ms,
        has_more,
        next_offset,
    };

    Ok(Json(ApiResponse::new(response)))
}

fn apply_search_filters(mut items: Vec<ItemResponse>, params: &SearchParams) -> Vec<ItemResponse> {
    // Get search query (prefer 'q' over 'query')
    let search_query = params.q.as_ref().or(params.query.as_ref());
    let case_sensitive = params.case_sensitive.unwrap_or(false);
    let fuzzy = params.fuzzy.unwrap_or(false);

    // Filter by item type/kind
    if let Some(ref kind_filter) = params.kind {
        let kind_lower = kind_filter.to_lowercase();
        items.retain(|item| {
            let item_kind = item.kind.to_lowercase();
            item_kind == kind_lower || item_kind.contains(&kind_lower)
        });
    }

    // Filter by name-only search
    if let Some(ref name_filter) = params.name {
        items.retain(|item| {
            if case_sensitive {
                if fuzzy {
                    fuzzy_match(&item.name, name_filter)
                } else {
                    item.name.contains(name_filter)
                }
            } else if fuzzy {
                fuzzy_match(&item.name.to_lowercase(), &name_filter.to_lowercase())
            } else {
                item.name.to_lowercase().contains(&name_filter.to_lowercase())
            }
        });
    }

    // Apply general search query (searches name, kind, and preview)
    if let Some(query) = search_query {
        items.retain(|item| {
            let search_in_name = if case_sensitive {
                if fuzzy {
                    fuzzy_match(&item.name, query)
                } else {
                    item.name.contains(query)
                }
            } else if fuzzy {
                fuzzy_match(&item.name.to_lowercase(), &query.to_lowercase())
            } else {
                item.name.to_lowercase().contains(&query.to_lowercase())
            };

            let search_in_kind = if case_sensitive {
                item.kind.contains(query)
            } else {
                item.kind.to_lowercase().contains(&query.to_lowercase())
            };

            let search_in_preview = item.preview.as_ref().is_some_and(|preview| {
                if case_sensitive {
                    if fuzzy {
                        fuzzy_match(preview, query)
                    } else {
                        preview.contains(query)
                    }
                } else if fuzzy {
                    fuzzy_match(&preview.to_lowercase(), &query.to_lowercase())
                } else {
                    preview.to_lowercase().contains(&query.to_lowercase())
                }
            });

            search_in_name || search_in_kind || search_in_preview
        });
    }

    items
}

fn apply_search_sorting(mut items: Vec<ItemResponse>, params: &SearchParams) -> ApiResult<Vec<ItemResponse>> {
    let sort_field = params.sort.as_deref().unwrap_or("name");
    let sort_order = params.order.as_deref().unwrap_or("asc");

    match sort_field {
        "name" => {
            items.sort_by(|a, b| {
                let cmp = a.name.cmp(&b.name);
                if sort_order == "desc" { cmp.reverse() } else { cmp }
            });
        }
        "kind" => {
            items.sort_by(|a, b| {
                let cmp = a.kind.cmp(&b.kind);
                if sort_order == "desc" { cmp.reverse() } else { cmp }
            });
        }
        "created_at" => {
            items.sort_by(|a, b| {
                let cmp = a.created_at.cmp(&b.created_at);
                if sort_order == "desc" { cmp.reverse() } else { cmp }
            });
        }
        "updated_at" => {
            items.sort_by(|a, b| {
                let cmp = a.updated_at.cmp(&b.updated_at);
                if sort_order == "desc" { cmp.reverse() } else { cmp }
            });
        }
        "value_length" => {
            items.sort_by(|a, b| {
                let cmp = a.value_length.cmp(&b.value_length);
                if sort_order == "desc" { cmp.reverse() } else { cmp }
            });
        }
        _ => {
            return Err(ApiError::ValidationError(format!("Invalid sort field: {sort_field}")));
        }
    }

    Ok(items)
}

// Simple fuzzy matching algorithm (Levenshtein distance-based)
fn fuzzy_match(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    if text.is_empty() {
        return false;
    }

    // Simple fuzzy logic: allow up to 20% character differences
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_precision_loss)]
    let max_distance = (pattern.len() as f64 * 0.2).ceil() as usize;
    levenshtein_distance(text, pattern) <= max_distance
}

fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    #[allow(clippy::needless_range_loop)]
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = usize::from(s1_chars[i - 1] != s2_chars[j - 1]);
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len1][len2]
}
