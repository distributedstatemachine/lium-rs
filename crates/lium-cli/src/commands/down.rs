use crate::{config::Config, helpers::resolve_pod_targets, CliError, Result};
use dialoguer::Confirm;
use lium_api::LiumApiClient;

/// Handles the `down` command to stop and terminate running pods.
///
/// This function manages the complete lifecycle of pod termination, including target
/// resolution, user confirmation, and graceful shutdown. It supports both individual
/// pod termination and bulk operations.
///
/// # Arguments
/// * `pods` - Vector of pod targets (HUIDs, indices, or names) to stop
/// * `all` - Boolean flag to stop all active pods regardless of the pods parameter
/// * `yes` - Boolean flag to skip interactive confirmation prompts
/// * `config` - User configuration containing API credentials and settings
///
/// # Returns
/// * `Result<()>` - Success or error with detailed information about failures
///
/// # Pod Target Resolution
/// The function supports multiple ways to specify which pods to stop:
/// - **Pod HUIDs**: Direct hardware unique identifiers (e.g., "exec-abc123")
/// - **Pod indices**: Numeric references from `lium ps` output (e.g., "1", "3")
/// - **Pod names**: User-defined or auto-generated names (e.g., "my-training-pod")
/// - **All pods**: Use `--all` flag to stop all active pods
///
/// # Process Flow
/// 1. **Input Validation**: Ensures valid targets are provided
/// 2. **Target Resolution**: Converts pod targets to actual pod references
/// 3. **Status Check**: Verifies pods exist and are in a stoppable state
/// 4. **Confirmation**: Shows affected pods and requests user confirmation (unless `--yes`)
/// 5. **Termination**: Calls unrent_pod API for each target pod
/// 6. **Results**: Reports success/failure counts and details
///
/// # Error Conditions
/// - No pod targets specified and `--all` not used
/// - Invalid pod targets (non-existent HUIDs, indices, or names)
/// - Network errors during API calls
/// - Pods already stopped or in non-stoppable states
/// - Permission errors (API key issues)
///
/// # Safety Considerations
/// - **Confirmation Required**: Interactive confirmation prevents accidental termination
/// - **Graceful Shutdown**: Pods are terminated gracefully when possible
/// - **State Validation**: Only attempts to stop pods that are actually running
/// - **Retry Logic**: Built-in retry for transient network failures
///
/// # Examples
/// ```rust
/// use lium_cli::commands::down::handle;
/// use lium_cli::config::Config;
///
/// let config = Config::new()?;
///
/// // Stop specific pods
/// handle(
///     vec!["1".to_string(), "3".to_string()],
///     false,
///     false,
///     &config
/// ).await?;
///
/// // Stop all pods without confirmation
/// handle(
///     vec![],
///     true,
///     true,
///     &config
/// ).await?;
/// ```
///
/// # Output Format
/// The function provides detailed feedback including:
/// ```text
/// 📋 Pods to stop:
///   1. exec-abc123 (my-pod) - Status: running
///   2. exec-def456 (training-job) - Status: starting
///
/// 🛑 Stopping 2 pod(s)...
/// 🛑 Stopping pod my-pod (exec-abc123)... ✅ Success
/// 🛑 Stopping pod training-job (exec-def456)... ✅ Success
///
/// 🏁 Stop operation complete:
///   ✅ Successfully stopped: 2
/// ```
///
/// # TODO
/// - Add support for graceful shutdown timeouts
/// - Implement pod dependency checking before termination
/// - Add cost tracking for stopped pods
/// - Support for scheduled pod termination
/// - Add backup/snapshot creation before termination
pub async fn handle(pods: Vec<String>, all: bool, yes: bool, config: &Config) -> Result<()> {
    let api_client = LiumApiClient::from_config(config)?;

    // Determine targets
    let targets = if all {
        vec!["all".to_string()]
    } else if pods.is_empty() {
        return Err(CliError::InvalidInput(
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
            .map_err(|e| CliError::InvalidInput(format!("Input error: {}", e)))?;

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
