use crate::errors::{ApiError, HttpError, Result};
use lium_core::{
    ApiExecutorResponse, ApiPodResponse, ApiTemplateResponse, ExecutorInfo, PodInfo, TemplateInfo,
};
use log::{debug, error, info, trace};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;

/// Trait for providing configuration to the API client
/// This allows the main application to implement config without circular dependencies
pub trait ApiConfig {
    type Error;

    /// Get the API key for authentication
    fn get_api_key(&self) -> std::result::Result<String, Self::Error>;

    /// Get the base URL for the API (optional, defaults to official API)
    fn get_base_url(&self) -> std::result::Result<Option<String>, Self::Error> {
        Ok(None)
    }
}

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
        let base_url = base_url.unwrap_or_else(|| "https://celiumcompute.ai/api".to_string());

        debug!("Creating LiumApiClient");
        debug!(
            "  API Key: {}...{}",
            &api_key[..4.min(api_key.len())],
            if api_key.len() > 4 {
                &api_key[api_key.len().saturating_sub(4)..]
            } else {
                ""
            }
        );
        debug!("  Base URL: {}", base_url);

        Self {
            client,
            api_key,
            base_url,
        }
    }

    /// Create API client from environment variable
    pub fn from_env() -> Result<Self> {
        debug!("Creating LiumApiClient from environment variable");
        let api_key = std::env::var("LIUM_API_KEY").map_err(|_| {
            error!("LIUM_API_KEY environment variable not set");
            ApiError::Http(HttpError::Config(
                "LIUM_API_KEY environment variable not set".to_string(),
            ))
        })?;

        debug!("Found API key in environment");
        Ok(Self::new(api_key, None))
    }

    /// Create API client from provided API key (convenience method)
    pub fn from_api_key(api_key: String) -> Self {
        debug!("Creating LiumApiClient with provided API key");
        Self::new(api_key, None)
    }

    /// Create API client with custom base URL  
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        debug!("Creating LiumApiClient with custom base URL: {}", base_url);
        Self::new(api_key, Some(base_url))
    }

    /// Create API client from any configuration implementing ApiConfig trait
    pub fn from_config<C>(config: &C) -> std::result::Result<Self, C::Error>
    where
        C: ApiConfig,
    {
        debug!("Creating LiumApiClient from config");
        let api_key = config.get_api_key()?;

        debug!(
            "Got API key from config: {}...{}",
            &api_key[..4.min(api_key.len())],
            if api_key.len() > 4 {
                &api_key[api_key.len().saturating_sub(4)..]
            } else {
                ""
            }
        );

        let base_url = config.get_base_url()?;

        if let Some(ref url) = base_url {
            debug!("Got custom base URL from config: {}", url);
        } else {
            debug!("Using default base URL");
        }

        Ok(Self::new(api_key, base_url))
    }

    /// Make a GET request
    async fn get(&self, endpoint: &str) -> Result<Response> {
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );

        debug!("HTTP GET request to: {}", url);
        trace!("Request headers:");
        trace!(
            "  X-API-Key: {}...{}",
            &self.api_key[..4.min(self.api_key.len())],
            if self.api_key.len() > 4 {
                &self.api_key[self.api_key.len().saturating_sub(4)..]
            } else {
                ""
            }
        );
        trace!(
            "  Authorization: Bearer {}...{}",
            &self.api_key[..4.min(self.api_key.len())],
            if self.api_key.len() > 4 {
                &self.api_key[self.api_key.len().saturating_sub(4)..]
            } else {
                ""
            }
        );
        trace!("  Content-Type: application/json");

        let response = self
            .client
            .get(&url)
            // Try multiple header formats - the API will use whichever it expects
            .header("X-API-Key", &self.api_key)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| {
                error!("GET request failed: {:?}", e);
                HttpError::Request(e)
            })?;

        debug!("Response status: {}", response.status());

        self.handle_response(response).await
    }

    /// Make a POST request
    async fn post(&self, endpoint: &str, body: Option<Value>) -> Result<Response> {
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );

        debug!("HTTP POST request to: {}", url);
        trace!("Request headers:");
        trace!(
            "  X-API-Key: {}...{}",
            &self.api_key[..4.min(self.api_key.len())],
            if self.api_key.len() > 4 {
                &self.api_key[self.api_key.len().saturating_sub(4)..]
            } else {
                ""
            }
        );
        trace!(
            "  Authorization: Bearer {}...{}",
            &self.api_key[..4.min(self.api_key.len())],
            if self.api_key.len() > 4 {
                &self.api_key[self.api_key.len().saturating_sub(4)..]
            } else {
                ""
            }
        );
        trace!("  Content-Type: application/json");

        if let Some(ref body) = body {
            trace!(
                "Request body: {}",
                serde_json::to_string_pretty(body).unwrap_or_else(|_| "Invalid JSON".to_string())
            );
        }

        let mut request = self
            .client
            .post(&url)
            // Try multiple header formats
            .header("X-API-Key", &self.api_key)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.map_err(|e| {
            error!("POST request failed: {:?}", e);
            HttpError::Request(e)
        })?;

        debug!("Response status: {}", response.status());

        self.handle_response(response).await
    }

    /// Make a DELETE request
    async fn delete(&self, endpoint: &str) -> Result<Response> {
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );

        debug!("HTTP DELETE request to: {}", url);
        trace!("Request headers:");
        trace!(
            "  X-API-Key: {}...{}",
            &self.api_key[..4.min(self.api_key.len())],
            if self.api_key.len() > 4 {
                &self.api_key[self.api_key.len().saturating_sub(4)..]
            } else {
                ""
            }
        );
        trace!(
            "  Authorization: Bearer {}...{}",
            &self.api_key[..4.min(self.api_key.len())],
            if self.api_key.len() > 4 {
                &self.api_key[self.api_key.len().saturating_sub(4)..]
            } else {
                ""
            }
        );
        trace!("  Content-Type: application/json");

        let response = self
            .client
            .delete(&url)
            // Try multiple header formats
            .header("X-API-Key", &self.api_key)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| {
                error!("DELETE request failed: {:?}", e);
                HttpError::Request(e)
            })?;

        debug!("Response status: {}", response.status());

        self.handle_response(response).await
    }

    /// Handle HTTP response and convert errors
    async fn handle_response(&self, response: Response) -> Result<Response> {
        let status = response.status();

        if status.is_success() {
            debug!("Request successful with status: {}", status);
            Ok(response)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            error!("Request failed with status: {}", status);
            debug!("Error response body: {}", error_text);

            let api_error = match status {
                StatusCode::UNAUTHORIZED => {
                    error!("Authentication failed (401 Unauthorized)");
                    HttpError::AuthenticationFailed
                }
                StatusCode::FORBIDDEN => {
                    error!("Invalid API key (403 Forbidden)");
                    HttpError::InvalidApiKey
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    error!("Rate limited (429 Too Many Requests)");
                    HttpError::RateLimited
                }
                StatusCode::SERVICE_UNAVAILABLE => {
                    error!("Service unavailable (503)");
                    HttpError::ServiceUnavailable
                }
                StatusCode::REQUEST_TIMEOUT => {
                    error!("Request timeout (408)");
                    HttpError::Timeout
                }
                _ => {
                    error!("HTTP error with status code: {}", status.as_u16());
                    HttpError::HttpError {
                        status: status.as_u16(),
                        message: error_text,
                    }
                }
            };

            Err(ApiError::Http(api_error))
        }
    }

    /// Get list of available executors
    pub async fn get_executors(&self) -> Result<Vec<ExecutorInfo>> {
        debug!("Fetching executors");
        let response = self.get("executors").await?;
        let raw_executors: Vec<ApiExecutorResponse> =
            response.json().await.map_err(HttpError::Request)?;

        info!("Successfully fetched {} executors", raw_executors.len());

        // Convert to ExecutorInfo with derived fields
        let executors = raw_executors.into_iter().map(|raw| raw.into()).collect();

        Ok(executors)
    }

    /// Get list of active pods
    pub async fn get_pods(&self) -> Result<Vec<PodInfo>> {
        debug!("Fetching pods");
        let response = self.get("pods").await?;
        let raw_pods: Vec<ApiPodResponse> = response.json().await.map_err(HttpError::Request)?;

        info!("Successfully fetched {} pods", raw_pods.len());

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
        debug!("Renting pod");
        debug!("  executor_id: {}", executor_id);
        debug!("  pod_name: {}", pod_name);
        debug!("  template_id: {}", template_id);
        debug!("  ssh_keys count: {}", user_public_keys.len());

        let body = serde_json::json!({
            "pod_name": pod_name,
            "template_id": template_id,
            "user_public_key": user_public_keys,  // Note: API expects "user_public_key" not "ssh_public_keys"
        });

        // Use the correct endpoint: /executors/{executor_id}/rent
        let endpoint = format!("executors/{}/rent", executor_id);
        let response = self.post(&endpoint, Some(body)).await?;
        let result: Value = response.json().await.map_err(HttpError::Request)?;

        info!("Successfully rented pod: {}", pod_name);

        Ok(result)
    }

    /// Stop/unrent a pod
    pub async fn unrent_pod(&self, executor_id: &str) -> Result<Value> {
        debug!("Unrenting pod with executor_id: {}", executor_id);

        // Use DELETE /executors/{executor_id}/rent
        let endpoint = format!("executors/{}/rent", executor_id);
        let response = self.delete(&endpoint).await?;
        let result: Value = response.json().await.map_err(HttpError::Request)?;

        info!("Successfully unrented pod");

        Ok(result)
    }

    /// Get available templates
    pub async fn get_templates(&self) -> Result<Vec<TemplateInfo>> {
        debug!("Fetching templates");
        let response = self.get("templates").await?;
        let raw_templates: Vec<ApiTemplateResponse> =
            response.json().await.map_err(HttpError::Request)?;

        info!("Successfully fetched {} templates", raw_templates.len());

        let templates = raw_templates.into_iter().map(|raw| raw.into()).collect();

        Ok(templates)
    }

    /// Post a new Docker image
    pub async fn post_image(&self, image_name: &str, digest: &str, tag: &str) -> Result<Value> {
        debug!("Posting Docker image: {}:{}", image_name, tag);
        let body = serde_json::json!({
            "image_name": image_name,
            "digest": digest,
            "tag": tag,
        });

        let response = self.post("images", Some(body)).await?;
        let result: Value = response.json().await.map_err(HttpError::Request)?;

        info!("Successfully posted Docker image");

        Ok(result)
    }

    /// Get funding wallets
    pub async fn get_funding_wallets(&self) -> Result<Value> {
        debug!("Fetching funding wallets");
        let response = self.get("funding/wallets").await?;
        let result: Value = response.json().await.map_err(HttpError::Request)?;

        info!("Successfully fetched funding wallets");

        Ok(result)
    }

    /// Get user information
    pub async fn get_users_me(&self) -> Result<Value> {
        debug!("Fetching user information");
        let response = self.get("users/me").await?;
        let result: Value = response.json().await.map_err(HttpError::Request)?;

        info!("Successfully fetched user information");

        Ok(result)
    }

    /// Get access key for wallet operations
    pub async fn get_access_key(&self) -> Result<String> {
        debug!("Getting access key from user info");
        let user_info = self.get_users_me().await?;
        let access_key = user_info
            .get("access_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HttpError::Config("Access key not found in user info".to_string()))?;

        debug!("Successfully retrieved access key");

        Ok(access_key.to_string())
    }

    /// Get app ID for wallet operations
    pub async fn get_app_id(&self) -> Result<String> {
        debug!("Getting app ID from user info");
        let user_info = self.get_users_me().await?;
        let app_id = user_info
            .get("app_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HttpError::Config("App ID not found in user info".to_string()))?;

        debug!("Successfully retrieved app ID");

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
        debug!("Adding wallet: {}", coldkey_ss58);
        let body = serde_json::json!({
            "coldkey_ss58": coldkey_ss58,
            "access_key": access_key,
            "signature_hex": signature_hex,
            "app_id": app_id,
        });

        let response = self.post("funding/wallets", Some(body)).await?;
        let result: Value = response.json().await.map_err(HttpError::Request)?;

        info!("Successfully added wallet");

        Ok(result)
    }

    /// Test connection to the API
    pub async fn test_connection(&self) -> Result<bool> {
        debug!("Testing API connection");
        match self.get("health").await {
            Ok(_) => {
                info!("API connection successful");
                Ok(true)
            }
            Err(e) => {
                error!("API connection failed: {:?}", e);
                Ok(false)
            }
        }
    }

    /// Execute command in a pod
    pub async fn exec_pod(&self, request: &Value) -> Result<String> {
        debug!("Executing command in pod");
        let response = self.post("pods/exec", Some(request.clone())).await?;
        let response = self.handle_response(response).await?;
        let result: Value = response.json().await?;

        // Extract command output from response
        if let Some(output) = result.get("output").and_then(|v| v.as_str()) {
            debug!("Command execution successful");
            Ok(output.to_string())
        } else {
            debug!("Using full response as command output");
            Ok(result.to_string())
        }
    }
}

// TODO: Add retry logic for transient failures
// TODO: Add request/response logging
// TODO: Add rate limiting handling
// TODO: Add pagination support for large result sets
// TODO: Add caching for frequently accessed data
