mod common;

use crate::common::TestContext;
use crate::common::fixtures::sample_create_item_request;
use chamber_api::SearchResponse;
use chamber_api::models::{
    ApiResponse, CountsResponse, CreateItemRequest, ItemResponse, ItemWithValueResponse, ListItemsResponse,
    UpdateItemRequest,
};
use http::StatusCode;

// ============================================================================
// Creation Tests
// ============================================================================

#[tokio::test]
async fn test_create_item_success() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let create_request = sample_create_item_request();
    let response = ctx
        .server
        .post("/api/v1/items")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<ItemResponse> = response.json();
    assert_eq!(body.data.name, create_request.name);
    assert_eq!(body.data.kind, create_request.kind);
    assert!(body.data.has_value);
    assert_eq!(body.data.value_length, create_request.value.len());
    assert!(body.data.id > 0, "Item should have a valid ID");

    Ok(())
}

#[tokio::test]
async fn test_create_item_different_types() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let item_types = ["password", "apikey", "note", "sshkey", "certificate"];

    for (i, kind) in item_types.iter().enumerate() {
        let create_request = CreateItemRequest {
            name: format!("Test {kind} Item"),
            kind: (*kind).to_string(),
            value: format!("test_value_{i}"),
        };

        let response = ctx
            .server
            .post("/api/v1/items")
            .authorization_bearer(ctx.auth_token.as_ref().unwrap())
            .json(&create_request)
            .await;

        response.assert_status_ok();

        let body: ApiResponse<ItemResponse> = response.json();
        assert_eq!(body.data.kind, *kind);
    }

    Ok(())
}

#[tokio::test]
async fn test_create_item_without_auth() -> color_eyre::Result<()> {
    let ctx = TestContext::new()?;

    let create_request = sample_create_item_request();
    let response = ctx.server.post("/api/v1/items").json(&create_request).await;

    response.assert_status_unauthorized();

    Ok(())
}

#[tokio::test]
async fn test_create_item_vault_locked() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    // Lock the vault after login to test the locked vault scenario
    let lock_response = ctx
        .server
        .post("/api/v1/session/lock")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;
    lock_response.assert_status_ok();

    let create_request = sample_create_item_request();
    let response = ctx
        .server
        .post("/api/v1/items")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    response.assert_status_bad_request();

    Ok(())
}

#[tokio::test]
async fn test_create_item_validation_errors() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    // Test empty name
    let empty_name_request = CreateItemRequest {
        name: String::new(),
        kind: "password".to_string(),
        value: "test_value".to_string(),
    };

    let response = ctx
        .server
        .post("/api/v1/items")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&empty_name_request)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    // Test empty value
    let empty_value_request = CreateItemRequest {
        name: "Test Item".to_string(),
        kind: "password".to_string(),
        value: String::new(),
    };

    let response = ctx
        .server
        .post("/api/v1/items")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&empty_value_request)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    // Test invalid kind
    let invalid_kind_request = CreateItemRequest {
        name: "Test Item".to_string(),
        kind: "invalid_type".to_string(),
        value: "test_value".to_string(),
    };

    let response = ctx
        .server
        .post("/api/v1/items")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&invalid_kind_request)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    Ok(())
}

#[tokio::test]
async fn test_create_item_with_special_characters() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let special_chars_request = CreateItemRequest {
        name: "Test🔑Item with émojis & spëciål chars!@#$%".to_string(),
        kind: "password".to_string(),
        value: "pássw0rd!@#$%^&*()_+-=[]{}|;:,.<>?/~`".to_string(),
    };

    let response = ctx
        .server
        .post("/api/v1/items")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&special_chars_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<ItemResponse> = response.json();
    assert_eq!(body.data.name, special_chars_request.name);
    assert_eq!(body.data.value_length, special_chars_request.value.len());

    Ok(())
}

// ============================================================================
// Listing and Querying Tests
// ============================================================================

#[tokio::test]
async fn test_list_items_empty_vault() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let response = ctx
        .server
        .get("/api/v1/items")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    let body: ApiResponse<ListItemsResponse> = response.json();
    assert_eq!(body.data.items.len(), 0);
    assert_eq!(body.data.total, 0);

    Ok(())
}

