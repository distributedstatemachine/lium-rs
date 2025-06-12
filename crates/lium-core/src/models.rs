use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Simple utility functions for core crate (no external dependencies)
fn extract_gpu_model_simple(machine_name: &str) -> String {
    // Very basic GPU model extraction - the full version is in lium-utils
    if machine_name.to_lowercase().contains("rtx") {
        "RTX".to_string()
    } else if machine_name.to_lowercase().contains("gtx") {
        "GTX".to_string()
    } else if machine_name.to_lowercase().contains("tesla") {
        "Tesla".to_string()
    } else if machine_name.to_lowercase().contains("h100") {
        "H100".to_string()
    } else if machine_name.to_lowercase().contains("a100") {
        "A100".to_string()
    } else {
        "Unknown".to_string()
    }
}

fn generate_human_id_simple(uuid: &str) -> String {
    // Very basic human ID generation - the full version is in lium-utils
    let hash = uuid.chars().take(8).collect::<String>();
    format!("exec-{}", hash)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ExecutorInfo {
    pub id: String,
    pub huid: String, // To be generated client-side
    pub machine_name: String,
    pub gpu_type: String, // To be extracted client-side
    pub gpu_count: i32,
    pub price_per_hour: f64,
    pub price_per_gpu_hour: f64, // To be calculated client-side
    pub location: HashMap<String, String>,
    pub specs: serde_json::Value, // Or more detailed structs
    pub status: String,           // From API; "active" field might mean rented
    pub available: bool,          // Derived field based on API's "active" status or similar logic
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PodInfo {
    pub id: String,
    pub name: String, // Corresponds to "pod_name" from API
    pub status: String,
    pub huid: String, // To be generated client-side
    #[serde(rename = "ssh_connect_cmd")]
    pub ssh_cmd: Option<String>,
    #[serde(rename = "ports_mapping")]
    pub ports: HashMap<String, i32>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub created_at: Option<DateTime<Utc>>, // Parse from API string
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub updated_at: Option<DateTime<Utc>>, // Parse from API string
    pub executor: serde_json::Value, // Or ExecutorInfo (if API nests it fully)
    pub template: serde_json::Value, // Or TemplateInfo
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub docker_image: String,
    pub docker_image_tag: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
}

// Raw API response structures for parsing
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiExecutorResponse {
    pub id: String,
    pub machine_name: String,

    // Make gpu_count optional with default value
    #[serde(default = "default_gpu_count")]
    pub gpu_count: i32,

    pub price_per_hour: f64,

    // Changed to handle both strings and numbers (like coordinates)
    #[serde(default)]
    pub location: HashMap<String, serde_json::Value>, // Changed from HashMap<String, String>

    #[serde(default)]
    pub specs: serde_json::Value,

    pub active: Option<bool>, // Maps to status/available

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>, // Catch unknown fields
}

// Default function for gpu_count
fn default_gpu_count() -> i32 {
    1 // Default to 1 GPU if not specified
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiPodResponse {
    pub id: String,
    pub pod_name: String,
    pub status: String,
    pub ssh_connect_cmd: Option<String>,
    pub ports_mapping: HashMap<String, i32>,
    pub created_at: String,
    pub updated_at: String,
    pub executor: serde_json::Value,
    pub template: serde_json::Value,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>, // Catch unknown fields
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiTemplateResponse {
    pub id: String,
    pub name: String,
    pub docker_image: String,
    pub docker_image_tag: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>, // Catch unknown fields
}

// Utility functions for conversions
impl From<ApiExecutorResponse> for ExecutorInfo {
    fn from(api_response: ApiExecutorResponse) -> Self {
        let gpu_type = extract_gpu_model_simple(&api_response.machine_name);
        let huid = generate_human_id_simple(&api_response.id);

        // Try to determine GPU count from various sources
        let gpu_count = determine_gpu_count(&api_response);

        let price_per_gpu_hour = if gpu_count > 0 {
            api_response.price_per_hour / gpu_count as f64
        } else {
            api_response.price_per_hour
        };

        // Convert location from HashMap<String, Value> to HashMap<String, String>
        let location: HashMap<String, String> = api_response
            .location
            .into_iter()
            .map(|(k, v)| {
                let value_str = match v {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    _ => v.to_string().trim_matches('"').to_string(),
                };
                (k, value_str)
            })
            .collect();

        ExecutorInfo {
            id: api_response.id,
            huid,
            machine_name: api_response.machine_name,
            gpu_type,
            gpu_count,
            price_per_hour: api_response.price_per_hour,
            price_per_gpu_hour,
            location,
            specs: api_response.specs,
            status: if api_response.active.unwrap_or(false) {
                "rented".to_string()
            } else {
                "available".to_string()
            },
            available: !api_response.active.unwrap_or(false),
        }
    }
}

// Helper function to determine GPU count from various sources
fn determine_gpu_count(api_response: &ApiExecutorResponse) -> i32 {
    // First, use the gpu_count field if available and valid
    if api_response.gpu_count > 0 {
        return api_response.gpu_count;
    }

    // Try to extract from specs
    if let Some(gpu_info) = api_response.specs.get("gpu_count") {
        if let Some(count) = gpu_info.as_i64() {
            if count > 0 {
                return count as i32;
            }
        }
        if let Some(count_str) = gpu_info.as_str() {
            if let Ok(count) = count_str.parse::<i32>() {
                if count > 0 {
                    return count;
                }
            }
        }
    }

    // Try to extract from extra fields
    if let Some(gpu_info) = api_response.extra.get("gpu_count") {
        if let Some(count) = gpu_info.as_i64() {
            if count > 0 {
                return count as i32;
            }
        }
    }

    // Try to extract from machine name (e.g., "machine-4x-rtx4090")
    let machine_lower = api_response.machine_name.to_lowercase();
    if let Some(x_pos) = machine_lower.find('x') {
        let before_x = &machine_lower[..x_pos];
        if let Some(dash_pos) = before_x.rfind('-') {
            let count_str = &before_x[dash_pos + 1..];
            if let Ok(count) = count_str.parse::<i32>() {
                if count > 0 {
                    return count;
                }
            }
        }
    }

    // Default fallback
    1
}

impl From<ApiPodResponse> for PodInfo {
    fn from(api_response: ApiPodResponse) -> Self {
        let huid = generate_human_id_simple(&api_response.id);

        // Try to parse dates
        let created_at = chrono::DateTime::parse_from_rfc3339(&api_response.created_at)
            .ok()
            .map(|dt| dt.with_timezone(&Utc));
        let updated_at = chrono::DateTime::parse_from_rfc3339(&api_response.updated_at)
            .ok()
            .map(|dt| dt.with_timezone(&Utc));

        PodInfo {
            id: api_response.id,
            name: api_response.pod_name,
            status: api_response.status,
            huid,
            ssh_cmd: api_response.ssh_connect_cmd,
            ports: api_response.ports_mapping,
            created_at,
            updated_at,
            executor: api_response.executor,
            template: api_response.template,
        }
    }
}

impl From<ApiTemplateResponse> for TemplateInfo {
    fn from(api_response: ApiTemplateResponse) -> Self {
        TemplateInfo {
            id: api_response.id,
            name: api_response.name,
            docker_image: api_response.docker_image,
            docker_image_tag: api_response.docker_image_tag,
            status: api_response.status,
            description: api_response.description,
        }
    }
}

// TODO: Add validation functions for each model
// TODO: Add builder patterns for creating instances
// TODO: Add display formatters for CLI output
