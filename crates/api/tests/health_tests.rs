mod common;

use crate::common::TestContext;
use chamber_api::models::{ApiResponse, HealthResponse};
use chamber_api::{HealthReportResponse, StatsResponse};

#[tokio::test]
async fn test_health_endpoint() -> color_eyre::Result<()> {
    let ctx = TestContext::new()?;

    let response = ctx.server.get("/api/v1/health").await;
    response.assert_status_ok();

    let body: ApiResponse<HealthResponse> = response.json();
    assert_eq!(body.data.status, "ok");
    assert!(!body.data.version.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_health_report() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    // Create some test passwords with security issues
    ctx.create_test_item("Weak Password", "password", "123").await?; // Short password
    ctx.create_test_item("Another Password", "password", "password123")
        .await?; // Common pattern
    ctx.create_test_item("Good Password", "password", "super_secure_password_with_numbers_123!@#")
        .await?;

    let response = ctx
        .server
        .get("/api/v1/health/report")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    let body: ApiResponse<HealthReportResponse> = response.json();
    assert!(body.data.total_items > 0);
    assert!(body.data.password_items > 0);
    assert!(body.data.security_score >= 0.0 && body.data.security_score <= 100.0);

    Ok(())
}

#[tokio::test]
async fn test_stats_endpoint() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;
    ctx.unlock_session().await?;

    // Create diverse items
    ctx.create_test_item("Password", "password", "pass123").await?;
    ctx.create_test_item("Note", "note", "Some note content").await?;
    ctx.create_test_item("Card", "creditcard", "4111111111111111").await?;

    let response = ctx
        .server
        .get("/api/v1/stats")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .await;

    response.assert_status_ok();

    let body: ApiResponse<StatsResponse> = response.json();
    assert_eq!(body.data.total_items, 3);
    assert_eq!(body.data.password_items, 1);
    assert_eq!(body.data.note_items, 1);
    assert_eq!(body.data.card_items, 1);
    assert!(body.data.vault_size_bytes > 0);

    Ok(())
}
