use crate::config::Config;
use crate::display::{display_pod_details, display_pods_table};
use crate::helpers::{resolve_pod_targets, store_pod_selection};
use crate::Result;
use clap::Args;
use lium_api::LiumApiClient;

#[derive(Args)]
pub struct PsArgs {
    /// Show detailed information for specific pods (by HUID, index, or "all")
    #[arg(value_name = "POD_TARGET")]
    pub targets: Vec<String>,

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

pub async fn handle(args: PsArgs, config: &Config) -> Result<()> {
    let client = LiumApiClient::from_config(config)?;

    // If specific targets are provided, show details for those pods
    if !args.targets.is_empty() {
        let resolved_pods = resolve_pod_targets(&client, &args.targets).await?;

        for (pod, identifier) in resolved_pods {
            println!("Pod details for {} ({}):", pod.huid, identifier);
            display_pod_details(&pod);
            println!();
        }
        return Ok(());
    }

    // Fetch all pods from API for listing
    let mut pods = client.get_pods().await?;

    if pods.is_empty() {
        println!("No pods found. Use 'lium up' to create a pod.");
        return Ok(());
    }

    // Apply filters
    if !args.all {
        // Default: show only running and starting pods
        pods.retain(|pod| {
            matches!(
                pod.status.to_lowercase().as_str(),
                "running" | "starting" | "active" | "ready"
            )
        });
    }

    if let Some(status_filter) = &args.status {
        let status_lower = status_filter.to_lowercase();
        pods.retain(|pod| pod.status.to_lowercase() == status_lower);
    }

    if let Some(gpu_filter) = &args.gpu {
        let gpu_upper = gpu_filter.to_uppercase();
        pods.retain(|pod| {
            pod.executor
                .get("gpu_type")
                .and_then(|v| v.as_str())
                .map(|gpu| gpu.to_uppercase().contains(&gpu_upper))
                .unwrap_or(false)
        });
    }

    if pods.is_empty() {
        println!("No pods found matching your criteria.");
        return Ok(());
    }

    // Store pod selection for index-based references in other commands
    store_pod_selection(&pods)?;

    // Display pods table
    display_pods_table(&pods);

    // Show summary
    let running_count = pods
        .iter()
        .filter(|p| matches!(p.status.to_lowercase().as_str(), "running" | "active"))
        .count();
    let starting_count = pods
        .iter()
        .filter(|p| matches!(p.status.to_lowercase().as_str(), "starting" | "creating"))
        .count();
    let stopped_count = pods
        .iter()
        .filter(|p| matches!(p.status.to_lowercase().as_str(), "stopped" | "terminated"))
        .count();

    println!();
    println!(
        "Summary: {} running, {} starting, {} stopped",
        running_count, starting_count, stopped_count
    );

    // Calculate total cost for active pods
    let total_cost: f64 = pods
        .iter()
        .filter(|p| {
            matches!(
                p.status.to_lowercase().as_str(),
                "running" | "starting" | "active"
            )
        })
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

    // Usage hint
    println!();
    println!("Use 'lium ps <pod_target>' for detailed info, or 'lium exec <pod_target> <command>' to run commands.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use lium_core::PodInfo;
    use std::collections::HashMap;

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
