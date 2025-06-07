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

// Re-export commonly used types
pub use errors::{LiumError, Result};
pub use models::{
    ApiExecutorResponse, ApiPodResponse, ApiTemplateResponse, ExecutorInfo, PodInfo, TemplateInfo,
};
pub use optimization::{
    calculate_pareto_frontier, dominates, extract_executor_metrics, extract_metrics,
    DefaultParetoOptimizer, ExecutorMetricsExtractor, MetricsExtractor, ParetoOptimizer,
};
