//! # Lium API
//!
//! HTTP API client for the Lium project.
//! This crate provides a high-level interface for interacting with
//! remote Lium services and executors.

pub mod client;
pub mod errors;
pub mod sdk;

// Re-export common types for convenience
pub use client::*;
pub use errors::*;
pub use sdk::*;

// Re-export core types that API consumers will need
pub use lium_core::{ExecutorInfo, PodInfo, Result as CoreResult, TemplateInfo};
