use crate::api::LiumApiClient;
use crate::config::Config;
use crate::display::{display_pod_details, display_pods_table};
use crate::errors::Result;
use crate::utils::parse_executor_index;
use clap::Args;

#[derive(Args)]
pub struct PsArgs {
    /// Show detailed information for a specific pod (by index)
    #[arg(short, long)]
    pub details: Option<String>,

    /// Show all pods (including stopped)
    #[arg(short, long)]
    pub all: bool,

    /// Filter by pod status (running, starting, stopped)
    #[arg(short, long)]
    pub status: Option<String>,

    /// Show only pods with specific GPU type
    #[arg(short, long)]
    pub gpu: Option<String>,
}

pub async fn handle_ps(args: PsArgs, _config: &Config) -> Result<()> {
    let client = LiumApiClient::from_config()?;

    // Fetch pods from API
    let mut pods = client.get_pods().await?;

    if pods.is_empty() {
        println!("No pods found.");
        return Ok(());
    }

    // Apply filters
    if !args.all {
        // Default: show only running and starting pods
        pods = pods
            .into_iter()
            .filter(|pod| pod.status == "running" || pod.status == "starting")
            .collect();
    }

    if let Some(status_filter) = &args.status {
        let status_lower = status_filter.to_lowercase();
        pods = pods
            .into_iter()
            .filter(|pod| pod.status.to_lowercase() == status_lower)
            .collect();
    }

    if let Some(gpu_filter) = &args.gpu {
        let gpu_upper = gpu_filter.to_uppercase();
        pods = pods
            .into_iter()
            .filter(|pod| {
                pod.executor
                    .get("gpu_type")
                    .and_then(|v| v.as_str())
                    .map(|gpu| gpu.to_uppercase().contains(&gpu_upper))
                    .unwrap_or(false)
            })
            .collect();
    }

    if pods.is_empty() {
        println!("No pods found matching your criteria.");
        return Ok(());
    }

    // Show details for specific pod
    if let Some(index_str) = &args.details {
        let index = parse_executor_index(index_str, pods.len())?;
        let pod = &pods[index];
        display_pod_details(pod);
        return Ok(());
    }

    // Display pods table
    display_pods_table(&pods);

    // Show summary
    let running_count = pods.iter().filter(|p| p.status == "running").count();
    let starting_count = pods.iter().filter(|p| p.status == "starting").count();
    let stopped_count = pods.iter().filter(|p| p.status == "stopped").count();

    println!();
    println!(
        "Summary: {} running, {} starting, {} stopped",
        running_count, starting_count, stopped_count
    );

    // Calculate total cost
    let total_cost: f64 = pods
        .iter()
        .filter(|p| p.status == "running" || p.status == "starting")
        .map(|pod| {
            pod.executor
                .get("price_per_hour")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
        })
        .sum();

    if total_cost > 0.0 {
        println!("Total hourly cost: ${:.3}/hr", total_cost);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::models::PodInfo;

    fn create_test_pod(huid: &str, status: &str, gpu_type: &str) -> PodInfo {
        let executor = serde_json::json!({
            "gpu_type": gpu_type,
            "price_per_hour": 1.5
        });

        let template = serde_json::json!({
            "id": "template_1",
            "name": "test_template"
        });

        PodInfo {
            id: format!("pod_{}", huid),
            huid: huid.to_string(),
            name: format!("test_pod_{}", huid),
            status: status.to_string(),
            executor,
            template,
            ports: HashMap::new(),
            ssh_cmd: Some(format!("ssh user@{}.example.com", huid)),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }

    #[test]
    fn test_filter_by_status() {
        let pods = vec![
            create_test_pod("1", "running", "RTX4090"),
            create_test_pod("2", "stopped", "H100"),
            create_test_pod("3", "starting", "RTX4090"),
        ];

        let running_pods: Vec<_> = pods.iter().filter(|p| p.status == "running").collect();
        assert_eq!(running_pods.len(), 1);
        assert_eq!(running_pods[0].huid, "1");
    }

    #[test]
    fn test_filter_by_gpu_type() {
        let pods = vec![
            create_test_pod("1", "running", "RTX4090"),
            create_test_pod("2", "running", "H100"),
            create_test_pod("3", "running", "RTX4090"),
        ];

        let rtx_pods: Vec<_> = pods
            .iter()
            .filter(|pod| {
                pod.executor
                    .get("gpu_type")
                    .and_then(|v| v.as_str())
                    .map(|gpu| gpu.contains("RTX4090"))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(rtx_pods.len(), 2);
    }
}

// TODO: Add real-time status updates
// TODO: Add cost tracking and billing information
// TODO: Add pod logs viewing capability
// TODO: Add resource usage monitoring
// TODO: Add pod grouping and tagging
