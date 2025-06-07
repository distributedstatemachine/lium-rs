//! # Lium Core
//!
//! Core domain logic for Lium GPU compute management.
//!
//! This crate contains pure business logic with no I/O dependencies:
//! - Domain models and types
//! - Error definitions  
//! - Optimization algorithms
//! - Business rule validation
//!
//! ## Design Principles
//!
//! - **Pure Functions**: No side effects, easy to test
//! - **Domain-Driven**: Models real-world GPU compute concepts
//! - **Dependency-Free**: No I/O, networking, or persistence dependencies
//! - **Composable**: Designed for reuse in different contexts

pub mod errors;
pub mod models;
pub mod optimization;
pub mod utils;

// Re-export commonly used types
pub use errors::{LiumError, Result};
pub use models::{
    ApiExecutorResponse, ApiPodResponse, ApiTemplateResponse, ExecutorInfo, PodInfo, TemplateInfo,
};
pub use optimization::{
    calculate_pareto_frontier, dominates, extract_executor_metrics, extract_metrics,
    DefaultParetoOptimizer, ExecutorMetricsExtractor, MetricsExtractor, ParetoOptimizer,
};
pub use utils::{
    filter_by_availability, filter_by_gpu_type, filter_by_price_range, find_pareto_optimal,
    group_by_gpu_type, parse_env_vars, parse_executor_index, parse_gpu_filter, parse_port_mappings,
    parse_price_range, sort_by_gpu_count, sort_by_price, validate_docker_image,
};
