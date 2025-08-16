mod common;

use crate::common::TestContext;
use crate::common::fixtures::sample_generate_password_request;
use chamber_api::models::{ApiResponse, GeneratePasswordRequest, PasswordResponse};

#[tokio::test]
async fn test_generate_password() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let generate_request = sample_generate_password_request();
    let response = ctx
        .server
        .post("/api/v1/passwords/generate")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&generate_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<PasswordResponse> = response.json();
    assert_eq!(body.data.password.len(), generate_request.length);
    assert!(!body.data.strength.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_generate_memorable_password() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let generate_request = serde_json::json!({
        "word_count": 4,
        "separator": "-",
        "capitalize": true,
        "include_numbers": true
    });

    let response = ctx
        .server
        .post("/api/v1/passwords/memorable")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&generate_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<PasswordResponse> = response.json();
    assert!(!body.data.password.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_generate_password_custom_length() -> color_eyre::Result<()> {
    let mut ctx = TestContext::new()?;
    ctx.login().await?;

    let generate_request = GeneratePasswordRequest {
        length: 32,
        include_uppercase: true,
        include_lowercase: true,
        include_digits: true,
        include_symbols: false,
        exclude_ambiguous: true,
    };

    let response = ctx
        .server
        .post("/api/v1/passwords/generate")
        .authorization_bearer(ctx.auth_token.as_ref().unwrap())
        .json(&generate_request)
        .await;

    response.assert_status_ok();

    let body: ApiResponse<PasswordResponse> = response.json();
    assert_eq!(body.data.password.len(), 32);

    // Check that no symbols are included
    assert!(!body.data.password.chars().any(|c| "!@#$%^&*()".contains(c)));

    Ok(())
}
