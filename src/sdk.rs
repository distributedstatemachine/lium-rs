use crate::api::LiumApiClient;
use crate::config::Config;
use crate::errors::{LiumError, Result};
use crate::ssh_utils;
use lium_core::{ExecutorInfo, PodInfo, TemplateInfo};
use std::collections::HashMap;
use std::path::Path;

/// Main SDK struct for Lium
pub struct Lium {
    api_client: LiumApiClient,
}

impl Lium {
    /// Create new Lium instance
    pub fn new(api_key: Option<String>) -> Result<Self> {
        let api_key = match api_key {
            Some(key) => key,
            None => {
                // Try environment variable first
                std::env::var("LIUM_API_KEY").or_else(|_| {
                    // Then try config
                    let config = Config::new()?;
                    config.get_api_key()?.ok_or_else(|| {
                        LiumError::InvalidInput(
                            "API key not found. Set LIUM_API_KEY or run 'lium init'".to_string(),
                        )
                    })
                })?
            }
        };

        let api_client = LiumApiClient::new(api_key, None);
        Ok(Self { api_client })
    }

    /// List available executors
    pub async fn list_executors(&self, gpu_type: Option<String>) -> Result<Vec<ExecutorInfo>> {
        let mut executors = self.api_client.get_executors().await?;

        if let Some(gpu_type) = gpu_type {
            executors.retain(|e| e.gpu_type.to_lowercase() == gpu_type.to_lowercase());
        }

        Ok(executors)
    }

    /// List active pods
    pub async fn list_pods(&self) -> Result<Vec<PodInfo>> {
        self.api_client.get_pods().await
    }

    /// Get available templates
    pub async fn get_templates(&self) -> Result<Vec<TemplateInfo>> {
        self.api_client.get_templates().await
    }

    /// Start a pod on an executor
    pub async fn start_pod(
        &self,
        executor_id: &str,
        pod_name: &str,
        template_id: Option<&str>,
        ssh_public_keys: Option<Vec<String>>,
    ) -> Result<serde_json::Value> {
        let template_id = match template_id {
            Some(id) => id.to_string(),
            None => {
                let config = Config::new()?;
                config.get_default_template_id()?.ok_or_else(|| {
                    LiumError::InvalidInput(
                        "No template ID provided and no default set".to_string(),
                    )
                })?
            }
        };

        let public_keys = match ssh_public_keys {
            Some(keys) => keys,
            None => {
                let config = Config::new()?;
                config.get_ssh_public_keys()?
            }
        };

        self.api_client
            .rent_pod(executor_id, pod_name, &template_id, public_keys)
            .await
    }

    /// Stop a pod
    pub async fn stop_pod(&self, pod_identifier: &str) -> Result<serde_json::Value> {
        let pod = self
            .get_pod_by_name_or_huid(pod_identifier)
            .await?
            .ok_or_else(|| LiumError::NotFound(format!("Pod not found: {}", pod_identifier)))?;

        let executor_id = crate::helpers::get_executor_id_from_pod(&pod)?;
        self.api_client.unrent_pod(&executor_id).await
    }

    /// Execute command on a pod
    pub async fn execute_command(
        &self,
        pod_identifier: &str,
        command: &str,
        env_vars: Option<HashMap<String, String>>,
        timeout_secs: Option<u64>,
    ) -> Result<(String, String, i32)> {
        let pod = self
            .get_pod_by_name_or_huid(pod_identifier)
            .await?
            .ok_or_else(|| LiumError::NotFound(format!("Pod not found: {}", pod_identifier)))?;

        let (host, port, user) = crate::helpers::extract_ssh_details(&pod)?;
        let config = Config::new()?;
        let private_key_path = config.get_ssh_private_key_path()?;

        // Build command with environment variables
        let full_command = if let Some(env_vars) = env_vars {
            let env_prefix: Vec<String> = env_vars
                .iter()
                .map(|(k, v)| format!("export {}='{}'", k, v))
                .collect();
            format!("{}; {}", env_prefix.join("; "), command)
        } else {
            command.to_string()
        };

        ssh_utils::execute_remote_command(
            &host,
            port,
            &user,
            &private_key_path,
            &full_command,
            None,
        )
        .await
    }

