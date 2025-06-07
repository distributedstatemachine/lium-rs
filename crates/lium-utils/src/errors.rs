use thiserror::Error;

/// Infrastructure-specific errors for lium-utils
#[derive(Error, Debug)]
pub enum UtilsError {
    #[error("SSH error: {0}")]
    Ssh(#[from] SshError),

    #[error("Docker error: {0}")]
    Docker(#[from] DockerError),

    #[error("GPU error: {0}")]
    Gpu(#[from] GpuError),

    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("Core domain error: {0}")]
    Core(#[from] lium_core::LiumError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Process error: {0}")]
    Process(String),
}

#[derive(Error, Debug)]
pub enum SshError {
    #[error("SSH command failed: {0}")]
    CommandFailed(String),

    #[error("SSH connection failed: {0}")]
    ConnectionFailed(String),

    #[error("SSH authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("SSH file transfer failed: {0}")]
    TransferFailed(String),

    #[error("SSH key error: {0}")]
    KeyError(String),
}

#[derive(Error, Debug)]
pub enum DockerError {
    #[error("Docker command failed: {0}")]
    CommandFailed(String),

    #[error("Docker API error: {0}")]
    ApiError(String),

    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    #[error("Image not found: {0}")]
    ImageNotFound(String),

    #[error("Docker login failed: {0}")]
    LoginFailed(String),

    #[error("Docker build failed: {0}")]
    BuildFailed(String),

    #[error("Docker push failed: {0}")]
    PushFailed(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Docker not available: {0}")]
    NotAvailable(String),

    #[error("Invalid image name: {0}")]
    InvalidImageName(String),
}

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("GPU detection failed: {0}")]
    DetectionFailed(String),

    #[error("GPU not available: {0}")]
    NotAvailable(String),

    #[error("GPU command failed: {0}")]
    CommandFailed(String),
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to parse: {0}")]
    InvalidFormat(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid value: {0}")]
    InvalidValue(String),
}

pub type Result<T> = std::result::Result<T, UtilsError>;
