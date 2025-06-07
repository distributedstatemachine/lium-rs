// Re-export core domain types from lium-core
pub use lium_core::{
    calculate_pareto_frontier, dominates, ExecutorInfo, MetricsExtractor, ParetoOptimizer, PodInfo,
    TemplateInfo,
};

// Application-level errors that combine core and I/O errors
pub mod errors;
pub use errors::{LiumError, Result};

// Local modules
pub mod cli;
pub mod config;
pub mod display;
pub mod helpers;
pub mod resolvers;
pub mod storage;
pub mod utils;

// Re-export from external crates
pub use lium_api as api;
pub use lium_api::Lium as sdk;
pub use lium_utils::{formatters, gpu, id_generator, parsers, pod};

// Commands module
pub mod commands;

// Re-export commonly used types and functions for convenience
pub use lium_api::Lium;

// TODO: Gradually deprecate these re-exports as modules are refactored
pub use formatters::{calculate_cost_spent, format_uptime};
pub use gpu::extract_gpu_model;
pub use id_generator::{generate_human_id, generate_uuid, is_valid_uuid};
pub use parsers::parse_ssh_command;
pub use pod::{extract_ssh_details, filter_ready_pods, get_executor_id_from_pod};
pub use resolvers::{resolve_executor_indices, resolve_pod_targets, resolve_single_pod_target};
pub use storage::{
    get_last_executor_selection, get_last_pod_selection, store_executor_selection,
    store_pod_selection,
};
