use thiserror::Error;

/// Core domain errors - no I/O dependencies
#[derive(Error, Debug)]
pub enum LiumError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Domain rule violation: {0}")]
    DomainRuleViolation(String),

    #[error("Resource conflict: {0}")]
    ResourceConflict(String),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, LiumError>;
