use crate::config::Config;
use crate::errors::Result;
use crate::helpers::{parse_ssh_command, resolve_pod_targets};
use std::process::Command;

/// Handle the rsync command for directory synchronization
pub async fn handle_rsync(
    source: String,
    destination: String,
    options: Vec<String>,
    config: &Config,
) -> Result<()> {
    // Parse source and destination to identify pod targets
    let (source_is_remote, source_pod, source_path) = parse_rsync_path(&source)?;
    let (dest_is_remote, dest_pod, dest_path) = parse_rsync_path(&destination)?;

    // Validate that exactly one side is remote
    if source_is_remote && dest_is_remote {
        return Err(crate::errors::LiumError::InvalidInput(
            "Remote-to-remote rsync is not supported. One path must be local.".to_string(),
        ));
    }

    if !source_is_remote && !dest_is_remote {
        return Err(crate::errors::LiumError::InvalidInput(
            "Both paths are local. At least one must be a pod target (pod_target:path)."
                .to_string(),
        ));
    }

    let api_client = crate::api::LiumApiClient::from_config()?;

    // Determine which side is remote and resolve the pod
    let (pod_target, is_upload) = if source_is_remote {
        (source_pod.unwrap(), false) // Download from pod
    } else {
        (dest_pod.unwrap(), true) // Upload to pod
    };

    // Resolve pod target
    let resolved_pods = resolve_pod_targets(&api_client, &[pod_target.clone()]).await?;

    if resolved_pods.is_empty() {
        return Err(crate::errors::LiumError::InvalidInput(format!(
            "Pod not found: {}",
            pod_target
        )));
    }

    if resolved_pods.len() > 1 {
        return Err(crate::errors::LiumError::InvalidInput(
            "Rsync command requires exactly one pod target".to_string(),
        ));
    }

    let (pod, _) = &resolved_pods[0];

    // Parse SSH details
    let ssh_cmd = pod.ssh_cmd.as_ref().ok_or_else(|| {
        crate::errors::LiumError::InvalidInput(format!(
            "Pod {} has no SSH connection info",
            pod.huid
        ))
    })?;

    let (host, port, user) = parse_ssh_command(ssh_cmd)?;

    // Get SSH private key path
    let private_key_path = config.get_ssh_private_key_path()?;

    // Construct rsync command
    let mut rsync_args = vec!["-avz".to_string()]; // Default options: archive, verbose, compress

    // Add user-provided options
    rsync_args.extend(options);

    // Add SSH options
    let ssh_options = format!(
        "ssh -i {} -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null{}",
        private_key_path.to_string_lossy(),
        if port != 22 {
            format!(" -p {}", port)
        } else {
            String::new()
        }
    );
    rsync_args.extend_from_slice(&["-e".to_string(), ssh_options]);

    // Add source and destination
    if is_upload {
        // Upload: local -> remote
        rsync_args.push(source_path.unwrap_or(source));
        rsync_args.push(format!(
            "{}@{}:{}",
            user,
            host,
            dest_path.unwrap_or_else(|| "~".to_string())
        ));
        println!("📤 Uploading to pod {} ({})...", pod.huid, pod.name);
    } else {
        // Download: remote -> local
        rsync_args.push(format!(
            "{}@{}:{}",
            user,
            host,
            source_path.unwrap_or_else(|| "~".to_string())
        ));
        rsync_args.push(dest_path.unwrap_or(destination));
        println!("📥 Downloading from pod {} ({})...", pod.huid, pod.name);
    }

    // Execute rsync command
    println!("🔄 Running rsync...");
    let status = Command::new("rsync")
        .args(&rsync_args)
        .status()
        .map_err(crate::errors::LiumError::Io)?;

    if status.success() {
        println!("✅ Rsync completed successfully");
    } else {
        return Err(crate::errors::LiumError::OperationFailed(format!(
            "Rsync failed with exit code: {:?}",
            status.code()
        )));
    }

    Ok(())
}

/// Parse rsync path to determine if it's remote (pod_target:path format)
/// Returns (is_remote, pod_target_option, path_option)
fn parse_rsync_path(path: &str) -> Result<(bool, Option<String>, Option<String>)> {
    if let Some((pod_target, remote_path)) = path.split_once(':') {
        // Remote path format: pod_target:path
        Ok((
            true,
            Some(pod_target.to_string()),
            Some(remote_path.to_string()),
        ))
    } else {
        // Local path
        Ok((false, None, Some(path.to_string())))
    }
}
