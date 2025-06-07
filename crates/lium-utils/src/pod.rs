use crate::errors::{ParseError, Result, UtilsError};
use lium_core::PodInfo;

/// Utility functions for working with pods
pub struct PodUtils;

impl PodUtils {
    /// Get pods that are in a ready state for operations
    pub fn filter_ready_pods(pods: &[PodInfo]) -> Vec<&PodInfo> {
        pods.iter()
            .filter(|pod| {
                matches!(
                    pod.status.to_lowercase().as_str(),
                    "running" | "active" | "ready" | "up"
                )
            })
            .collect()
    }

    /// Get the executor ID from a pod (needed for some API operations)
    pub fn get_executor_id_from_pod(pod: &PodInfo) -> Result<String> {
        // Try to extract executor ID from the pod's executor field
        if let Some(executor_obj) = pod.executor.as_object() {
            if let Some(id) = executor_obj.get("id").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }

        // Fallback: if executor is just a string ID
        if let Some(id) = pod.executor.as_str() {
            return Ok(id.to_string());
        }

        Err(UtilsError::Parse(ParseError::InvalidFormat(format!(
            "Could not determine executor ID for pod '{}'",
            pod.huid
        ))))
    }

    /// Extract SSH connection details from a pod
    pub fn extract_ssh_details(pod: &PodInfo) -> Result<(String, u16, String)> {
        // Parse from ssh_cmd if available
        if let Some(ssh_cmd) = &pod.ssh_cmd {
            return crate::parsers::parse_ssh_command(ssh_cmd);
        }

        // Fallback: use pod huid as host
        let host = pod.huid.clone();
        let port = 22u16; // Default SSH port
        let user = "root".to_string(); // Default user

        Ok((host, port, user))
    }
}

// Convenience functions for backward compatibility
pub fn filter_ready_pods(pods: &[PodInfo]) -> Vec<&PodInfo> {
    PodUtils::filter_ready_pods(pods)
}

pub fn get_executor_id_from_pod(pod: &PodInfo) -> Result<String> {
    PodUtils::get_executor_id_from_pod(pod)
}

pub fn extract_ssh_details(pod: &PodInfo) -> Result<(String, u16, String)> {
    PodUtils::extract_ssh_details(pod)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_filter_ready_pods() {
        let pods = vec![
            PodInfo {
                id: "pod1".to_string(),
                name: "test-pod-1".to_string(),
                huid: "brave-cat-1234".to_string(),
                status: "running".to_string(),
                ssh_cmd: None,
                ports: Default::default(),
                created_at: Some(chrono::Utc::now()),
                updated_at: Some(chrono::Utc::now()),
                executor: json!({}),
                template: json!({}),
            },
            PodInfo {
                id: "pod2".to_string(),
                name: "test-pod-2".to_string(),
                huid: "smart-dog-5678".to_string(),
                status: "stopped".to_string(),
                ssh_cmd: None,
                ports: Default::default(),
                created_at: Some(chrono::Utc::now()),
                updated_at: Some(chrono::Utc::now()),
                executor: json!({}),
                template: json!({}),
            },
        ];

        let ready = PodUtils::filter_ready_pods(&pods);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].status, "running");
    }

    #[test]
    fn test_get_executor_id_from_pod() {
        let pod = PodInfo {
            id: "pod1".to_string(),
            name: "test-pod".to_string(),
            huid: "brave-cat-1234".to_string(),
            status: "running".to_string(),
            ssh_cmd: None,
            ports: Default::default(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
            executor: json!({"id": "exec123"}),
            template: json!({}),
        };

        let executor_id = PodUtils::get_executor_id_from_pod(&pod).unwrap();
        assert_eq!(executor_id, "exec123");
    }

    #[test]
    fn test_extract_ssh_details() {
        let pod = PodInfo {
            id: "pod1".to_string(),
            name: "test-pod".to_string(),
            huid: "brave-cat-1234".to_string(),
            status: "running".to_string(),
            ssh_cmd: Some("ssh -p 2222 root@192.168.1.10".to_string()),
            ports: Default::default(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
            executor: json!({}),
            template: json!({}),
        };

        let (host, port, user) = PodUtils::extract_ssh_details(&pod).unwrap();
        assert_eq!(host, "192.168.1.10");
        assert_eq!(port, 2222);
        assert_eq!(user, "root");
    }
}
