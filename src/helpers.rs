use crate::errors::{LiumError, Result};
use crate::models::ExecutorInfo;
use regex::Regex;
use std::collections::HashMap;
use uuid::Uuid;

// Word lists for generating human-readable IDs
const ADJECTIVES: &[&str] = &[
    "brave", "calm", "clever", "cool", "eager", "fast", "gentle", "happy", "keen", "lively",
    "nice", "proud", "quick", "quiet", "smart", "swift", "warm", "wise", "young", "bold", "bright",
    "clean", "fresh", "grand", "great", "kind", "light", "lucky", "merry", "mild", "neat", "plain",
    "rich", "sharp", "shiny", "silly", "small", "super", "sweet", "thick",
];

const NOUNS: &[&str] = &[
    "ant", "bat", "bee", "cat", "cow", "dog", "elk", "fox", "gnu", "hen", "jay", "owl", "pig",
    "rat", "ram", "yak", "ape", "bug", "cub", "doe", "eel", "fly", "hog", "kid", "lab", "mom",
    "pup", "sun", "web", "zoo", "ace", "ash", "bay", "box", "day", "eye", "gem", "ink", "key",
    "oak",
];

/// Generate a human-readable ID from a UUID
/// Format: adjective-noun-hexsuffix (e.g., "brave-cat-a1b2")
pub fn generate_human_id(uuid: &str) -> String {
    // Create a simple hash from the UUID for consistent selection
    let hash = uuid.chars().enumerate().fold(0u32, |acc, (i, c)| {
        acc.wrapping_add((c as u32) * (i as u32 + 1))
    });

    let adj_idx = (hash % ADJECTIVES.len() as u32) as usize;
    let noun_idx = ((hash / ADJECTIVES.len() as u32) % NOUNS.len() as u32) as usize;

    // Get last 4 characters of UUID for hex suffix
    let hex_suffix = uuid
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();

    format!("{}-{}-{}", ADJECTIVES[adj_idx], NOUNS[noun_idx], hex_suffix)
}

/// Extract GPU model from machine name using regex patterns
pub fn extract_gpu_model(machine_name: &str) -> String {
    // Common GPU patterns to match
    let gpu_patterns = vec![
        r"(?i)(RTX\s*\d+(?:\s*Ti)?(?:\s*Super)?)",
        r"(?i)(GTX\s*\d+(?:\s*Ti)?(?:\s*Super)?)",
        r"(?i)(Tesla\s*[A-Z]\d+)",
        r"(?i)(A\d+(?:\s*SXM)?)",
        r"(?i)(V\d+(?:\s*SXM)?)",
        r"(?i)(H\d+(?:\s*SXM)?)",
        r"(?i)(Quadro\s*\w+)",
        r"(?i)(T4)",
        r"(?i)(P100)",
        r"(?i)(K80)",
    ];

    for pattern in gpu_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(captures) = re.captures(machine_name) {
                if let Some(matched) = captures.get(1) {
                    return matched.as_str().to_string();
                }
            }
        }
    }

    // If no pattern matches, try to extract any sequence that looks GPU-like
    if let Ok(re) = Regex::new(r"(?i)([A-Z]+\d+[A-Z]*\d*)") {
        if let Some(captures) = re.captures(machine_name) {
            if let Some(matched) = captures.get(1) {
                return matched.as_str().to_string();
            }
        }
    }

    // Fallback: return "Unknown"
    "Unknown".to_string()
}

/// Extract metrics from executor JSON for Pareto frontier calculation
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

