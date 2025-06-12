use crate::errors::{ApiError, HttpError, Result};
use lium_core::{
    ApiExecutorResponse, ApiPodResponse, ApiTemplateResponse, ExecutorInfo, PodInfo, TemplateInfo,
};
use log::{debug, error, info, trace};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;

/// Trait for providing configuration to the API client.
///
/// This trait allows the main application to implement configuration without creating circular dependencies.
/// It provides a flexible way to inject API configuration into the client while keeping the client
/// independent of specific configuration implementations.
///
/// # Type Parameters
/// * `Error` - The error type that can be returned by the configuration methods
///
/// # Examples
/// ```rust
/// use lium_api::client::ApiConfig;
///
/// struct MyConfig {
///     api_key: String,
///     base_url: Option<String>,
/// }
///
/// impl ApiConfig for MyConfig {
///     type Error = String;
///     
///     fn get_api_key(&self) -> Result<String, Self::Error> {
///         Ok(self.api_key.clone())
///     }
///     
///     fn get_base_url(&self) -> Result<Option<String>, Self::Error> {
///         Ok(self.base_url.clone())
///     }
/// }
/// ```
pub trait ApiConfig {
    /// The error type that can be returned by the configuration methods
    type Error;

    /// Retrieves the API key for authentication.
    ///
    /// # Returns
    /// * `Ok(String)` - The API key if successfully retrieved
    /// * `Err(Self::Error)` - An error if the API key could not be retrieved
    fn get_api_key(&self) -> std::result::Result<String, Self::Error>;

    /// Retrieves the base URL for the API.
    ///
    /// This method is optional and defaults to the official API URL if not implemented.
    ///
    /// # Returns
    /// * `Ok(Some(String))` - A custom base URL if specified
    /// * `Ok(None)` - Use the default official API URL
    /// * `Err(Self::Error)` - An error if the base URL could not be retrieved
    fn get_base_url(&self) -> std::result::Result<Option<String>, Self::Error> {
        Ok(None)
    }
}

/// HTTP client for interacting with the Celium Compute API.
///
/// This client provides methods for making authenticated HTTP requests to the Celium Compute API.
/// It handles authentication, request formatting, and response parsing.
///
/// # Fields
/// * `client` - The underlying HTTP client for making requests
/// * `api_key` - The API key used for authentication
/// * `base_url` - The base URL for the API endpoints
///
/// # Examples
/// ```rust
/// use lium_api::client::LiumApiClient;
///
/// let client = LiumApiClient::new(
///     "your-api-key".to_string(),
///     Some("https://api.example.com".to_string())
/// );
/// ```
#[derive(Debug, Clone)]
pub struct LiumApiClient {
    /// The underlying HTTP client for making requests
    client: Client,
    /// The API key used for authentication
    api_key: String,
    /// The base URL for the API endpoints
    base_url: String,
}

/// Implementation of the LiumApiClient struct.
///
/// This implementation provides methods for creating and managing API clients for the Celium Compute API.
/// It includes factory methods for different initialization scenarios and handles authentication setup.
///
/// # Factory Methods
/// * `new()` - Creates a new client with specified API key and optional base URL
/// * `from_env()` - Creates a client using API key from environment variable
/// * `from_api_key()` - Creates a client with just an API key
/// * `with_base_url()` - Creates a client with custom base URL
///
/// # Authentication
/// All methods ensure proper API key handling and logging while maintaining security by
/// only logging partial API keys in debug messages.
///
/// # Error Handling
/// Methods return Result types where appropriate, with detailed error messages for
/// configuration and environment variable issues.
///
/// # Examples
/// ```rust
/// // Create with API key only
/// let client = LiumApiClient::from_api_key("your-api-key".to_string());
///
/// // Create with custom base URL
/// let client = LiumApiClient::with_base_url(
///     "your-api-key".to_string(),
///     "https://custom-api.example.com".to_string()
/// );
///
/// // Create from environment
/// let client = LiumApiClient::from_env()?;
/// ```
impl LiumApiClient {
    /// Create a new API client
    /// Creates a new instance of LiumApiClient with the specified API key and optional base URL.
    ///
    /// # Arguments
    /// * `api_key` - The API key used for authentication. This should be a valid API key from the Celium Compute platform.
    /// * `base_url` - Optional custom base URL. If not provided, defaults to "https://celiumcompute.ai/api".
    ///
    /// # Returns
    /// A new instance of LiumApiClient configured with the provided credentials.
    ///
    /// # Security
    /// The API key is logged partially (first 4 and last 4 characters) for debugging purposes.
    /// Full API key is never logged to maintain security.
    ///
    /// # Examples
    /// ```rust
    /// let client = LiumApiClient::new(
    ///     "your-api-key".to_string(),
    ///     Some("https://custom-api.example.com".to_string())
    /// );
    /// ```
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

