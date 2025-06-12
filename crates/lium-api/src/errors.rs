use thiserror::Error;

/// API-specific errors for the Lium API client.
///
/// This enum represents all possible error types that can occur when interacting with the Lium API.
/// It includes HTTP errors, core domain errors, utility errors, and various other error types.
///
/// # Variants
/// * `Http` - Errors related to HTTP communication, wrapped from HttpError
/// * `Core` - Errors from the core domain logic, wrapped from lium_core::LiumError
/// * `Utils` - Errors from utility functions, wrapped from lium_utils::UtilsError
/// * `Request` - Errors from HTTP requests, wrapped from reqwest::Error
/// * `Json` - Errors related to JSON serialization/deserialization
/// * `Config` - Configuration-related errors with a descriptive message
///
/// # Examples
/// ```rust
/// match result {
///     Ok(data) => println!("Success: {:?}", data),
///     Err(ApiError::Http(e)) => println!("HTTP error: {}", e),
///     Err(ApiError::Config(msg)) => println!("Config error: {}", msg),
///     // ... handle other variants
/// }
/// ```
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP error: {0}")]
    Http(#[from] HttpError),

    #[error("Core domain error: {0}")]
    Core(#[from] lium_core::LiumError),

    #[error("Utils error: {0}")]
    Utils(#[from] lium_utils::UtilsError),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

/// HTTP-specific errors that can occur during API communication.
///
/// This enum represents various HTTP-related errors that can occur when making requests
/// to the Lium API. It includes authentication errors, rate limiting, service availability,
/// and general HTTP errors.
///
/// # Variants
/// * `AuthenticationFailed` - When authentication credentials are invalid
/// * `InvalidApiKey` - When the provided API key is invalid
/// * `RateLimited` - When the request is rate limited by the API
/// * `ServiceUnavailable` - When the API service is temporarily unavailable
/// * `Timeout` - When the request times out
/// * `HttpError` - General HTTP errors with status code and message
/// * `Request` - Errors from the reqwest HTTP client
/// * `Config` - Configuration-related errors with a descriptive message
///
/// # Examples
/// ```rust
/// match http_result {
///     Ok(response) => println!("Success: {:?}", response),
///     Err(HttpError::AuthenticationFailed) => println!("Auth failed"),
///     Err(HttpError::RateLimited) => println!("Rate limited"),
///     // ... handle other variants
/// }
/// ```
#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Rate limited")]
    RateLimited,

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Request timeout")]
    Timeout,

    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

/// Type alias for Result that uses ApiError as the error type.
///
/// This is a convenience type alias that makes it easier to work with Results
/// throughout the codebase. It uses ApiError as the error type, which provides
/// a unified error handling approach.
///
/// # Examples
/// ```rust
/// fn some_function() -> Result<String> {
///     // Function implementation
///     Ok("success".to_string())
/// }
/// ```
pub type Result<T> = std::result::Result<T, ApiError>;
