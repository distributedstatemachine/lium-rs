pub mod api;
pub mod cli;
pub mod commands;
pub mod config;
pub mod display;
pub mod docker_utils;
pub mod errors;
pub mod helpers;
pub mod models;
pub mod sdk;
pub mod ssh_utils;
pub mod utils;

// Re-export main public types
pub use errors::{LiumError, Result};
pub use models::{ExecutorInfo, PodInfo, TemplateInfo};
pub use sdk::Lium;

// Re-export for CLI usage
pub use api::LiumApiClient;
pub use config::Config;
