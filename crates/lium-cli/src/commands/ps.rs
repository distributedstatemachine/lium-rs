use crate::config::Config;
use crate::display::{display_pod_details, display_pods_table};
use crate::helpers::{resolve_pod_targets, store_pod_selection};
use crate::Result;
use clap::Args;
use lium_api::LiumApiClient;
use log::{debug, info};

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

    println!("DEBUG: Total pods fetched from API: {}", pods.len());

    // Debug: Print all pod statuses
    for (i, pod) in pods.iter().enumerate() {
        println!(
            "DEBUG: Pod {} - HUID: {}, Status: '{}'",
            i, pod.huid, pod.status
        );
    }

    if pods.is_empty() {
        println!("No pods found. Use 'lium up' to create a pod.");
        return Ok(());
    }

    // Apply filters
    if !args.all {
        println!("DEBUG: Filtering pods (not using --all flag)");
        println!("DEBUG: Looking for statuses: running, starting, active, ready");

        // Debug: Check what pods are being filtered out
        let before_count = pods.len();

        pods.retain(|pod| {
            let status_lower = pod.status.to_lowercase();
            let keep = matches!(
                status_lower.as_str(),
                "running" | "starting" | "active" | "ready"
            );

            if !keep {
                println!(
                    "DEBUG: Filtering out pod {} with status '{}'",
                    pod.huid, pod.status
                );
            }

            keep
        });

        println!(
            "DEBUG: Filtered from {} to {} pods",
            before_count,
            pods.len()
        );
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
        println!("Hint: Use 'lium ps --all' to see all pods regardless of status.");
        return Ok(());
    }

    // Store pod selection for index-based references in other commands
    store_pod_selection(&pods)?;

    // Debug: Analyze pod data structure before display
    println!("\n=== DEBUG: Analyzing Pod Data ===");
    for (i, pod) in pods.iter().take(1).enumerate() {
        // Just analyze first pod
        println!("\nPod {} data analysis:", i);

        // Check what's in executor
        println!("\nExecutor top-level keys:");
        if let Some(obj) = pod.executor.as_object() {
            for (key, value) in obj {
                match value {
                    serde_json::Value::Object(_) => println!("  {} = [object]", key),
                    serde_json::Value::Array(arr) => {
                        println!("  {} = [array with {} items]", key, arr.len())
                    }
                    serde_json::Value::String(s) => println!("  {} = \"{}\"", key, s),
                    serde_json::Value::Number(n) => println!("  {} = {}", key, n),
                    serde_json::Value::Bool(b) => println!("  {} = {}", key, b),
                    serde_json::Value::Null => println!("  {} = null", key),
                }
            }
        }

        // Look for GPU info in different places
        println!("\nSearching for GPU info:");

        // Check executor.gpu_type (what display_pods_table expects)
        if let Some(gpu_type) = pod.executor.get("gpu_type") {
            println!("  Found executor.gpu_type: {:?}", gpu_type);
        } else {
            println!("  executor.gpu_type NOT FOUND");
        }

        // Check executor.gpu_count
        if let Some(gpu_count) = pod.executor.get("gpu_count") {
            println!("  Found executor.gpu_count: {:?}", gpu_count);
        } else {
            println!("  executor.gpu_count NOT FOUND");
        }

        // Check executor.specs
        if let Some(specs) = pod.executor.get("specs") {
            println!("  Found executor.specs");

            // Check specs.gpu
            if let Some(gpu) = specs.get("gpu") {
                println!("    Found specs.gpu:");
                if let Some(count) = gpu.get("count") {
                    println!("      gpu.count = {}", count);
                }
                if let Some(details) = gpu.get("details") {
                    if let Some(arr) = details.as_array() {
                        println!("      gpu.details has {} GPUs", arr.len());
                        if let Some(first) = arr.first() {
                            if let Some(name) = first.get("name") {
                                println!("      First GPU name: {}", name);
                            }
                        }
                    }
                }
            }
        }

        // Check for machine_name
        if let Some(machine_name) = pod.executor.get("machine_name") {
            println!("  Found executor.machine_name: {:?}", machine_name);
        }

        // Check created_at
        println!("\nTiming info:");
        println!("  pod.created_at: {:?}", pod.created_at);
        if let Some(uptime) = pod.executor.get("uptime_in_minutes") {
            println!("  executor.uptime_in_minutes: {:?}", uptime);
        }
    }
    println!("=== END DEBUG ===\n");

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
// TODO: Add pod grouping and ta
