use crate::client::LiumApiClient;
use crate::errors::{ApiError, Result};
use lium_core::{ExecutorInfo, PodInfo, TemplateInfo};

/// Main SDK struct for Lium
///
/// This struct provides a high-level interface for interacting with the Lium platform.
/// It encapsulates all the functionality needed to manage executors, pods, templates,
/// and other platform resources.
///
/// # Examples
/// ```rust
/// use lium_api::Lium;
///
/// #[tokio::main]
/// async fn main() -> Result<(), ApiError> {
///     // Create a new Lium instance
///     let lium = Lium::new("your-api-key".to_string());
///     
///     // List available executors
///     let executors = lium.list_executors(None).await?;
///     
///     // Start a pod
///     let pod = lium.start_pod(
///         "exec-123",
///         "my-pod",
///         "template-456",
///         vec!["ssh-rsa AAAAB3NzaC1yc2EAAAADA...".to_string()]
///     ).await?;
///     
///     Ok(())
/// }
/// ```
pub struct Lium {
    /// The underlying API client used for making HTTP requests
    api_client: LiumApiClient,
}

impl Lium {
    /// Creates a new Lium instance with the provided API key.
    ///
    /// # Arguments
    /// * `api_key` - The API key to use for authentication
    ///
    /// # Returns
    /// A new `Lium` instance configured with the provided API key
    ///
    /// # Examples
    /// ```rust
    /// let lium = Lium::new("your-api-key".to_string());
    /// ```
    pub fn new(api_key: String) -> Self {
        let api_client = LiumApiClient::new(api_key, None);
        Self { api_client }
    }

    /// Creates a new Lium instance using the API key from the `LIUM_API_KEY` environment variable.
    ///
    /// # Returns
    /// * `Ok(Lium)` - A new `Lium` instance if the environment variable is set
    /// * `Err(ApiError)` - An error if the environment variable is not set
    ///
    /// # Examples
    /// ```rust
    /// let lium = Lium::from_env()?;
    /// ```
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("LIUM_API_KEY").map_err(|_| {
            ApiError::Config("LIUM_API_KEY environment variable not set".to_string())
        })?;
        Ok(Self::new(api_key))
    }

    /// Lists all available executors, optionally filtered by GPU type.
    ///
    /// # Arguments
    /// * `gpu_type` - Optional GPU type to filter executors by (e.g., "a100", "v100")
    ///
    /// # Returns
    /// * `Ok(Vec<ExecutorInfo>)` - A list of available executors
    /// * `Err(ApiError)` - An error if the request fails
    ///
    /// # Examples
    /// ```rust
    /// // List all executors
    /// let all_executors = lium.list_executors(None).await?;
    ///
    /// // List only A100 executors
    /// let a100_executors = lium.list_executors(Some("a100".to_string())).await?;
    /// ```
    pub async fn list_executors(&self, gpu_type: Option<String>) -> Result<Vec<ExecutorInfo>> {
        let mut executors = self.api_client.get_executors().await?;

        if let Some(gpu_type) = gpu_type {
            executors.retain(|e| e.gpu_type.to_lowercase() == gpu_type.to_lowercase());
        }

        Ok(executors)
    }

    /// Lists all active pods associated with the user's account.
    ///
    /// # Returns
    /// * `Ok(Vec<PodInfo>)` - A list of active pods
    /// * `Err(ApiError)` - An error if the request fails
    ///
    /// # Examples
    /// ```rust
    /// let pods = lium.list_pods().await?;
    /// for pod in pods {
    ///     println!("Pod: {} (HUID: {})", pod.name, pod.huid);
    /// }
    /// ```
    pub async fn list_pods(&self) -> Result<Vec<PodInfo>> {
        self.api_client.get_pods().await
    }

    /// Retrieves all available pod templates.
    ///
    /// # Returns
    /// * `Ok(Vec<TemplateInfo>)` - A list of available templates
    /// * `Err(ApiError)` - An error if the request fails
    ///
    /// # Examples
    /// ```rust
    /// let templates = lium.get_templates().await?;
    /// for template in templates {
    ///     println!("Template: {} (ID: {})", template.name, template.id);
    /// }
    /// ```
    pub async fn get_templates(&self) -> Result<Vec<TemplateInfo>> {
        self.api_client.get_templates().await
    }

    /// Starts a new pod on the specified executor using the given template.
    ///
    /// # Arguments
    /// * `executor_id` - The ID of the executor to start the pod on
    /// * `pod_name` - A unique name for the pod
    /// * `template_id` - The ID of the template to use
    /// * `ssh_public_keys` - A list of SSH public keys to authorize for pod access
    ///
    /// # Returns
    /// * `Ok(Value)` - The API response containing pod details
    /// * `Err(ApiError)` - An error if the request fails
    ///
    /// # Examples
    /// ```rust
    /// let pod = lium.start_pod(
    ///     "exec-123",
    ///     "my-pod",
    ///     "template-456",
    ///     vec!["ssh-rsa AAAAB3NzaC1yc2EAAAADA...".to_string()]
    /// ).await?;
    /// ```
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

    /// Stops a pod running on the specified executor.
    ///
    /// # Arguments
    /// * `executor_id` - The ID of the executor running the pod to stop
    ///
    /// # Returns
    /// * `Ok(Value)` - The API response confirming pod termination
    /// * `Err(ApiError)` - An error if the request fails
    ///
    /// # Examples
    /// ```rust
    /// lium.stop_pod("exec-123").await?;
    /// ```
    pub async fn stop_pod(&self, executor_id: &str) -> Result<serde_json::Value> {
        self.api_client.unrent_pod(executor_id).await
    }

    /// Retrieves a pod by its name or HUID (Hardware Unique Identifier).
    ///
    /// # Arguments
    /// * `name_or_huid` - The name or HUID of the pod to find
    ///
    /// # Returns
    /// * `Ok(Some(PodInfo))` - The pod information if found
    /// * `Ok(None)` - If no pod matches the given name or HUID
    /// * `Err(ApiError)` - An error if the request fails
    ///
    /// # Examples
    /// ```rust
    /// if let Some(pod) = lium.get_pod_by_name_or_huid("my-pod").await? {
    ///     println!("Found pod: {}", pod.name);
    /// }
    /// ```
    pub async fn get_pod_by_name_or_huid(&self, name_or_huid: &str) -> Result<Option<PodInfo>> {
        let pods = self.list_pods().await?;

        Ok(pods
            .into_iter()
            .find(|pod| pod.name == name_or_huid || pod.huid == name_or_huid))
    }

    /// Retrieves an executor by its HUID (Hardware Unique Identifier).
    ///
    /// # Arguments
    /// * `huid` - The HUID of the executor to find
    ///
    /// # Returns
    /// * `Ok(Some(ExecutorInfo))` - The executor information if found
    /// * `Ok(None)` - If no executor matches the given HUID
    /// * `Err(ApiError)` - An error if the request fails
    ///
    /// # Examples
    /// ```rust
    /// if let Some(executor) = lium.get_executor_by_huid("exec-123").await? {
    ///     println!("Found executor: {}", executor.name);
    /// }
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
