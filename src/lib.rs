// Re-export core domain types from lium-core
pub use lium_core::{
    calculate_pareto_frontier, dominates, ExecutorInfo, MetricsExtractor, ParetoOptimizer, PodInfo,
    TemplateInfo,
};

// Application-level errors that combine core and I/O errors
pub mod errors;
pub use errors::{LiumError, Result};

// Core modules
pub mod api;
pub mod cli;
pub mod config;
pub mod display;
pub mod sdk;
pub mod utils;

// Infrastructure modules
pub mod docker_utils;
pub mod ssh_utils;

// Feature modules organized by domain
pub mod formatters;
pub mod gpu_utils;
pub mod helpers;
pub mod id_generator;
pub mod parsers;
pub mod pod_utils;
pub mod resolvers;
pub mod storage;

// Commands module
pub mod commands;

// Re-export commonly used types and functions for convenience
pub use sdk::Lium;

// TODO: Gradually deprecate these re-exports as modules are refactored
pub use formatters::{calculate_cost_spent, format_uptime};
pub use gpu_utils::extract_gpu_model;
pub use id_generator::{generate_human_id, generate_uuid, is_valid_uuid};
pub use parsers::parse_ssh_command;
pub use pod_utils::{extract_ssh_details, filter_ready_pods, get_executor_id_from_pod};
pub use resolvers::{resolve_executor_indices, resolve_pod_targets, resolve_single_pod_target};
pub use storage::{
    get_last_executor_selection, get_last_pod_selection, store_executor_selection,
    store_pod_selection,
};
