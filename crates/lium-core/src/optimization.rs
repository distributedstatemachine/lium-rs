use crate::errors::Result;
use crate::models::ExecutorInfo;
use std::collections::HashMap;

/// Trait for metrics extraction
pub trait MetricsExtractor<T> {
    fn extract_metrics(&self, item: &T) -> Result<HashMap<String, f64>>;
}

/// Pareto frontier calculator trait
pub trait ParetoOptimizer<T> {
    fn calculate_pareto_frontier(&self, items: Vec<T>) -> Vec<(T, bool)>;
    fn dominates(&self, metrics_a: &HashMap<String, f64>, metrics_b: &HashMap<String, f64>)
        -> bool;
}

/// Default executor metrics extractor
pub struct ExecutorMetricsExtractor;

impl MetricsExtractor<ExecutorInfo> for ExecutorMetricsExtractor {
    fn extract_metrics(&self, executor: &ExecutorInfo) -> Result<HashMap<String, f64>> {
        let mut metrics = HashMap::new();

        // Price per GPU hour (lower is better, so negate for dominance comparison)
        metrics.insert(
            "price_per_gpu_hour".to_string(),
            -executor.price_per_gpu_hour,
        );

        // GPU count (higher is better)
        metrics.insert("gpu_count".to_string(), executor.gpu_count as f64);

        // Extract specs from JSON
        if let Some(specs) = executor.specs.as_object() {
            // RAM (higher is better)
            if let Some(ram) = specs.get("ram_gb").and_then(|v| v.as_f64()) {
                metrics.insert("ram_gb".to_string(), ram);
            }

            // CPU cores (higher is better)
            if let Some(cpu) = specs.get("cpu_cores").and_then(|v| v.as_f64()) {
                metrics.insert("cpu_cores".to_string(), cpu);
            }

            // Storage (higher is better)
            if let Some(storage) = specs.get("storage_gb").and_then(|v| v.as_f64()) {
                metrics.insert("storage_gb".to_string(), storage);
            }
        }

        Ok(metrics)
    }
}

