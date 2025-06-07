use crate::config::Config;
use crate::errors::Result;
use crate::helpers::resolve_pod_targets;
use dialoguer::Confirm;

/// Handle the down command for stopping pods
pub async fn handle_down(pods: Vec<String>, all: bool, yes: bool, config: &Config) -> Result<()> {
    let api_client = crate::api::LiumApiClient::from_config()?;

    // Determine targets
    let targets = if all {
        vec!["all".to_string()]
    } else if pods.is_empty() {
        return Err(crate::errors::LiumError::InvalidInput(
            "No pod targets specified. Use --all to stop all pods or specify pod targets."
                .to_string(),
        ));
    } else {
        pods
    };

    // Resolve pod targets
    let resolved_pods = resolve_pod_targets(&api_client, &targets).await?;

    if resolved_pods.is_empty() {
        println!("No active pods found to stop.");
        return Ok(());
    }

    println!("📋 Pods to stop:");
    for (i, (pod, _)) in resolved_pods.iter().enumerate() {
        println!(
            "  {}. {} ({}) - Status: {}",
            i + 1,
            pod.huid,
            pod.name,
            pod.status
        );
    }

    // Confirmation unless -y flag
    if !yes {
        let confirm = Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to stop {} pod(s)?",
                resolved_pods.len()
            ))
            .default(false)
            .interact()
            .map_err(|e| {
                lium_core::errors::LiumError::InvalidInput(format!("Input error: {}", e))
            })?;

        if !confirm {
            println!("❌ Operation cancelled.");
            return Ok(());
        }
    }

    println!("\n🛑 Stopping {} pod(s)...", resolved_pods.len());

    let mut success_count = 0;
    let mut failure_count = 0;

    // Stop each pod
    for (pod, target_name) in resolved_pods {
        let pod_display = if target_name == "all" {
            format!("{} ({})", pod.huid, pod.name)
        } else {
            format!("{} ({})", target_name, pod.huid)
        };

        print!("🛑 Stopping pod {}... ", pod_display);

        // Extract executor ID from pod data
        let executor_id = pod
            .executor
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&pod.id); // Fallback to pod ID if executor ID not found

        match api_client.unrent_pod(executor_id).await {
            Ok(_) => {
                println!("✅ Success");
                success_count += 1;
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
                failure_count += 1;
            }
        }
    }

    println!("\n🏁 Stop operation complete:");
    println!("  ✅ Successfully stopped: {}", success_count);
    if failure_count > 0 {
        println!("  ❌ Failed to stop: {}", failure_count);
    }

    Ok(())
}