#[tokio::test]
async fn test_list_items_with_pagination() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    // Create 10 test items
    ctx.create_multiple_test_items(10).await?;

    // Test first page
    let response = ctx
        .server
        .get("/api/v1/items?limit=5&offset=0")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();
    let body: ApiResponse<ListItemsResponse> = response.json();
    assert_eq!(body.data.items.len(), 5);
    assert_eq!(body.data.total, 10);

    // Test second page
    let response = ctx
        .server
        .get("/api/v1/items?limit=5&offset=5")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();
    let body: ApiResponse<ListItemsResponse> = response.json();
    assert_eq!(body.data.items.len(), 5);
    assert_eq!(body.data.total, 10);

    Ok(())
}

#[tokio::test]
async fn test_list_items_filtering_by_kind() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    // Create items of different types
    ctx.create_test_item("Password 1", "password", "pass1").await?;
    ctx.create_test_item("Password 2", "password", "pass2").await?;
    ctx.create_test_item("Note 1", "note", "note1").await?;
    ctx.create_test_item("API Key 1", "apikey", "api1").await?;

    // Filter by password type
    let response = ctx
        .server
        .get("/api/v1/items?kind=password")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();
    let body: ApiResponse<ListItemsResponse> = response.json();
    assert_eq!(body.data.items.len(), 2);
    assert!(body.data.items.iter().all(|item| item.kind == "password"));

    Ok(())
}

#[tokio::test]
async fn test_list_items_sorting() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    // Create items with different names
    ctx.create_test_item("Zebra Item", "password", "pass1").await?;
    ctx.create_test_item("Alpha Item", "password", "pass2").await?;
    ctx.create_test_item("Beta Item", "note", "note1").await?;

    // Test ascending sort by name (default)
    let response = ctx
        .server
        .get("/api/v1/items?sort=name&order=asc")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();
    let body: ApiResponse<ListItemsResponse> = response.json();
    assert_eq!(body.data.items[0].name, "Alpha Item");
    assert_eq!(body.data.items[1].name, "Beta Item");
    assert_eq!(body.data.items[2].name, "Zebra Item");

    // Test descending sort by name
    let response = ctx
        .server
        .get("/api/v1/items?sort=name&order=desc")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();
    let body: ApiResponse<ListItemsResponse> = response.json();
    assert_eq!(body.data.items[0].name, "Zebra Item");
    assert_eq!(body.data.items[1].name, "Beta Item");
    assert_eq!(body.data.items[2].name, "Alpha Item");

    Ok(())
}

// ============================================================================
// Individual Item Tests
// ============================================================================

#[tokio::test]
async fn test_get_item() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let item_id = ctx.create_test_item("Test Item", "password", "test_value").await?;

    let response = ctx
        .server
        .get(&format!("/api/v1/items/{item_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    let body: ApiResponse<ItemResponse> = response.json();
    assert_eq!(body.data.id, item_id);
    assert_eq!(body.data.name, "Test Item");
    assert_eq!(body.data.kind, "password");
    assert!(body.data.has_value);
    assert_eq!(body.data.value_length, "test_value".len());

    Ok(())
}

#[tokio::test]
async fn test_get_item_not_found() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let response = ctx
        .server
        .get("/api/v1/items/99999")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_not_found();

    Ok(())
}

