use crate::{errors::LiumError, models::ExecutorInfo, Result};
use std::collections::HashMap;

/// Parse executor index from user input (1-based to 0-based)
pub fn parse_executor_index(input: &str, max_index: usize) -> Result<usize> {
    let index = input
        .trim()
        .parse::<usize>()
        .map_err(|_| LiumError::InvalidInput(format!("Invalid index: {}", input)))?;

    if index == 0 || index > max_index {
        return Err(LiumError::InvalidInput(format!(
            "Index must be between 1 and {}",
            max_index
        )));
    }

    Ok(index - 1) // Convert to 0-based
}

/// Parse GPU type filter from command line arguments
pub fn parse_gpu_filter(gpu_type: &str) -> String {
    gpu_type.to_uppercase()
}

/// Parse price range filter
pub fn parse_price_range(price_str: &str) -> Result<(f64, f64)> {
    let parts: Vec<&str> = price_str.split('-').collect();
    if parts.len() != 2 {
        return Err(LiumError::InvalidInput(
            "Price range must be in format 'min-max' (e.g., '0.5-2.0')".to_string(),
        ));
    }

    let min = parts[0]
        .parse::<f64>()
        .map_err(|_| LiumError::InvalidInput(format!("Invalid minimum price: {}", parts[0])))?;
    let max = parts[1]
        .parse::<f64>()
        .map_err(|_| LiumError::InvalidInput(format!("Invalid maximum price: {}", parts[1])))?;

    if min >= max {
        return Err(LiumError::InvalidInput(
            "Minimum price must be less than maximum price".to_string(),
        ));
    }

    Ok((min, max))
}

/// Filter executors by GPU type
pub fn filter_by_gpu_type(executors: &[ExecutorInfo], gpu_type: &str) -> Vec<ExecutorInfo> {
    let filter = parse_gpu_filter(gpu_type);
    executors
        .iter()
        .filter(|e| e.gpu_type.to_uppercase().contains(&filter))
        .cloned()
        .collect()
}

/// Filter executors by price range (per GPU per hour)
pub fn filter_by_price_range(
    executors: &[ExecutorInfo],
    min_price: f64,
    max_price: f64,
) -> Vec<ExecutorInfo> {
    executors
        .iter()
        .filter(|e| e.price_per_gpu_hour >= min_price && e.price_per_gpu_hour <= max_price)
        .cloned()
        .collect()
}

/// Filter executors by availability
pub fn filter_by_availability(
    executors: &[ExecutorInfo],
    available_only: bool,
) -> Vec<ExecutorInfo> {
    if available_only {
        executors.iter().filter(|e| e.available).cloned().collect()
    } else {
        executors.to_vec()
    }
}

/// Sort executors by price (ascending)
pub fn sort_by_price(executors: &mut [ExecutorInfo]) {
    executors.sort_by(|a, b| {
        a.price_per_gpu_hour
            .partial_cmp(&b.price_per_gpu_hour)
            .unwrap()
    });
}

/// Sort executors by GPU count (descending)
pub fn sort_by_gpu_count(executors: &mut [ExecutorInfo]) {
    executors.sort_by(|a, b| b.gpu_count.cmp(&a.gpu_count));
}

/// Group executors by GPU type
pub fn group_by_gpu_type(executors: &[ExecutorInfo]) -> HashMap<String, Vec<ExecutorInfo>> {
    let mut groups: HashMap<String, Vec<ExecutorInfo>> = HashMap::new();

    for executor in executors {
        groups
            .entry(executor.gpu_type.clone())
            .or_default()
            .push(executor.clone());
    }

    groups
}

/// Find Pareto optimal executors (best price/performance ratio)
pub fn find_pareto_optimal(executors: &[ExecutorInfo]) -> Vec<ExecutorInfo> {
    let mut pareto_optimal = Vec::new();

    for executor in executors {
        let mut is_dominated = false;

        // Check if this executor is dominated by any other
        for other in executors {
            if other.huid != executor.huid
                && other.price_per_gpu_hour <= executor.price_per_gpu_hour
                && other.gpu_count >= executor.gpu_count
                && (other.price_per_gpu_hour < executor.price_per_gpu_hour
                    || other.gpu_count > executor.gpu_count)
            {
                is_dominated = true;
                break;
            }
        }

        if !is_dominated {
            pareto_optimal.push(executor.clone());
        }
    }

    pareto_optimal
}

/// Validate Docker image name
pub fn validate_docker_image(image: &str) -> Result<()> {
    if image.is_empty() {
        return Err(LiumError::InvalidInput(
            "Docker image cannot be empty".to_string(),
        ));
    }

    // Basic validation - Docker image names can contain lowercase letters, digits, and separators
    let valid_chars = image.chars().all(|c| {
        c.is_ascii_lowercase()
            || c.is_ascii_digit()
            || c == '/'
            || c == ':'
            || c == '.'
            || c == '-'
            || c == '_'
    });

    if !valid_chars {
        return Err(LiumError::InvalidInput(
            "Docker image name contains invalid characters".to_string(),
        ));
    }

    Ok(())
}

