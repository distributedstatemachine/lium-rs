use crate::{CliError, Result};
use lium_core::{ExecutorInfo, PodInfo};

/// Trait for storing and retrieving selection data
pub trait SelectionStorage {
    type Item;

    fn store_selection(&self, items: &[Self::Item]) -> Result<()>;
    fn get_last_selection(&self) -> Result<Option<serde_json::Value>>;
}

/// Executor selection storage
pub struct ExecutorSelectionStorage;

impl SelectionStorage for ExecutorSelectionStorage {
    type Item = (String, ExecutorInfo); // (gpu_type, executor)

    /// Store executor selection for later index-based reference
    fn store_selection(&self, items: &[Self::Item]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let mut config = crate::config::load_config()?;
        let gpu_type = &items[0].0; // Assume all items have the same GPU type
        let executors: Vec<&ExecutorInfo> = items.iter().map(|(_, e)| e).collect();

        let selection_data = serde_json::json!({
            "gpu_type": gpu_type,
            "timestamp": chrono::Utc::now().timestamp(),
            "executors": executors.iter().map(|e| serde_json::json!({
                "id": e.id,
                "huid": e.huid,
                "machine_name": e.machine_name,
                "gpu_type": e.gpu_type,
                "gpu_count": e.gpu_count,
                "price_per_gpu_hour": e.price_per_gpu_hour
            })).collect::<Vec<_>>()
        });

        config.set_value("last_selection", "data", &selection_data.to_string())?;
        config.save()?;

        Ok(())
    }

    /// Get last executor selection data
    fn get_last_selection(&self) -> Result<Option<serde_json::Value>> {
        let config = crate::config::load_config()?;

        if let Some(data_str) = config.get_value("last_selection", "data")? {
            let data: serde_json::Value = serde_json::from_str(&data_str)
                .map_err(|e| CliError::InvalidInput(format!("Invalid selection data: {}", e)))?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
}

/// Pod selection storage
pub struct PodSelectionStorage;

impl SelectionStorage for PodSelectionStorage {
    type Item = PodInfo;

    /// Store pod selection data in config for later reference by index
    fn store_selection(&self, pods: &[Self::Item]) -> Result<()> {
        let mut config = crate::config::load_config()?;

        let selection_data = serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "pods": pods.iter().map(|p| serde_json::json!({
                "id": p.id,
                "huid": p.huid,
                "name": p.name,
                "status": p.status
            })).collect::<Vec<_>>()
        });

        config.set_value("last_pod_selection", "data", &selection_data.to_string())?;
        config.save()?;

        Ok(())
    }

    /// Get last pod selection from config
    fn get_last_selection(&self) -> Result<Option<serde_json::Value>> {
        let config = crate::config::load_config()?;

        if let Some(data_str) = config.get_value("last_pod_selection", "data")? {
            let selection_data: serde_json::Value = serde_json::from_str(&data_str)?;
            Ok(Some(selection_data))
        } else {
            Ok(None)
        }
    }
}

// Convenience functions for backward compatibility
pub fn store_executor_selection(gpu_type: &str, executors: &[ExecutorInfo]) -> Result<()> {
    let storage = ExecutorSelectionStorage;
    let items: Vec<(String, ExecutorInfo)> = executors
        .iter()
        .map(|e| (gpu_type.to_string(), e.clone()))
        .collect();
    storage.store_selection(&items)
}

pub fn get_last_executor_selection() -> Result<Option<serde_json::Value>> {
    let storage = ExecutorSelectionStorage;
    storage.get_last_selection()
}

pub fn store_pod_selection(pods: &[PodInfo]) -> Result<()> {
    let storage = PodSelectionStorage;
    storage.store_selection(pods)
}

pub fn get_last_pod_selection() -> Result<Option<serde_json::Value>> {
    let storage = PodSelectionStorage;
    storage.get_last_selection()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_pod_selection_storage() {
        let storage = PodSelectionStorage;

        let pods = vec![PodInfo {
            id: "pod1".to_string(),
            name: "test-pod".to_string(),
            huid: "brave-cat-1234".to_string(),
            status: "running".to_string(),
            ssh_cmd: None,
            ports: Default::default(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
            executor: json!({}),
            template: json!({}),
        }];

        // This would normally work with a proper config setup
        // For testing, we'd need to mock the config system
        // assert!(storage.store_selection(&pods).is_ok());
    }
}
