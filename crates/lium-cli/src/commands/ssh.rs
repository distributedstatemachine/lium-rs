use crate::{
    config::Config,
    helpers::{parse_ssh_command, resolve_pod_targets},
    CliError, Result,
};
use lium_api::LiumApiClient;
use std::process::Command;

/// Handle the ssh command for interactive SSH sessions
pub async fn handle(pod_target: String, config: &Config) -> Result<()> {
    let api_client = LiumApiClient::from_config(config)?;

    // Resolve single pod target
    let resolved_pods = resolve_pod_targets(&api_client, &[pod_target.clone()]).await?;

    if resolved_pods.is_empty() {
        return Err(CliError::InvalidInput(format!(
            "Pod not found: {}",
            pod_target
        )));
    }

    if resolved_pods.len() > 1 {
        return Err(CliError::InvalidInput(
            "SSH command requires exactly one pod target".to_string(),
        ));
    }

    let (pod, _) = &resolved_pods[0];

    // Parse SSH details
    let ssh_cmd = pod.ssh_cmd.as_ref().ok_or_else(|| {
        CliError::InvalidInput(format!("Pod {} has no SSH connection info", pod.huid))
    })?;

    let (host, port, user) = parse_ssh_command(ssh_cmd)?;

    // Get SSH private key path
    let private_key_path = config.get_ssh_private_key_path()?;

    // Construct SSH command
    let mut ssh_args = vec![
        "-i".to_string(),
        private_key_path.to_string_lossy().to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=no".to_string(),
        "-o".to_string(),
        "UserKnownHostsFile=/dev/null".to_string(),
    ];

    if port != 22 {
        ssh_args.extend_from_slice(&["-p".to_string(), port.to_string()]);
    }

    ssh_args.push(format!("{}@{}", user, host));

    println!("🔗 Connecting to pod {} ({})...", pod.huid, pod.name);
    println!("Host: {}:{}, User: {}", host, port, user);

    // Execute SSH command
    let status = Command::new("ssh")
        .args(&ssh_args)
        .status()
        .map_err(CliError::Io)?;

    if !status.success() {
        return Err(CliError::OperationFailed(format!(
            "SSH connection failed with exit code: {:?}",
            status.code()
        )));
    }

    Ok(())
}