    /// Upload file to pod
    pub async fn upload_file(
        &self,
        pod_identifier: &str,
        local_path: &Path,
        remote_path: &str,
        _timeout_secs: Option<u64>,
    ) -> Result<()> {
        let pod = self
            .get_pod_by_name_or_huid(pod_identifier)
            .await?
            .ok_or_else(|| LiumError::NotFound(format!("Pod not found: {}", pod_identifier)))?;

        let (host, port, user) = crate::helpers::extract_ssh_details(&pod)?;
        let config = Config::new()?;
        let private_key_path = config.get_ssh_private_key_path()?;

        ssh_utils::upload_file_sftp(
            &host,
            port,
            &user,
            &private_key_path,
            local_path,
            remote_path,
        )
        .await
    }

    /// Download file from pod
    pub async fn download_file(
        &self,
        pod_identifier: &str,
        remote_path: &str,
        local_path: &Path,
        _timeout_secs: Option<u64>,
    ) -> Result<()> {
        let pod = self
            .get_pod_by_name_or_huid(pod_identifier)
            .await?
            .ok_or_else(|| LiumError::NotFound(format!("Pod not found: {}", pod_identifier)))?;

        let (host, port, user) = crate::helpers::extract_ssh_details(&pod)?;
        let config = Config::new()?;
        let private_key_path = config.get_ssh_private_key_path()?;

        ssh_utils::download_file_sftp(
            &host,
            port,
            &user,
            &private_key_path,
            remote_path,
            local_path,
        )
        .await
    }

    /// Sync directory with pod using rsync
    pub async fn sync_directory(
        &self,
        pod_identifier: &str,
        local_path: &Path,
        remote_path: &str,
        direction: RSyncDirection,
        delete: bool,
        exclude: Option<Vec<String>>,
    ) -> Result<()> {
        let pod = self
            .get_pod_by_name_or_huid(pod_identifier)
            .await?
            .ok_or_else(|| LiumError::NotFound(format!("Pod not found: {}", pod_identifier)))?;

        let (host, port, user) = crate::helpers::extract_ssh_details(&pod)?;
        let config = Config::new()?;
        let private_key_path = config.get_ssh_private_key_path()?;

        crate::utils::rsync_directory(
            local_path,
            remote_path,
            &host,
            port,
            &user,
            &private_key_path,
            direction,
            delete,
            exclude,
        )
        .await
    }

    /// Wait for pod to be ready
    pub async fn wait_for_pod_ready(
        &self,
        pod_identifier: &str,
        max_wait_secs: Option<u64>,
        check_interval_secs: Option<u64>,
    ) -> Result<bool> {
        let max_wait = max_wait_secs.unwrap_or(300); // 5 minutes default
        let interval = check_interval_secs.unwrap_or(10); // 10 seconds default

        let start_time = std::time::Instant::now();

        loop {
            if let Some(pod) = self.get_pod_by_name_or_huid(pod_identifier).await? {
                if !crate::helpers::filter_ready_pods(&[pod]).is_empty() {
                    return Ok(true);
                }
            }

            if start_time.elapsed().as_secs() >= max_wait {
                return Ok(false);
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    }

    /// Get pod by name or HUID
    pub async fn get_pod_by_name_or_huid(&self, name_or_huid: &str) -> Result<Option<PodInfo>> {
        let pods = self.list_pods().await?;

        for pod in pods {
            if pod.name == name_or_huid || pod.huid == name_or_huid || pod.id == name_or_huid {
                return Ok(Some(pod));
            }
        }

        Ok(None)
    }

    /// Get executor by HUID
    pub async fn get_executor_by_huid(&self, huid: &str) -> Result<Option<ExecutorInfo>> {
        let executors = self.list_executors(None).await?;

        for executor in executors {
            if executor.huid == huid || executor.id == huid {
                return Ok(Some(executor));
            }
        }

        Ok(None)
    }
}

/// Direction for rsync operations
#[derive(Debug, Clone, Copy)]
pub enum RSyncDirection {
    Upload,   // Local to remote
    Download, // Remote to local
}
