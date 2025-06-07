use crate::{CliError, Result};
use lium_core::PodInfo;
use std::future::Future;

/// Trait for resolving targets to concrete items
pub trait TargetResolver<T> {
    type Input;
    type Output;

    fn resolve_targets(
        &self,
        inputs: &[Self::Input],
    ) -> impl Future<Output = Result<Vec<Self::Output>>> + Send;
}

/// Pod target resolver
pub struct PodTargetResolver<'a> {
    api_client: &'a lium_api::LiumApiClient,
}

impl<'a> PodTargetResolver<'a> {
    pub fn new(api_client: &'a lium_api::LiumApiClient) -> Self {
        Self { api_client }
    }
}

impl TargetResolver<PodInfo> for PodTargetResolver<'_> {
    type Input = String;
    type Output = (PodInfo, String);

    /// Resolve pod targets (indices, HUIDs, names, "all") to (PodInfo, identifier) pairs
    fn resolve_targets(
        &self,
        target_inputs: &[Self::Input],
    ) -> impl Future<Output = Result<Vec<Self::Output>>> + Send {
        async move {
            let all_pods = self.api_client.get_pods().await?;
            let mut resolved_pods = Vec::new();

            for target in target_inputs {
                if target == "all" {
                    // Add all pods
                    for pod in &all_pods {
                        resolved_pods.push((pod.clone(), "all".to_string()));
                    }
                } else if let Ok(index) = target.parse::<usize>() {
                    // Numeric index (1-based)
                    if index == 0 || index > all_pods.len() {
                        return Err(CliError::InvalidInput(format!(
                            "Invalid pod index: {}. Valid range: 1-{}",
                            index,
                            all_pods.len()
                        )));
                    }

                    let pod = all_pods[index - 1].clone();
                    resolved_pods.push((pod, target.clone()));
                } else {
                    // HUID, name, or UUID
                    let mut found = false;
                    for pod in &all_pods {
                        if pod.huid == *target || pod.name == *target || pod.id == *target {
                            resolved_pods.push((pod.clone(), target.clone()));
                            found = true;
                            break;
                        }
                    }

                    if !found {
                        return Err(CliError::InvalidInput(format!("Pod not found: {}", target)));
                    }
                }
            }

            if resolved_pods.is_empty() && !target_inputs.is_empty() {
                return Err(CliError::InvalidInput(
                    "No pods matched the specified targets".to_string(),
                ));
            }

            Ok(resolved_pods)
        }
    }
}

/// Executor target resolver
pub struct ExecutorTargetResolver;

impl ExecutorTargetResolver {
    /// Resolve executor indices/HUIDs to executor IDs
    pub fn resolve_executor_indices(
        &self,
        indices: &[String],
        last_selection_data: &serde_json::Value,
    ) -> Result<Vec<String>> {
        let mut resolved_ids = Vec::new();

        // Parse the last selection data
        let executors = last_selection_data
            .get("executors")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                CliError::InvalidInput(
                    "No executor selection data found. Run 'lium ls' first.".to_string(),
                )
            })?;

        for index_str in indices {
            if let Ok(index) = index_str.parse::<usize>() {
                // Numeric index (1-based)
                if index == 0 || index > executors.len() {
                    return Err(CliError::InvalidInput(format!(
                        "Invalid executor index: {}. Valid range: 1-{}",
                        index,
                        executors.len()
                    )));
                }

                let executor = &executors[index - 1];
                let id = executor.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                    CliError::InvalidInput("Invalid executor data in selection".to_string())
                })?;

                resolved_ids.push(id.to_string());
            } else {
                // HUID or UUID - find matching executor
                let mut found = false;
                for executor in executors {
                    let executor_huid = executor.get("huid").and_then(|v| v.as_str());
                    let executor_id = executor.get("id").and_then(|v| v.as_str());

                    if executor_huid == Some(index_str) || executor_id == Some(index_str) {
                        if let Some(id) = executor_id {
                            resolved_ids.push(id.to_string());
                            found = true;
                            break;
                        }
                    }
                }

                if !found {
                    return Err(CliError::InvalidInput(format!(
                        "Executor not found: {}",
                        index_str
                    )));
                }
            }
        }

        Ok(resolved_ids)
    }
}

// Convenience functions for backward compatibility and easier usage
pub async fn resolve_pod_targets(
    api_client: &lium_api::LiumApiClient,
    target_inputs: &[String],
) -> Result<Vec<(PodInfo, String)>> {
    let resolver = PodTargetResolver::new(api_client);
    resolver.resolve_targets(target_inputs).await
}

pub fn resolve_executor_indices(
    indices: &[String],
    last_selection_data: &serde_json::Value,
) -> Result<Vec<String>> {
    let resolver = ExecutorTargetResolver;
    resolver.resolve_executor_indices(indices, last_selection_data)
}

/// Validate that all provided pod targets exist before performing operations
pub async fn validate_pod_targets(
    api_client: &lium_api::LiumApiClient,
    target_inputs: &[String],
) -> Result<Vec<PodInfo>> {
    let resolved = resolve_pod_targets(api_client, target_inputs).await?;
    Ok(resolved.into_iter().map(|(pod, _)| pod).collect())
}

/// Resolve a single pod target (wrapper around resolve_pod_targets for single pod)
pub async fn resolve_single_pod_target(
    api_client: &lium_api::LiumApiClient,
    target: &str,
) -> Result<PodInfo> {
    let pods = resolve_pod_targets(api_client, &[target.to_string()]).await?;

    if pods.is_empty() {
        return Err(CliError::NotFound(format!(
            "No pod found matching: {}",
            target
        )));
    }

    if pods.len() > 1 {
        return Err(CliError::InvalidInput(format!(
            "Multiple pods found matching '{}'. Please be more specific.",
            target
        )));
    }

    Ok(pods.into_iter().next().unwrap().0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_executor_resolver() {
        let resolver = ExecutorTargetResolver;

        let selection_data = json!({
            "executors": [
                {"id": "exec1", "huid": "brave-cat-1234"},
                {"id": "exec2", "huid": "smart-dog-5678"}
            ]
        });

        // Test numeric index
        let result = resolver
            .resolve_executor_indices(&["1".to_string()], &selection_data)
            .unwrap();
        assert_eq!(result, vec!["exec1"]);

        // Test HUID
        let result = resolver
            .resolve_executor_indices(&["brave-cat-1234".to_string()], &selection_data)
            .unwrap();
        assert_eq!(result, vec!["exec1"]);

        // Test invalid index
        let result = resolver.resolve_executor_indices(&["999".to_string()], &selection_data);
        assert!(result.is_err());
    }
}
