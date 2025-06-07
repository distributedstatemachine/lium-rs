//! Error types for Lium CLI application
//!
//! This module re-exports core domain errors from lium-core and adds
//! application-specific errors for I/O operations, networking, etc.

// Re-export core domain errors
pub use lium_core::{LiumError as CoreError, Result as CoreResult};

use thiserror::Error;

/// Application-level errors that wrap core errors and add I/O-specific errors
#[derive(Error, Debug)]
pub enum LiumError {
    #[error("Core domain error: {0}")]
    Core(#[from] CoreError),

    #[error("API error: {0}")]
    Api(#[from] ApiError),

    #[error("SSH error: {0}")]
    Ssh(#[from] SshError),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Docker error: {0}")]
    Docker(#[from] DockerError),

    #[error("Utils error: {0}")]
    Utils(#[from] lium_utils::UtilsError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Command failed: {0}")]
    Command(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("Other: {0}")]
    Other(String),
}

/// API client errors
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Rate limited")]
    RateLimited,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Request timeout")]
    Timeout,

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
}

/// SSH connection errors
#[derive(Error, Debug)]
pub enum SshError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("File transfer failed: {0}")]
    TransferFailed(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("SSH2 error: {0}")]
    Ssh2(#[from] ssh2::Error),
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found")]
    NotFound,

    #[error("Invalid config format: {0}")]
    InvalidFormat(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },

    #[error("Config directory creation failed: {0}")]
    DirectoryCreationFailed(String),

    #[error("INI parsing error: {0}")]
    IniError(String),
}

/// Docker operation errors
#[derive(Error, Debug)]
pub enum DockerError {
    #[error("Docker daemon not running")]
    DaemonNotRunning,

    #[error("Docker not available: {0}")]
    NotAvailable(String),

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Image build failed: {0}")]
    BuildFailed(String),

    #[error("Image push failed: {0}")]
    PushFailed(String),

    #[error("Login failed: {0}")]
    LoginFailed(String),

    #[error("Invalid image name: {0}")]
    InvalidImageName(String),

    #[error("Bollard error: {0}")]
    Bollard(#[from] bollard::errors::Error),
}

pub type Result<T> = std::result::Result<T, LiumError>;

impl From<dialoguer::Error> for LiumError {
    fn from(err: dialoguer::Error) -> Self {
        LiumError::Other(format!("Input error: {}", err))
    }
}
