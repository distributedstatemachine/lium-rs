//! Legacy helpers module - gradually being refactored into specialized modules
//!
//! TODO: This module should eventually be removed once all CLI code is updated
//! to use the new specialized modules directly.

// Re-export functions from specialized modules for backward compatibility
pub use crate::formatters::{calculate_cost_spent, format_uptime};
pub use crate::gpu_utils::extract_gpu_model;
pub use crate::id_generator::{generate_human_id, generate_uuid, is_valid_uuid};
pub use crate::parsers::parse_ssh_command;
pub use crate::pod_utils::{extract_ssh_details, filter_ready_pods, get_executor_id_from_pod};
pub use crate::resolvers::{
    resolve_executor_indices, resolve_pod_targets, resolve_single_pod_target, validate_pod_targets,
};
pub use crate::storage::{
    get_last_executor_selection, get_last_pod_selection, store_executor_selection,
    store_pod_selection,
};
pub use lium_core::optimization::{
    calculate_pareto_frontier, dominates, extract_executor_metrics, extract_metrics,
};

// TODO: Remove this file once all imports have been updated to use the new modules directly
