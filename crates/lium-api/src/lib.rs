//! # Lium API
//!
//! A high-performance HTTP API client for the Lium project, providing a robust interface for interacting with
//! remote Lium services and executors. This crate handles authentication, request/response serialization,
//! error handling, and provides type-safe access to Lium's core functionality.
//!
//! ## Features
//!
//! - Asynchronous HTTP client with automatic retries and error handling
//! - Type-safe API responses with automatic serialization/deserialization
//! - Comprehensive error handling with detailed error types
//! - Built-in logging and debugging capabilities
//! - Support for all major Lium API endpoints
//!
//! ## Core Components
//!
//! - `client`: Main API client implementation with methods for all endpoints
//! - `errors`: Comprehensive error handling and custom error types
//! - `sdk`: Additional SDK functionality and utilities
//!
//! ## Usage
//!
//! ```rust
//! use lium_api::{Client, ApiError};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ApiError> {
//!     let client = Client::new("https://api.lium.com", "your-api-key")?;
//!     
//!     // Get available executors
//!     let executors = client.get_executors().await?;
//!     
//!     // Create a new pod
//!     let pod = client.rent_pod(
//!         "exec-123",
//!         "my-pod",
//!         "template-456",
//!         vec!["ssh-rsa AAAAB3NzaC1yc2EAAAADA...".to_string()]
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Error Handling
//!
//! The crate provides detailed error types through the `ApiError` enum, which includes:
//! - HTTP errors
//! - Authentication errors
//! - Rate limiting
//! - Service availability issues
//! - Configuration errors
//!
//! ## Security
//!
//! - API keys are handled securely and never logged in full
//! - All requests are made over HTTPS
//! - Sensitive data is properly sanitized in logs
//!
//! ## Dependencies
//!
//! - `reqwest`: HTTP client
//! - `serde`: Serialization/deserialization
//! - `tokio`: Async runtime
//! - `thiserror`: Error handling
//! - `log`: Logging

pub mod client;
pub mod errors;
pub mod sdk;

// Re-export common types for convenience
pub use client::*;
pub use errors::*;
pub use sdk::*;

// Re-export core types that API consumers will need
pub use lium_core::{ExecutorInfo, PodInfo, Result as CoreResult, TemplateInfo};
