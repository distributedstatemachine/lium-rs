use crate::errors::{ApiError, LiumError, Result};
use lium_core::{
    ApiExecutorResponse, ApiPodResponse, ApiTemplateResponse, ExecutorInfo, PodInfo, TemplateInfo,
};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;

/// HTTP client for interacting with the Celium Compute API
#[derive(Debug, Clone)]
pub struct LiumApiClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl LiumApiClient {
    /// Create a new API client
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let client = Client::new();
        let base_url = base_url.unwrap_or_else(|| "https://api.celium.com".to_string());

        Self {
            client,
            api_key,
            base_url,
        }
    }

    /// Create API client from config
    pub fn from_config() -> Result<Self> {
        let config = crate::config::load_config()?;
        let api_key = config.get_api_key()?.ok_or_else(|| {
            LiumError::InvalidInput("API key not found. Run 'lium init' to configure.".to_string())
        })?;

        Ok(Self::new(api_key, None))
    }

    /// Make a GET request
    async fn get(&self, endpoint: &str) -> Result<Response> {
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(LiumError::Request)?;

        self.handle_response(response).await
    }

    /// Make a POST request
    async fn post(&self, endpoint: &str, body: Option<Value>) -> Result<Response> {
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );

        let mut request = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.map_err(LiumError::Request)?;

        self.handle_response(response).await
    }

    /// Make a DELETE request
    async fn delete(&self, endpoint: &str) -> Result<Response> {
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(LiumError::Request)?;

        self.handle_response(response).await
    }

    /// Handle HTTP response and convert errors
    async fn handle_response(&self, response: Response) -> Result<Response> {
        let status = response.status();

        if status.is_success() {
            Ok(response)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            let api_error = match status {
                StatusCode::UNAUTHORIZED => ApiError::AuthenticationFailed,
                StatusCode::FORBIDDEN => ApiError::InvalidApiKey,
                StatusCode::TOO_MANY_REQUESTS => ApiError::RateLimited,
                StatusCode::SERVICE_UNAVAILABLE => ApiError::ServiceUnavailable,
                StatusCode::REQUEST_TIMEOUT => ApiError::Timeout,
                _ => ApiError::HttpError {
                    status: status.as_u16(),
                    message: error_text,
                },
            };

            Err(LiumError::Api(api_error))
        }
    }

    /// Get list of available executors
    pub async fn get_executors(&self) -> Result<Vec<ExecutorInfo>> {
        let response = self.get("executors").await?;
        let raw_executors: Vec<ApiExecutorResponse> =
            response.json().await.map_err(LiumError::Request)?;

        // Convert to ExecutorInfo with derived fields
        let executors = raw_executors.into_iter().map(|raw| raw.into()).collect();

        Ok(executors)
    }

    /// Get list of active pods
    pub async fn get_pods(&self) -> Result<Vec<PodInfo>> {
        let response = self.get("pods").await?;
        let raw_pods: Vec<ApiPodResponse> = response.json().await.map_err(LiumError::Request)?;

        // Convert to PodInfo with derived fields
        let pods = raw_pods.into_iter().map(|raw| raw.into()).collect();

        Ok(pods)
    }

    /// Rent a pod on an executor
    pub async fn rent_pod(
        &self,
        executor_id: &str,
        pod_name: &str,
        template_id: &str,
        user_public_keys: Vec<String>,
    ) -> Result<Value> {
        let body = serde_json::json!({
            "executor_id": executor_id,
            "pod_name": pod_name,
            "template_id": template_id,
            "ssh_public_keys": user_public_keys,
        });

        let response = self.post("pods", Some(body)).await?;
        let result: Value = response.json().await.map_err(LiumError::Request)?;

        Ok(result)
    }

    /// Stop/unrent a pod
    pub async fn unrent_pod(&self, executor_id: &str) -> Result<Value> {
        let endpoint = format!("pods/{}", executor_id);
        let response = self.delete(&endpoint).await?;
        let result: Value = response.json().await.map_err(LiumError::Request)?;

        Ok(result)
    }

    /// Get available templates
    pub async fn get_templates(&self) -> Result<Vec<TemplateInfo>> {
        let response = self.get("templates").await?;
        let raw_templates: Vec<ApiTemplateResponse> =
            response.json().await.map_err(LiumError::Request)?;

        let templates = raw_templates.into_iter().map(|raw| raw.into()).collect();

        Ok(templates)
    }

    /// Post a new Docker image
    pub async fn post_image(&self, image_name: &str, digest: &str, tag: &str) -> Result<Value> {
        let body = serde_json::json!({
            "image_name": image_name,
            "digest": digest,
            "tag": tag,
        });

        let response = self.post("images", Some(body)).await?;
        let result: Value = response.json().await.map_err(LiumError::Request)?;

        Ok(result)
    }

    /// Get funding wallets
    pub async fn get_funding_wallets(&self) -> Result<Value> {
        let response = self.get("funding/wallets").await?;
        let result: Value = response.json().await.map_err(LiumError::Request)?;

        Ok(result)
    }

    /// Get user information
    pub async fn get_users_me(&self) -> Result<Value> {
        let response = self.get("users/me").await?;
        let result: Value = response.json().await.map_err(LiumError::Request)?;

        Ok(result)
    }

    /// Get access key for wallet operations
    pub async fn get_access_key(&self) -> Result<String> {
        let user_info = self.get_users_me().await?;
        let access_key = user_info
            .get("access_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                LiumError::InvalidInput("Access key not found in user info".to_string())
            })?;

        Ok(access_key.to_string())
    }

    /// Get app ID for wallet operations
    pub async fn get_app_id(&self) -> Result<String> {
        let user_info = self.get_users_me().await?;
        let app_id = user_info
            .get("app_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LiumError::InvalidInput("App ID not found in user info".to_string()))?;

        Ok(app_id.to_string())
    }

    /// Add a wallet for funding
    pub async fn add_wallet(
        &self,
        coldkey_ss58: &str,
        access_key: &str,
        signature_hex: &str,
        app_id: &str,
    ) -> Result<Value> {
        let body = serde_json::json!({
            "coldkey_ss58": coldkey_ss58,
            "access_key": access_key,
            "signature_hex": signature_hex,
            "app_id": app_id,
        });

        let response = self.post("funding/wallets", Some(body)).await?;
        let result: Value = response.json().await.map_err(LiumError::Request)?;

        Ok(result)
    }

    /// Test connection to the API
    pub async fn test_connection(&self) -> Result<bool> {
        match self.get("health").await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Execute command in a pod
    pub async fn exec_pod(&self, request: &Value) -> Result<String> {
        let response = self.post("pods/exec", Some(request.clone())).await?;
        let response = self.handle_response(response).await?;
        let result: Value = response.json().await?;

        // Extract command output from response
        if let Some(output) = result.get("output").and_then(|v| v.as_str()) {
            Ok(output.to_string())
        } else {
            Ok(result.to_string())
        }
    }
}

// TODO: Add retry logic for transient failures
// TODO: Add request/response logging
// TODO: Add rate limiting handling
// TODO: Add pagination support for large result sets
// TODO: Add caching for frequently accessed data
