use crate::client::LiumApiClient;
use crate::errors::{ApiError, Result};
use lium_core::{ExecutorInfo, PodInfo, TemplateInfo};

/// Main SDK struct for Lium
pub struct Lium {
    api_client: LiumApiClient,
}

impl Lium {
    /// Create new Lium instance with API key
    pub fn new(api_key: String) -> Self {
        let api_client = LiumApiClient::new(api_key, None);
        Self { api_client }
    }

    /// Create from environment variable
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("LIUM_API_KEY").map_err(|_| {
            ApiError::Config("LIUM_API_KEY environment variable not set".to_string())
        })?;
        Ok(Self::new(api_key))
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
        template_id: &str,
        ssh_public_keys: Vec<String>,
    ) -> Result<serde_json::Value> {
        self.api_client
            .rent_pod(executor_id, pod_name, template_id, ssh_public_keys)
            .await
    }

    /// Stop a pod by executor ID
    pub async fn stop_pod(&self, executor_id: &str) -> Result<serde_json::Value> {
        self.api_client.unrent_pod(executor_id).await
    }

    /// Get pod by name or HUID
    pub async fn get_pod_by_name_or_huid(&self, name_or_huid: &str) -> Result<Option<PodInfo>> {
        let pods = self.list_pods().await?;

        Ok(pods
            .into_iter()
            .find(|pod| pod.name == name_or_huid || pod.huid == name_or_huid))
    }

    /// Get executor by HUID
    pub async fn get_executor_by_huid(&self, huid: &str) -> Result<Option<ExecutorInfo>> {
        let executors = self.list_executors(None).await?;

        Ok(executors.into_iter().find(|executor| executor.huid == huid))
    }

    /// Test API connection
    pub async fn test_connection(&self) -> Result<bool> {
        self.api_client.test_connection().await
    }

    /// Get funding wallets
    pub async fn get_funding_wallets(&self) -> Result<serde_json::Value> {
        self.api_client.get_funding_wallets().await
    }

    /// Get user info
    pub async fn get_user_info(&self) -> Result<serde_json::Value> {
        self.api_client.get_users_me().await
    }

    /// Post Docker image
    pub async fn post_image(
        &self,
        image_name: &str,
        digest: &str,
        tag: &str,
    ) -> Result<serde_json::Value> {
        self.api_client.post_image(image_name, digest, tag).await
    }
}
