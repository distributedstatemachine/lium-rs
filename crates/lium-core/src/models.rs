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
    pub gpu_count: i32,
    pub price_per_hour: f64,
    pub location: HashMap<String, String>,
    pub specs: serde_json::Value,
    pub active: Option<bool>, // Maps to status/available
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>, // Catch unknown fields
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
        let price_per_gpu_hour = if api_response.gpu_count > 0 {
            api_response.price_per_hour / api_response.gpu_count as f64
        } else {
            0.0
        };

        ExecutorInfo {
            id: api_response.id,
            huid,
            machine_name: api_response.machine_name,
            gpu_type,
            gpu_count: api_response.gpu_count,
            price_per_hour: api_response.price_per_hour,
            price_per_gpu_hour,
            location: api_response.location,
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
