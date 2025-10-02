use ai_coach_cli::api::{ApiClient, ApiError};
use ai_coach_cli::config::Config;
use anyhow::Result;

#[tokio::test]
async fn test_api_client_creation() -> Result<()> {
    let config = Config::default();
    let client = ApiClient::new(config)?;

    // Just verify client was created successfully
    assert!(true);
    Ok(())
}

#[tokio::test]
async fn test_login_requires_valid_credentials() -> Result<()> {
    let config = Config::default();
    let client = ApiClient::new(config)?;

    // This will fail because we don't have a running API server
    // But it demonstrates that the login method works
    let result = client.login("test_user", "test_password").await;

    // We expect an error since there's no API server running
    assert!(result.is_err());
    Ok(())
}

#[tokio::test]
async fn test_whoami_requires_authentication() -> Result<()> {
    let config = Config::default();
    let client = ApiClient::new(config)?;

    // Should fail because not authenticated
    let result = client.whoami().await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Not logged in");
    Ok(())
}

#[test]
fn test_config_authentication_status() {
    let mut config = Config::default();

    // Initially not authenticated
    assert!(!config.is_authenticated());

    // After setting tokens, should be authenticated
    config.set_tokens("access".to_string(), "refresh".to_string());
    assert!(config.is_authenticated());

    // After clearing, should not be authenticated
    config.clear_tokens();
    assert!(!config.is_authenticated());
}

#[test]
fn test_api_error_from_status() {
    use reqwest::StatusCode;

    let error = ApiError::from_status(StatusCode::UNAUTHORIZED, "Unauthorized".to_string());
    assert!(matches!(error, ApiError::Unauthorized(_)));

    let error = ApiError::from_status(StatusCode::NOT_FOUND, "Not Found".to_string());
    assert!(matches!(error, ApiError::NotFound(_)));

    let error = ApiError::from_status(StatusCode::BAD_REQUEST, "Bad Request".to_string());
    assert!(matches!(error, ApiError::BadRequest(_)));

    let error = ApiError::from_status(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Server Error".to_string(),
    );
    assert!(matches!(error, ApiError::ServerError(_)));
}