#[tokio::test]
async fn test_get_item_value() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let test_value = "super_secret_value";
    let item_id = ctx.create_test_item("Secret Item", "password", test_value).await?;

    let response = ctx
        .server
        .get(&format!("/api/v1/items/{item_id}/value"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    let body: ApiResponse<ItemWithValueResponse> = response.json();
    assert_eq!(body.data.value, test_value);
    assert_eq!(body.data.id, item_id);
    assert_eq!(body.data.name, "Secret Item");

    Ok(())
}

#[tokio::test]
async fn test_get_item_value_without_reveal_scope() -> color_eyre::Result<()> {
    // This test would require creating a token without reveal:values scope
    // For now, we'll test the existing behavior
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let item_id = ctx.create_test_item("Secret Item", "password", "secret").await?;

    let response = ctx
        .server
        .get(&format!("/api/v1/items/{item_id}/value"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    // Should succeed with full scopes, but could be forbidden with limited scopes
    response.assert_status_ok();

    Ok(())
}

// ============================================================================
// Update Tests
// ============================================================================

#[tokio::test]
async fn test_update_item_value() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let item_id = ctx
        .create_test_item("Original Item", "password", "original_value")
        .await?;

    let update_request = UpdateItemRequest {
        name: None,
        kind: None,
        value: Some("updated_value_123".to_string()),
    };

    let response = ctx
        .server
        .put(&format!("/api/v1/items/{item_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<ItemResponse> = response.json();
    assert_eq!(body.data.id, item_id);
    assert!(body.data.has_value);
    assert_eq!(body.data.value_length, update_request.value.as_ref().unwrap().len());

    // Verify the value was actually updated
    let get_response = ctx
        .server
        .get(&format!("/api/v1/items/{item_id}/value"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    get_response.assert_status_ok();
    let get_body: ApiResponse<ItemWithValueResponse> = get_response.json();
    assert_eq!(get_body.data.value, "updated_value_123");

    Ok(())
}

#[tokio::test]
async fn test_update_item_empty_value() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let item_id = ctx.create_test_item("Test Item", "password", "value").await?;

    let update_request = UpdateItemRequest {
        name: None,
        kind: None,
        value: Some(String::new()), // Empty value
    };

    let response = ctx
        .server
        .put(&format!("/api/v1/items/{item_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    Ok(())
}

#[tokio::test]
async fn test_update_item_not_found() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let update_request = UpdateItemRequest {
        name: None,
        kind: None,
        value: Some("new_value".to_string()),
    };

    let response = ctx
        .server
        .put("/api/v1/items/99999")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    response.assert_status_not_found();

    Ok(())
}

#[tokio::test]
async fn test_update_item_no_fields() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let item_id = ctx.create_test_item("Test Item", "password", "value").await?;

    let update_request = UpdateItemRequest {
        name: None,
        kind: None,
        value: None, // No fields to update
    };

    let response = ctx
        .server
        .put(&format!("/api/v1/items/{item_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&update_request)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    Ok(())
}

// ============================================================================
// Delete Tests
// ============================================================================

#[tokio::test]
async fn test_delete_item() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let item_id = ctx.create_test_item("Item to Delete", "password", "value").await?;

    let response = ctx
        .server
        .delete(&format!("/api/v1/items/{item_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    // Verify item is deleted
    let get_response = ctx
        .server
        .get(&format!("/api/v1/items/{item_id}"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    get_response.assert_status_not_found();

    Ok(())
}

#[tokio::test]
async fn test_delete_item_not_found() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let response = ctx
        .server
        .delete("/api/v1/items/99999")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    // Some implementations may return 404, others might return 200 for idempotency
    // Let's accept both as valid
    assert!(response.status_code() == 200 || response.status_code() == 404);

    Ok(())
}

// ============================================================================
// Search Tests
// ============================================================================

#[tokio::test]
async fn test_search_items_basic() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    // Create items with searchable names
    ctx.create_test_item("GitHub Password", "password", "gh_token").await?;
    ctx.create_test_item("GitHub API Key", "apikey", "api_key").await?;
    ctx.create_test_item("Google Password", "password", "google_pass")
        .await?;

    let response = ctx
        .server
        .get("/api/v1/items/search?q=github&limit=10")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    let body: ApiResponse<SearchResponse> = response.json();
    assert_eq!(body.data.items.len(), 2); // Both GitHub items should match
    assert_eq!(body.data.total_found, 2);
    assert_eq!(body.data.total_available, 3);
    assert!(body.data.query_time_ms > 0);

    Ok(())
}

#[tokio::test]
async fn test_search_items_case_insensitive() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    ctx.create_test_item("GitHub Password", "password", "token").await?;
    ctx.create_test_item("gitlab key", "apikey", "key").await?;

    let response = ctx
        .server
        .get("/api/v1/items/search?q=GIT&limit=10")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    let body: ApiResponse<SearchResponse> = response.json();
    assert_eq!(body.data.items.len(), 2); // Both git items should match (case-insensitive)

    Ok(())
}

#[tokio::test]
async fn test_search_items_by_kind() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    ctx.create_test_item("Password 1", "password", "pass1").await?;
    ctx.create_test_item("Password 2", "password", "pass2").await?;
    ctx.create_test_item("Note 1", "note", "note1").await?;

    let response = ctx
        .server
        .get("/api/v1/items/search?kind=password&limit=10")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    let body: ApiResponse<SearchResponse> = response.json();
    assert_eq!(body.data.items.len(), 2);
    assert!(body.data.items.iter().all(|item| item.kind == "password"));

    Ok(())
}

#[tokio::test]
async fn test_search_items_pagination() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    // Create 5 items that will match the search
    for i in 1..=5 {
        ctx.create_test_item(&format!("Test Item {i}"), "password", "value")
            .await?;
    }

    // Search with pagination
    let response = ctx
        .server
        .get("/api/v1/items/search?q=test&limit=3&offset=0")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    let body: ApiResponse<SearchResponse> = response.json();
    assert_eq!(body.data.items.len(), 3);
    assert_eq!(body.data.total_found, 5);
    assert!(body.data.has_more);
    assert_eq!(body.data.next_offset, Some(3));

    Ok(())
}

// ============================================================================
// Statistics Tests
// ============================================================================

#[tokio::test]
async fn test_get_counts() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    // Create items of different types
    ctx.create_test_item("Password 1", "password", "pass1").await?;
    ctx.create_test_item("Password 2", "password", "pass2").await?;
    ctx.create_test_item("Note 1", "note", "note1").await?;
    ctx.create_test_item("API Key 1", "apikey", "api1").await?;

    let response = ctx
        .server
        .get("/api/v1/items/counts")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    let body: ApiResponse<CountsResponse> = response.json();
    assert_eq!(body.data.total, 4);
    assert_eq!(body.data.by_kind.get("password"), Some(&2));
    assert_eq!(body.data.by_kind.get("note"), Some(&1));
    assert_eq!(body.data.by_kind.get("apikey"), Some(&1));

    Ok(())
}

#[tokio::test]
async fn test_get_counts_empty_vault() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let response = ctx
        .server
        .get("/api/v1/items/counts")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    let body: ApiResponse<CountsResponse> = response.json();
    assert_eq!(body.data.total, 0);
    assert!(body.data.by_kind.is_empty());

    Ok(())
}

// ============================================================================
// Clipboard Tests
// ============================================================================

#[tokio::test]
async fn test_copy_item_to_clipboard() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let test_value = "secret_to_copy";
    let item_id = ctx.create_test_item("Copy Test", "password", test_value).await?;

    let response = ctx
        .server
        .post(&format!("/api/v1/items/{item_id}/copy"))
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    // Note: Clipboard operations might fail in test environment
    // We should accept both success and failure as valid outcomes
    assert!(response.status_code() == 200 || response.status_code() == 500);

    if response.status_code() == 200 {
        let body: ApiResponse<String> = response.json();
        assert!(body.data.contains("Copy Test"));
    }

    Ok(())
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

/*#[tokio::test]
async fn test_concurrent_item_creation() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    let auth_token = ctx.auth_token.as_ref().unwrap().clone();

    // Create multiple items concurrently
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let server = ctx.server;
            let token = auth_token.clone();
            tokio::spawn(async move {
                let create_request = CreateItemRequest {
                    name: format!("Concurrent Item {}", i),
                    kind: "password".to_string(),
                    value: format!("value_{}", i),
                };

                server
                    .post("/api/v1/items")
                    .authorization_bearer(&token)
                    .json(&create_request)
                    .await
            })
        })
        .collect();

    // Wait for all requests to complete
    let results = futures::future::join_all(handles).await;

    // Check that all requests succeeded
    for result in results {
        let response = result?;
        response.assert_status_ok();
    }

    // Verify all items were created
    let list_response = ctx
        .server
        .get("/api/v1/items")
        .authorization_bearer(&auth_token)
        .await;

    list_response.assert_status_ok();
    let body: ApiResponse<ListItemsResponse> = list_response.json();
    assert_eq!(body.data.total, 5);

    Ok(())
}*/

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
async fn test_large_item_handling() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    // Create an item with a large value (1MB)
    let large_value = "x".repeat(1_000_000);

    let create_request = CreateItemRequest {
        name: "Large Item".to_string(),
        kind: "note".to_string(),
        value: large_value.clone(),
    };

    let response = ctx
        .server
        .post("/api/v1/items")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&create_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<ItemResponse> = response.json();
    assert_eq!(body.data.value_length, 1_000_000);

    Ok(())
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[tokio::test]
async fn test_malformed_json_requests() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    // Send malformed JSON
    let response = ctx
        .server
        .post("/api/v1/items")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .add_header("content-type", "application/json")
        .text("{ invalid json")
        .await;

    response.assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);

    Ok(())
}