/// Check if metrics_a dominates metrics_b (Pareto dominance)
/// Returns true if metrics_a is better or equal in all aspects and strictly better in at least one
pub fn dominates(metrics_a: &HashMap<String, f64>, metrics_b: &HashMap<String, f64>) -> bool {
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

/// Calculate Pareto frontier from a list of executors
/// Returns executors with a flag indicating if they're on the frontier
pub fn calculate_pareto_frontier(executors: Vec<ExecutorInfo>) -> Vec<(ExecutorInfo, bool)> {
    let mut result = Vec::new();

    for (i, executor) in executors.iter().enumerate() {
        let executor_json = serde_json::to_value(executor).unwrap_or_default();
        let metrics = extract_metrics(&executor_json).unwrap_or_default();

        let mut is_dominated = false;

        // Check if this executor is dominated by any other
        for (j, other) in executors.iter().enumerate() {
            if i == j {
                continue;
            }

            let other_json = serde_json::to_value(other).unwrap_or_default();
            let other_metrics = extract_metrics(&other_json).unwrap_or_default();

            if dominates(&other_metrics, &metrics) {
                is_dominated = true;
                break;
            }
        }

        result.push((executor.clone(), !is_dominated));
    }

    result
}

/// Resolve executor indices from string inputs (HUIDs, UUIDs, or numeric indices)
pub fn resolve_executor_indices(
    indices: &[String],
    last_selection_data: &serde_json::Value,
) -> Result<Vec<String>> {
    let mut resolved_ids = Vec::new();

    // Parse last selection data to get the list of executors
    let executors = last_selection_data
        .get("executors")
        .and_then(|v| v.as_array())
        .ok_or_else(|| LiumError::InvalidInput("No executor selection data found".to_string()))?;

    for index_str in indices {
        // Try parsing as numeric index first
        if let Ok(index) = index_str.parse::<usize>() {
            if index > 0 && index <= executors.len() {
                if let Some(executor) = executors.get(index - 1) {
                    if let Some(id) = executor.get("id").and_then(|v| v.as_str()) {
                        resolved_ids.push(id.to_string());
                        continue;
                    }
                }
            }
            return Err(LiumError::InvalidInput(format!(
                "Invalid executor index: {}",
                index
            )));
        }

        // Try matching as HUID or UUID
        let mut found = false;
        for executor in executors {
            if let (Some(id), Some(huid)) = (
                executor.get("id").and_then(|v| v.as_str()),
                executor.get("huid").and_then(|v| v.as_str()),
            ) {
                if id == index_str || huid == index_str {
                    resolved_ids.push(id.to_string());
                    found = true;
                    break;
                }
            }
        }

        if !found {
            return Err(LiumError::InvalidInput(format!(
                "Executor not found: {}. Use 'lium ls' to see available executors.",
                index_str
            )));
        }
    }

    Ok(resolved_ids)
}

/// Store executor selection data in config for later reference
pub fn store_executor_selection(gpu_type: &str, executors: &[ExecutorInfo]) -> Result<()> {
    let mut config = crate::config::load_config()?;

    let selection_data = serde_json::json!({
        "gpu_type": gpu_type,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "executors": executors
    });

    config.set_value("last_selection", "data", &selection_data.to_string())?;
    config.save()?;

    Ok(())
}

/// Get last executor selection from config
pub fn get_last_executor_selection() -> Result<Option<serde_json::Value>> {
    let config = crate::config::load_config()?;

    if let Some(data_str) = config.get_value("last_selection", "data")? {
        let selection_data: serde_json::Value = serde_json::from_str(&data_str)?;
        Ok(Some(selection_data))
    } else {
        Ok(None)
    }
}

/// Generate a new UUID v4
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Validate UUID format
pub fn is_valid_uuid(uuid_str: &str) -> bool {
    Uuid::parse_str(uuid_str).is_ok()
}

/// Resolve pod targets from string inputs (HUIDs, UUIDs, indices, or "all")
/// Returns vector of (PodInfo, resolved_identifier) tuples
pub async fn resolve_pod_targets(
    api_client: &crate::api::LiumApiClient,
    target_inputs: &[String],
) -> Result<Vec<(crate::models::PodInfo, String)>> {
    let all_pods = api_client.get_pods().await?;

    if all_pods.is_empty() {
        return Err(LiumError::OperationFailed(
            "No active pods found".to_string(),
        ));
    }

    let mut resolved_pods = Vec::new();

    for target in target_inputs {
        if target.to_lowercase() == "all" {
            // Return all pods
            for pod in &all_pods {
                resolved_pods.push((pod.clone(), pod.huid.clone()));
            }
        } else if let Ok(index) = target.parse::<usize>() {
            // Numeric index (1-based)
            if index > 0 && index <= all_pods.len() {
                let pod = &all_pods[index - 1];
                resolved_pods.push((pod.clone(), format!("#{}", index)));
            } else {
                return Err(LiumError::InvalidInput(format!(
                    "Invalid pod index: {}. Valid range: 1-{}",
                    index,
                    all_pods.len()
                )));
            }
        } else {
            // Try to match by HUID or UUID
            let mut found = false;
            for pod in &all_pods {
                if pod.huid == *target || pod.id == *target || pod.name == *target {
                    resolved_pods.push((pod.clone(), target.clone()));
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(LiumError::InvalidInput(format!(
                    "Pod not found: {}. Use 'lium ps' to see available pods.",
                    target
                )));
            }
        }
    }

    if resolved_pods.is_empty() {
        return Err(LiumError::InvalidInput(
            "No pods resolved from targets".to_string(),
        ));
    }

    Ok(resolved_pods)
}

/// Parse pod target from a single string (used by commands that take one pod)
pub async fn resolve_single_pod_target(
    api_client: &crate::api::LiumApiClient,
    target: &str,
) -> Result<crate::models::PodInfo> {
    let resolved = resolve_pod_targets(api_client, &[target.to_string()]).await?;

    if resolved.len() != 1 {
        return Err(LiumError::InvalidInput(format!(
            "Expected single pod, but found {}",
            resolved.len()
        )));
    }

    Ok(resolved[0].0.clone())
}

/// Extract SSH connection details from pod info
pub fn extract_ssh_details(pod: &crate::models::PodInfo) -> Result<(String, u16, String)> {
    let ssh_cmd = pod.ssh_cmd.as_ref().ok_or_else(|| {
        LiumError::InvalidInput("Pod does not have SSH connection info".to_string())
    })?;

    // Parse SSH command like: "ssh -p 12345 root@hostname"
    let re = regex::Regex::new(r"ssh\s+(?:-p\s+(\d+)\s+)?(\w+)@([\w\.-]+)")
        .map_err(|e| LiumError::InvalidInput(format!("Regex error: {}", e)))?;

    if let Some(captures) = re.captures(ssh_cmd) {
        let port = captures
            .get(1)
            .map(|m| m.as_str().parse::<u16>().unwrap_or(22))
            .unwrap_or(22);
        let user = captures
            .get(2)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "root".to_string());
        let host = captures
            .get(3)
            .ok_or_else(|| {
                LiumError::InvalidInput("Could not parse hostname from SSH command".to_string())
            })?
            .as_str()
            .to_string();

        Ok((host, port, user))
    } else {
        Err(LiumError::InvalidInput(format!(
            "Could not parse SSH command: {}",
            ssh_cmd
        )))
    }
}

// TODO: Add function to resolve pod targets (similar to executor resolution)
// TODO: Add metrics comparison utilities
// TODO: Add pricing calculation helpers
// TODO: Add location/region utilities
