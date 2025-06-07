use thiserror::Error;

/// API-specific errors for lium-api
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

pub type Result<T> = std::result::Result<T, ApiError>;
