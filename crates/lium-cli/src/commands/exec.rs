use crate::{
    config::Config,
    helpers::{parse_ssh_command, resolve_pod_targets},
    CliError, Result,
};
use lium_utils::execute_remote_command;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Handle the exec command for remote command execution
pub async fn handle(
    pods: Vec<String>,
    command: String,
    script: Option<String>,
    env_vars: Vec<String>,
    config: &Config,
) -> Result<()> {
    let api_client = lium_api::LiumApiClient::from_config(config)?;

    // Resolve pod targets
    let resolved_pods = resolve_pod_targets(&api_client, &pods).await?;

    if resolved_pods.is_empty() {
        return Err(CliError::InvalidInput(
            "No pods found to execute command on".to_string(),
        ));
    }

    // Parse environment variables
    let mut env_map = HashMap::new();
    for env_var in env_vars {
        if let Some((key, value)) = env_var.split_once('=') {
            env_map.insert(key.to_string(), value.to_string());
        } else {
            return Err(CliError::InvalidInput(format!(
                "Invalid environment variable format: {}. Use KEY=VALUE",
                env_var
            )));
        }
    }

    // Determine the command to execute
    let exec_command = if let Some(script_path) = script {
        // Read script file
        let script_content = fs::read_to_string(&script_path).map_err(CliError::Io)?;

        println!("📄 Executing script: {}", script_path);
        script_content
    } else {
        command
    };

    // Get SSH private key path
    let private_key_path = config.get_ssh_private_key_path()?;

    println!(
        "🚀 Executing command on {} pod(s)...\n",
        resolved_pods.len()
    );

    // Execute on each pod
    for (i, (pod, target_name)) in resolved_pods.iter().enumerate() {
        let pod_display = if target_name == "all" {
            format!("Pod {} ({})", i + 1, pod.huid)
        } else {
            format!("Pod {} ({})", target_name, pod.huid)
        };

        println!("🖥️  {}", pod_display);
        println!("{}", "─".repeat(50));

        // Parse SSH details
        let ssh_cmd = pod.ssh_cmd.as_ref().ok_or_else(|| {
            CliError::InvalidInput(format!("Pod {} has no SSH connection info", pod.huid))
        })?;

        let (host, port, user) = parse_ssh_command(ssh_cmd)?;

        // Execute the command
        match execute_remote_command(
            &host,
            port,
            &user,
            &private_key_path,
            &exec_command,
            if env_map.is_empty() {
                None
            } else {
                Some(env_map.clone())
            },
        )
        .await
        {
            Ok((stdout, stderr, exit_code)) => {
                if !stdout.is_empty() {
                    println!("📤 STDOUT:");
                    println!("{}", stdout);
                }

                if !stderr.is_empty() {
                    println!("📥 STDERR:");
                    println!("{}", stderr);
                }

                if exit_code == 0 {
                    println!(
                        "✅ Command completed successfully (exit code: {})",
                        exit_code
                    );
                } else {
                    println!("❌ Command failed (exit code: {})", exit_code);
                }
            }
            Err(e) => {
                println!("❌ Command execution failed: {}", e);
            }
        }

        // Add separator between pods (except for the last one)
        if i < resolved_pods.len() - 1 {
            println!("\n{}\n", "=".repeat(60));
        }
    }

    println!("\n🏁 Command execution complete on all pods.");
    Ok(())
}

/// Execute a script on pods (helper for common patterns)
pub async fn execute_script_on_pods(
    pod_targets: &[String],
    script_name: &str,
    env_vars: &HashMap<String, String>,
    config: &Config,
) -> Result<()> {
    // Look for script in common locations
    let script_paths = vec![
        format!("scripts/{}.sh", script_name),
        format!("./{}.sh", script_name),
        format!("/usr/local/share/lium/scripts/{}.sh", script_name),
    ];

    let mut script_path = None;
    for path in &script_paths {
        if Path::new(path).exists() {
            script_path = Some(path.clone());
            break;
        }
    }

    let script_path = script_path.ok_or_else(|| {
        CliError::InvalidInput(format!(
            "Script '{}' not found in any of: {}",
            script_name,
            script_paths.join(", ")
        ))
    })?;

    // Convert env vars to command line format
    let env_args: Vec<String> = env_vars
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    handle(
        pod_targets.to_vec(),
        String::new(), // Empty command since we're using script
        Some(script_path),
        env_args,
        config,
    )
    .await
}

/// Convenience function for executing Jupyter setup script
pub async fn setup_jupyter_on_pods(
    pod_targets: &[String],
    port: Option<u16>,
    config: &Config,
) -> Result<()> {
    let mut env_vars = HashMap::new();
    if let Some(p) = port {
        env_vars.insert("JUPYTER_PORT".to_string(), p.to_string());
    }

    execute_script_on_pods(pod_targets, "jupyter", &env_vars, config).await
}

/// Convenience function for executing iota setup script  
pub async fn setup_iota_on_pods(
    pod_targets: &[String],
    coldkey_name: Option<String>,
    hotkey_name: Option<String>,
    huggingface_token: Option<String>,
    config: &Config,
) -> Result<()> {
    let mut env_vars = HashMap::new();

    if let Some(cold) = coldkey_name {
        env_vars.insert("COLD".to_string(), cold);
    }

    if let Some(hot) = hotkey_name {
        env_vars.insert("HOT".to_string(), hot);
    }

    if let Some(hf_token) = huggingface_token {
        env_vars.insert("HF".to_string(), hf_token);
    }

    execute_script_on_pods(pod_targets, "iota", &env_vars, config).await
}