/// Default Pareto optimizer implementation
pub struct DefaultParetoOptimizer<T, E> {
    extractor: E,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, E> DefaultParetoOptimizer<T, E>
where
    E: MetricsExtractor<T>,
{
    pub fn new(extractor: E) -> Self {
        Self {
            extractor,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, E> ParetoOptimizer<T> for DefaultParetoOptimizer<T, E>
where
    T: Clone,
    E: MetricsExtractor<T>,
{
    /// Check if metrics_a dominates metrics_b (Pareto dominance)
    /// Returns true if metrics_a is better or equal in all aspects and strictly better in at least one
    fn dominates(
        &self,
        metrics_a: &HashMap<String, f64>,
        metrics_b: &HashMap<String, f64>,
    ) -> bool {
        let mut strictly_better_in_one = false;

        // Get all metric keys from both maps
        let mut all_keys: std::collections::HashSet<String> = metrics_a.keys().cloned().collect();
        all_keys.extend(metrics_b.keys().cloned());

        for key in all_keys {
            let a_val = metrics_a.get(&key).unwrap_or(&0.0);
            let b_val = metrics_b.get(&key).unwrap_or(&0.0);

            if a_val < b_val {
                // A is worse in this metric, so A doesn't dominate B
                return false;
            } else if a_val > b_val {
                // A is better in this metric
                strictly_better_in_one = true;
            }
        }

        strictly_better_in_one
    }

    /// Calculate Pareto frontier for item selection
    /// Returns vector of (item, is_pareto_optimal) pairs
    fn calculate_pareto_frontier(&self, items: Vec<T>) -> Vec<(T, bool)> {
        let mut result = Vec::new();

        for (i, item_a) in items.iter().enumerate() {
            let mut is_pareto_optimal = true;

            // Extract metrics for this item
            let metrics_a = match self.extractor.extract_metrics(item_a) {
                Ok(m) => m,
                Err(_) => {
                    result.push((item_a.clone(), false));
                    continue;
                }
            };

            // Check if any other item dominates this one
            for (j, item_b) in items.iter().enumerate() {
                if i == j {
                    continue;
                }

                if let Ok(metrics_b) = self.extractor.extract_metrics(item_b) {
                    if self.dominates(&metrics_b, &metrics_a) {
                        is_pareto_optimal = false;
                        break;
                    }
                }
            }

            result.push((item_a.clone(), is_pareto_optimal));
        }

        result
    }
}

// Convenience functions for backward compatibility
pub fn extract_executor_metrics(executor: &ExecutorInfo) -> Result<HashMap<String, f64>> {
    ExecutorMetricsExtractor.extract_metrics(executor)
}

pub fn dominates(metrics_a: &HashMap<String, f64>, metrics_b: &HashMap<String, f64>) -> bool {
    let optimizer = DefaultParetoOptimizer::new(ExecutorMetricsExtractor);
    optimizer.dominates(metrics_a, metrics_b)
}

pub fn calculate_pareto_frontier(executors: Vec<ExecutorInfo>) -> Vec<(ExecutorInfo, bool)> {
    let optimizer = DefaultParetoOptimizer::new(ExecutorMetricsExtractor);
    optimizer.calculate_pareto_frontier(executors)
}

/// Extract metrics from executor JSON for Pareto frontier calculation
/// TODO: This should be moved to a separate JSON utils module
pub fn extract_metrics(executor_json: &serde_json::Value) -> Result<HashMap<String, f64>> {
    let mut metrics = HashMap::new();

    // Extract common metrics from specs
    if let Some(specs) = executor_json.get("specs") {
        // CPU cores
        if let Some(cpu) = specs.get("cpu_cores").and_then(|v| v.as_f64()) {
            metrics.insert("cpu_cores".to_string(), cpu);
        }

        // Memory (GB)
        if let Some(memory) = specs.get("memory_gb").and_then(|v| v.as_f64()) {
            metrics.insert("memory_gb".to_string(), memory);
        }

        // Storage (GB)
        if let Some(storage) = specs.get("storage_gb").and_then(|v| v.as_f64()) {
            metrics.insert("storage_gb".to_string(), storage);
        }

        // GPU memory (GB)
        if let Some(gpu_memory) = specs.get("gpu_memory_gb").and_then(|v| v.as_f64()) {
            metrics.insert("gpu_memory_gb".to_string(), gpu_memory);
        }
    }

    // Add price metrics (negative because lower is better)
    if let Some(price) = executor_json.get("price_per_hour").and_then(|v| v.as_f64()) {
        metrics.insert("price_per_hour".to_string(), -price); // Negative for minimization
    }

    if let Some(price_gpu) = executor_json
        .get("price_per_gpu_hour")
        .and_then(|v| v.as_f64())
    {
        metrics.insert("price_per_gpu_hour".to_string(), -price_gpu); // Negative for minimization
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_pareto_dominance() {
        let optimizer = DefaultParetoOptimizer::new(ExecutorMetricsExtractor);

        let metrics_a = [
            ("price".to_string(), -1.0),
            ("performance".to_string(), 10.0),
        ]
        .iter()
        .cloned()
        .collect();
        let metrics_b = [
            ("price".to_string(), -2.0),
            ("performance".to_string(), 5.0),
        ]
        .iter()
        .cloned()
        .collect();

        // A dominates B (better price and performance)
        assert!(optimizer.dominates(&metrics_a, &metrics_b));
        assert!(!optimizer.dominates(&metrics_b, &metrics_a));
    }

    #[test]
    fn test_executor_metrics_extraction() {
        let executor = ExecutorInfo {
            id: "test".to_string(),
            huid: "test-huid".to_string(),
            machine_name: "test-machine".to_string(),
            gpu_type: "RTX4090".to_string(),
            gpu_count: 2,
            price_per_gpu_hour: 1.5,
            price_per_hour: 3.0,
            available: true,
            specs: json!({
                "ram_gb": 64.0,
                "cpu_cores": 16.0,
                "storage_gb": 1000.0
            }),
            location: Default::default(),
            status: "active".to_string(),
        };

        let extractor = ExecutorMetricsExtractor;
        let metrics = extractor.extract_metrics(&executor).unwrap();

        assert_eq!(metrics.get("gpu_count"), Some(&2.0));
        assert_eq!(metrics.get("price_per_gpu_hour"), Some(&-1.5));
        assert_eq!(metrics.get("ram_gb"), Some(&64.0));
    }
}