/// Parse environment variables from string format "KEY=VALUE,KEY2=VALUE2"
pub fn parse_env_vars(env_str: &str) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();

    if env_str.trim().is_empty() {
        return Ok(env_vars);
    }

    for pair in env_str.split(',') {
        let parts: Vec<&str> = pair.trim().splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(LiumError::InvalidInput(format!(
                "Invalid environment variable format: '{}'. Use KEY=VALUE",
                pair
            )));
        }

        let key = parts[0].trim();
        let value = parts[1].trim();

        if key.is_empty() {
            return Err(LiumError::InvalidInput(
                "Environment variable key cannot be empty".to_string(),
            ));
        }

        env_vars.insert(key.to_string(), value.to_string());
    }

    Ok(env_vars)
}

/// Parse port mappings from string format "8080:80,9000:9000"
pub fn parse_port_mappings(ports_str: &str) -> Result<HashMap<String, String>> {
    let mut port_mappings = HashMap::new();

    if ports_str.trim().is_empty() {
        return Ok(port_mappings);
    }

    for mapping in ports_str.split(',') {
        let parts: Vec<&str> = mapping.trim().splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(LiumError::InvalidInput(format!(
                "Invalid port mapping format: '{}'. Use HOST_PORT:CONTAINER_PORT",
                mapping
            )));
        }

        let host_port = parts[0].trim();
        let container_port = parts[1].trim();

        // Validate port numbers
        let _: u16 = host_port
            .parse()
            .map_err(|_| LiumError::InvalidInput(format!("Invalid host port: {}", host_port)))?;
        let _: u16 = container_port.parse().map_err(|_| {
            LiumError::InvalidInput(format!("Invalid container port: {}", container_port))
        })?;

        port_mappings.insert(host_port.to_string(), container_port.to_string());
    }

    Ok(port_mappings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_executor_index() {
        assert_eq!(parse_executor_index("1", 5).unwrap(), 0);
        assert_eq!(parse_executor_index("5", 5).unwrap(), 4);
        assert!(parse_executor_index("0", 5).is_err());
        assert!(parse_executor_index("6", 5).is_err());
        assert!(parse_executor_index("abc", 5).is_err());
    }

    #[test]
    fn test_parse_price_range() {
        let (min, max) = parse_price_range("0.5-2.0").unwrap();
        assert_eq!(min, 0.5);
        assert_eq!(max, 2.0);

        assert!(parse_price_range("2.0-0.5").is_err()); // min >= max
        assert!(parse_price_range("invalid").is_err());
    }

    #[test]
    fn test_parse_env_vars() {
        let env_vars = parse_env_vars("KEY1=value1,KEY2=value2").unwrap();
        assert_eq!(env_vars.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env_vars.get("KEY2"), Some(&"value2".to_string()));

        assert!(parse_env_vars("INVALID").is_err());
        assert!(parse_env_vars("=value").is_err());
    }

    #[test]
    fn test_filter_by_gpu_type() {
        let executors = vec![
            create_test_executor("1", "RTX4090", 1.0, true),
            create_test_executor("2", "H100", 2.0, true),
            create_test_executor("3", "RTX4090", 1.5, false),
        ];

        let filtered = filter_by_gpu_type(&executors, "RTX4090");
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|e| e.gpu_type.contains("RTX4090")));
    }

    #[test]
    fn test_filter_by_availability() {
        let executors = vec![
            create_test_executor("1", "RTX4090", 1.0, true),
            create_test_executor("2", "H100", 2.0, false),
            create_test_executor("3", "RTX4090", 1.5, true),
        ];

        let filtered = filter_by_availability(&executors, true);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|e| e.available));
    }

    fn create_test_executor(
        huid: &str,
        gpu_type: &str,
        price: f64,
        available: bool,
    ) -> ExecutorInfo {
        ExecutorInfo {
            id: format!("exec_{}", huid),
            huid: huid.to_string(),
            machine_name: format!("machine-{}-{}", gpu_type.to_lowercase(), huid),
            gpu_type: gpu_type.to_string(),
            gpu_count: 1,
            price_per_gpu_hour: price,
            price_per_hour: price,
            available,
            status: if available {
                "available".to_string()
            } else {
                "rented".to_string()
            },
            location: HashMap::new(),
            specs: json!({
                "cpu_cores": 8,
                "memory_gb": 32,
                "storage_gb": 500
            }),
        }
    }
}

// TODO: Add more sophisticated filtering options (location, specs)
// TODO: Add caching for expensive operations
// TODO: Add input validation for more data types
// TODO: Add support for regex patterns in filters
// TODO: Add configuration validation helpers