    /// Creates a new LiumApiClient instance using the API key from the LIUM_API_KEY environment variable.
    ///
    /// # Returns
    /// * `Ok(LiumApiClient)` - A new client instance if the environment variable is set
    /// * `Err(ApiError)` - If the LIUM_API_KEY environment variable is not set
    ///
    /// # Examples
    /// ```rust
    /// let client = LiumApiClient::from_env()?;
    /// ```
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

    /// Creates a new LiumApiClient instance with the provided API key.
    /// This is a convenience method that uses the default base URL.
    ///
    /// # Arguments
    /// * `api_key` - The API key to use for authentication
    ///
    /// # Returns
    /// A new instance of LiumApiClient
    ///
    /// # Examples
    /// ```rust
    /// let client = LiumApiClient::from_api_key("your-api-key".to_string());
    /// ```
    pub fn from_api_key(api_key: String) -> Self {
        debug!("Creating LiumApiClient with provided API key");
        Self::new(api_key, None)
    }

    /// Creates a new LiumApiClient instance with a custom base URL.
    ///
    /// # Arguments
    /// * `api_key` - The API key to use for authentication
    /// * `base_url` - The custom base URL to use for API requests
    ///
    /// # Returns
    /// A new instance of LiumApiClient configured with the custom base URL
    ///
    /// # Examples
    /// ```rust
    /// let client = LiumApiClient::with_base_url(
    ///     "your-api-key".to_string(),
    ///     "https://custom-api.example.com".to_string()
    /// );
    /// ```
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        debug!("Creating LiumApiClient with custom base URL: {}", base_url);
        Self::new(api_key, Some(base_url))
    }

    /// Creates a new LiumApiClient instance from any configuration implementing the ApiConfig trait.
    ///
    /// # Arguments
    /// * `config` - A configuration object implementing the ApiConfig trait
    ///
    /// # Returns
    /// * `Ok(LiumApiClient)` - A new client instance if configuration is valid
    /// * `Err(C::Error)` - If there's an error retrieving configuration values
    ///
    /// # Examples
    /// ```rust
    /// let config = MyConfig::new();
    /// let client = LiumApiClient::from_config(&config)?;
    /// ```
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

    /// Makes a GET request to the specified endpoint.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint to request, without leading slash
    ///
    /// # Returns
    /// * `Ok(Response)` - The HTTP response if successful
    /// * `Err(ApiError)` - If the request fails
    ///
    /// # Authentication
    /// The request includes both X-API-Key and Authorization headers for compatibility
    /// with different API versions.
    ///
    /// # Examples
    /// ```rust
    /// let response = client.get("users/me").await?;
    /// ```
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

    /// Makes a POST request to the specified endpoint with an optional JSON body.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint to request, without leading slash
    /// * `body` - Optional JSON body to send with the request
    ///
    /// # Returns
    /// * `Ok(Response)` - The HTTP response if successful
    /// * `Err(ApiError)` - If the request fails
    ///
    /// # Authentication
    /// The request includes both X-API-Key and Authorization headers for compatibility
    /// with different API versions.
    ///
    /// # Examples
    /// ```rust
    /// let body = serde_json::json!({
    ///     "name": "example",
    ///     "value": 42
    /// });
    /// let response = client.post("resources", Some(body)).await?;
    /// ```
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

    /// Makes a DELETE request to the specified API endpoint.
    ///
    /// This method handles the construction and execution of DELETE requests to the Celium Compute API.
    /// It includes proper authentication headers and error handling.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint to send the DELETE request to (e.g., "executors/123/rent")
    ///
    /// # Returns
    /// * `Ok(Response)` - The HTTP response if the request was successful
    /// * `Err(ApiError)` - An error if the request failed
    ///
    /// # Security
    /// The API key is logged partially (first 4 and last 4 characters) for debugging purposes.
    /// Full API key is never logged to maintain security.
    ///
    /// # Examples
    /// ```rust
    /// let response = client.delete("executors/123/rent").await?;
    /// ```
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
            // Try multiple header formats for maximum compatibility
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

    /// Handles HTTP responses and converts them into appropriate Result types.
    ///
    /// This method processes HTTP responses and converts them into either successful responses
    /// or appropriate error types based on the HTTP status code and response body.
    ///
    /// # Arguments
    /// * `response` - The HTTP response to process
    ///
    /// # Returns
    /// * `Ok(Response)` - The original response if successful
    /// * `Err(ApiError)` - An appropriate error based on the response status
    ///
    /// # Error Handling
    /// The method handles various HTTP status codes:
    /// * 401 - Authentication failed
    /// * 403 - Invalid API key
    /// * 429 - Rate limited
    /// * 503 - Service unavailable
    /// * 408 - Request timeout
    /// * Other - Generic HTTP error with status code and message
    ///
    /// # Examples
    /// ```rust
    /// let response = client.get("endpoint").await?;
    /// let processed_response = client.handle_response(response).await?;
    /// ```
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

    /// Retrieves a list of available executors from the Celium Compute API.
    ///
    /// This method fetches information about all available executors that can be used
    /// for running pods. The response is automatically converted from the API format
    /// to the internal ExecutorInfo type.
    ///
    /// # Returns
    /// * `Ok(Vec<ExecutorInfo>)` - A vector of executor information if successful
    /// * `Err(ApiError)` - An error if the request failed
    ///
    /// # Examples
    /// ```rust
    /// let executors = client.get_executors().await?;
    /// for executor in executors {
    ///     println!("Executor: {}", executor.id);
    /// }
    /// ```
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

    /// Retrieves a list of all active pods from the Celium Compute API.
    ///
    /// This method fetches information about all currently running pods across all executors.
    /// The response is automatically converted from the API format to the internal PodInfo type.
    ///
    /// # Returns
    /// * `Ok(Vec<PodInfo>)` - A vector of pod information if successful
    /// * `Err(ApiError)` - An error if the request failed
    ///
    /// # Examples
    /// ```rust
    /// let pods = client.get_pods().await?;
    /// for pod in pods {
    ///     println!("Pod: {} on executor {}", pod.name, pod.executor_id);
    /// }
    /// ```
    pub async fn get_pods(&self) -> Result<Vec<PodInfo>> {
        debug!("Fetching pods");
        let response = self.get("pods").await?;
        let raw_pods: Vec<ApiPodResponse> = response.json().await.map_err(HttpError::Request)?;

        info!("Successfully fetched {} pods", raw_pods.len());

        // Convert to PodInfo with derived fields
        let pods = raw_pods.into_iter().map(|raw| raw.into()).collect();

        Ok(pods)
    }

    /// Rents a new pod on a specified executor using a template.
    ///
    /// This method creates a new pod on the specified executor using the provided template
    /// and configuration. The pod will be accessible using the provided SSH public keys.
    ///
    /// # Arguments
    /// * `executor_id` - The ID of the executor to rent the pod on
    /// * `pod_name` - A unique name for the pod
    /// * `template_id` - The ID of the template to use for the pod
    /// * `user_public_keys` - Vector of SSH public keys that will have access to the pod
    ///
    /// # Returns
    /// * `Ok(Value)` - The API response containing pod details if successful
    /// * `Err(ApiError)` - An error if the request failed
    ///
    /// # Examples
    /// ```rust
    /// let result = client.rent_pod(
    ///     "exec-123",
    ///     "my-pod",
    ///     "template-456",
    ///     vec!["ssh-rsa AAAAB3NzaC1yc2EAAAADA...".to_string()]
    /// ).await?;
    /// ```
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

    /// Stops and unrents a pod from an executor.
    ///
    /// This method terminates the pod running on the specified executor and releases
    /// the resources. This is a destructive operation that will stop all running
    /// processes in the pod.
    ///
    /// # Arguments
    /// * `executor_id` - The ID of the executor whose pod should be unrented
    ///
    /// # Returns
    /// * `Ok(Value)` - The API response confirming the unrent if successful
    /// * `Err(ApiError)` - An error if the request failed
    ///
    /// # Examples
    /// ```rust
    /// let result = client.unrent_pod("exec-123").await?;
    /// ```
    pub async fn unrent_pod(&self, executor_id: &str) -> Result<Value> {
        debug!("Unrenting pod with executor_id: {}", executor_id);

        // Use DELETE /executors/{executor_id}/rent
        let endpoint = format!("executors/{}/rent", executor_id);
        let response = self.delete(&endpoint).await?;
        let result: Value = response.json().await.map_err(HttpError::Request)?;

        info!("Successfully unrented pod");

        Ok(result)
    }

    /// Retrieves a list of available pod templates from the Celium Compute API.
    ///
    /// This method fetches information about all available templates that can be used
    /// to create new pods. Templates define the base configuration for pods including
    /// the operating system, pre-installed software, and resource limits.
    ///
    /// # Returns
    /// * `Ok(Vec<TemplateInfo>)` - A vector of template information if successful
    /// * `Err(ApiError)` - An error if the request failed
    ///
    /// # Examples
    /// ```rust
    /// let templates = client.get_templates().await?;
    /// for template in templates {
    ///     println!("Template: {} ({})", template.name, template.id);
    /// }
    /// ```
    pub async fn get_templates(&self) -> Result<Vec<TemplateInfo>> {
        debug!("Fetching templates");
        let response = self.get("templates").await?;
        let raw_templates: Vec<ApiTemplateResponse> =
            response.json().await.map_err(HttpError::Request)?;

        info!("Successfully fetched {} templates", raw_templates.len());

        let templates = raw_templates.into_iter().map(|raw| raw.into()).collect();

        Ok(templates)
    }

    /// Registers a new Docker image with the Celium Compute platform.
    ///
    /// This method allows you to register a Docker image that can be used in pods.
    /// The image must be available in a container registry accessible to the platform.
    ///
    /// # Arguments
    /// * `image_name` - The full name of the Docker image (e.g., "myorg/myapp")
    /// * `digest` - The SHA256 digest of the image
    /// * `tag` - The tag of the image (e.g., "latest", "v1.0.0")
    ///
    /// # Returns
    /// * `Ok(Value)` - The API response confirming the image registration if successful
    /// * `Err(ApiError)` - An error if the request failed
    ///
    /// # Examples
    /// ```rust
    /// let result = client.post_image(
    ///     "myorg/myapp",
    ///     "sha256:1234567890abcdef...",
    ///     "v1.0.0"
    /// ).await?;
    /// ```
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

    /// Retrieves a list of funding wallets associated with the user's account.
    ///
    /// This method fetches all funding wallets that have been registered with the Celium Compute platform.
    /// Each wallet represents a funding source that can be used for platform operations.
    ///
    /// # Returns
    /// * `Ok(Value)` - A JSON value containing the list of funding wallets and their details
    /// * `Err(ApiError)` - An error if the request fails or the response cannot be parsed
    ///
    /// # Examples
    /// ```rust
    /// let wallets = client.get_funding_wallets().await?;
    /// println!("Available funding wallets: {}", wallets);
    /// ```
    pub async fn get_funding_wallets(&self) -> Result<Value> {
        debug!("Fetching funding wallets");
        let response = self.get("funding/wallets").await?;
        let result: Value = response.json().await.map_err(HttpError::Request)?;

        info!("Successfully fetched funding wallets");

        Ok(result)
    }

    /// Retrieves detailed information about the currently authenticated user.
    ///
    /// This method fetches the user's profile information, including their access key,
    /// app ID, and other account details. The response contains sensitive information
    /// that should be handled securely.
    ///
    /// # Returns
    /// * `Ok(Value)` - A JSON value containing the user's profile information
    /// * `Err(ApiError)` - An error if the request fails or the response cannot be parsed
    ///
    /// # Security
    /// The response contains sensitive information including access keys and app IDs.
    /// Ensure proper handling and storage of this data.
    ///
    /// # Examples
    /// ```rust
    /// let user_info = client.get_users_me().await?;
    /// println!("User profile: {}", user_info);
    /// ```
    pub async fn get_users_me(&self) -> Result<Value> {
        debug!("Fetching user information");
        let response = self.get("users/me").await?;
        let result: Value = response.json().await.map_err(HttpError::Request)?;

        info!("Successfully fetched user information");

        Ok(result)
    }

    /// Retrieves the access key required for wallet operations.
    ///
    /// This method extracts the access key from the user's profile information.
    /// The access key is required for various wallet-related operations, including
    /// adding new wallets and managing existing ones.
    ///
    /// # Returns
    /// * `Ok(String)` - The access key if successfully retrieved
    /// * `Err(ApiError)` - An error if the access key cannot be found or retrieved
    ///
    /// # Security
    /// The access key is a sensitive credential that should be handled securely.
    /// Avoid logging or exposing this value.
    ///
    /// # Examples
    /// ```rust
    /// let access_key = client.get_access_key().await?;
    /// // Use access_key for wallet operations
    /// ```
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

    /// Retrieves the application ID required for wallet operations.
    ///
    /// This method extracts the application ID from the user's profile information.
    /// The app ID is required for various wallet-related operations and serves as
    /// an identifier for the application making the requests.
    ///
    /// # Returns
    /// * `Ok(String)` - The application ID if successfully retrieved
    /// * `Err(ApiError)` - An error if the app ID cannot be found or retrieved
    ///
    /// # Examples
    /// ```rust
    /// let app_id = client.get_app_id().await?;
    /// // Use app_id for wallet operations
    /// ```
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

    /// Adds a wallet for funding operations to the user's account.
    ///
    /// This method registers a new wallet that can be used for funding operations on the platform.
    /// It requires several pieces of information to verify ownership and establish the connection:
    /// - A coldkey in SS58 format for wallet identification
    /// - An access key for authentication
    /// - A signature to verify ownership
    /// - An application ID to associate the wallet with
    ///
    /// # Arguments
    /// * `coldkey_ss58` - The SS58-encoded public key of the wallet to add
    /// * `access_key` - The access key for authentication
    /// * `signature_hex` - A hex-encoded signature proving ownership of the wallet
    /// * `app_id` - The application ID to associate with this wallet
    ///
    /// # Returns
    /// * `Ok(Value)` - The API response containing wallet details if successful
    /// * `Err(ApiError)` - An error if the request fails
    ///
    /// # Security
    /// The access key and signature are sensitive credentials that should be handled securely.
    /// Avoid logging or exposing these values.
    ///
    /// # Examples
    /// ```rust
    /// let result = client.add_wallet(
    ///     "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
    ///     "access-key-123",
    ///     "0x1234...",
    ///     "app-456"
    /// ).await?;
    /// ```
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

    /// Tests the connection to the Celium Compute API.
    ///
    /// This method performs a health check request to verify that the API is accessible
    /// and responding correctly. It's useful for validating configuration and network
    /// connectivity before attempting more complex operations.
    ///
    /// # Returns
    /// * `Ok(true)` - If the API is accessible and responding
    /// * `Ok(false)` - If the API is not accessible or not responding correctly
    /// * `Err(ApiError)` - If an unexpected error occurs during the check
    ///
    /// # Examples
    /// ```rust
    /// if client.test_connection().await? {
    ///     println!("API is accessible");
    /// } else {
    ///     println!("API is not accessible");
    /// }
    /// ```
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

    /// Executes a command in a specified pod.
    ///
    /// This method sends a command to be executed in a pod and returns its output.
    /// The command execution is asynchronous and the response includes the command's
    /// output or any error messages that occurred during execution.
    ///
    /// # Arguments
    /// * `request` - A JSON value containing the command execution details
    ///              (e.g., pod ID, command to execute, timeout settings)
    ///
    /// # Returns
    /// * `Ok(String)` - The command output if successful
    /// * `Err(ApiError)` - An error if the command execution fails
    ///
    /// # Examples
    /// ```rust
    /// let request = serde_json::json!({
    ///     "pod_id": "pod-123",
    ///     "command": "ls -la",
    ///     "timeout": 30
    /// });
    /// let output = client.exec_pod(&request).await?;
    /// println!("Command output: {}", output);
    /// ```
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
