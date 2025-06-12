use crate::{
    config::Config,
    display::{print_error, print_info, print_success, print_warning},
    helpers::resolve_pod_targets,
    CliError, Result,
};
use lium_api::LiumApiClient;
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Parse SSH command to extract host, port, and user
/// This is a local implementation that handles empty strings and edge cases
fn parse_ssh_command(ssh_cmd: &str) -> Result<(String, u16, String)> {
    // Handle empty or whitespace-only strings
    if ssh_cmd.trim().is_empty() {
        return Err(CliError::InvalidInput("SSH command is empty".to_string()));
    }

    // Try using the utils parser first
    match lium_utils::parse_ssh_command(ssh_cmd) {
        Ok(result) => Ok(result),
        Err(_) => {
            // Fallback parsing for edge cases
            // Try to extract user@host pattern
            if let Some(at_pos) = ssh_cmd.find('@') {
                let user_start = ssh_cmd[..at_pos].rfind(' ').map(|p| p + 1).unwrap_or(0);
                let user = ssh_cmd[user_start..at_pos].trim();

                let host_start = at_pos + 1;
                let mut host_end = ssh_cmd.len();
                let mut port = 22u16;

                // Look for port
                if let Some(space_pos) = ssh_cmd[host_start..].find(' ') {
                    host_end = host_start + space_pos;

                    if ssh_cmd[host_end..].contains("-p ") {
                        if let Some(port_flag_pos) = ssh_cmd[host_end..].find("-p ") {
                            let port_start = host_end + port_flag_pos + 3;
                            if let Some(port_str) = ssh_cmd[port_start..].split_whitespace().next()
                            {
                                port = port_str.parse().unwrap_or(22);
                            }
                        }
                    }
                }

                let host = ssh_cmd[host_start..host_end].trim();

                if !user.is_empty() && !host.is_empty() {
                    return Ok((host.to_string(), port, user.to_string()));
                }
            }

            Err(CliError::InvalidInput(format!(
                "Failed to parse SSH command: '{}'",
                ssh_cmd
            )))
        }
    }
}

/// Handle SCP command to copy files to/from pods
pub async fn handle(
    source: String,
    destination: String,
    coldkey: Option<String>,
    hotkey: Option<String>,
    config: &Config,
) -> Result<()> {
    // Check if scp is available
    if !command_exists("scp") {
        return Err(CliError::InvalidInput(
            "Error: 'scp' command not found in your system PATH. \
             Please install an SCP client (usually part of OpenSSH)."
                .to_string(),
        ));
    }

    let api_client = LiumApiClient::from_config(config)?;

    // Parse the source and destination to determine direction
    let (is_upload, pod_targets_str, local_path, remote_path) =
        parse_scp_args(&source, &destination)?;

    debug!(
        "SCP direction: {}",
        if is_upload { "upload" } else { "download" }
    );
    debug!("Pod targets: {}", pod_targets_str);
    debug!("Local path: {}", local_path);
    debug!("Remote path: {}", remote_path);

    // Resolve pod targets
    let pod_targets: Vec<String> = pod_targets_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let resolved_pods = resolve_pod_targets(&api_client, &pod_targets).await?;

    if resolved_pods.is_empty() {
        return Err(CliError::InvalidInput("No valid pods found.".to_string()));
    }

    // Debug: Check what's in the resolved pods
    debug!("Number of resolved pods: {}", resolved_pods.len());
    for (pod, original_ref) in &resolved_pods {
        debug!(
            "Pod {} ({}): ssh_cmd = {:?}",
            pod.huid, original_ref, pod.ssh_cmd
        );
    }

    // Get SSH private key path
    let private_key_path = config.get_ssh_private_key_path()?;
    if !private_key_path.exists() {
        return Err(CliError::InvalidInput(format!(
            "SSH private key not found at '{}'. Configure it with: lium config set ssh.key_path /path/to/key",
            private_key_path.display()
        )));
    }

    // Collect files to copy
    let mut files_to_copy: Vec<(PathBuf, String)> = Vec::new();

    if is_upload {
        // Handle wallet files if specified
        if let Some(coldkey_name) = &coldkey {
            let wallet_path = dirs::home_dir()
                .ok_or_else(|| CliError::InvalidInput("Cannot find home directory".to_string()))?
                .join(".bittensor")
                .join("wallets")
                .join(coldkey_name);

            // Add coldkeypub.txt
            let coldkeypub_path = wallet_path.join("coldkeypub.txt");
            if !coldkeypub_path.exists() {
                return Err(CliError::InvalidInput(format!(
                    "Coldkey public key not found at '{}'",
                    coldkeypub_path.display()
                )));
            }
            files_to_copy.push((
                coldkeypub_path,
                format!("~/.bittensor/wallets/{}/coldkeypub.txt", coldkey_name),
            ));

            // Add hotkey if specified
            if let Some(hotkey_name) = &hotkey {
                let hotkey_path = wallet_path.join("hotkeys").join(hotkey_name);
                if !hotkey_path.exists() {
                    return Err(CliError::InvalidInput(format!(
                        "Hotkey not found at '{}'",
                        hotkey_path.display()
                    )));
                }
                files_to_copy.push((
                    hotkey_path,
                    format!(
                        "~/.bittensor/wallets/{}/hotkeys/{}",
                        coldkey_name, hotkey_name
                    ),
                ));
            }
        } else if hotkey.is_some() {
            return Err(CliError::InvalidInput(
                "--hotkey requires --coldkey".to_string(),
            ));
        }

        // Add the main file if no wallet files or in addition to wallet files
        if files_to_copy.is_empty() || coldkey.is_some() {
            let local_file = PathBuf::from(&local_path);
            if !local_file.exists() {
                return Err(CliError::InvalidInput(format!(
                    "Local file '{}' not found",
                    local_path
                )));
            }

            // If remote path is empty, use home directory with filename
            let remote_dest = if remote_path.is_empty() {
                format!("~/{}", local_file.file_name().unwrap().to_string_lossy())
            } else {
                remote_path.clone()
            };

            files_to_copy.push((local_file, remote_dest));
        }
    }

    // Show what we're about to copy
    if is_upload {
        print_info(&format!(
            "📁 Copying {} file(s) to {} pod(s):",
            files_to_copy.len(),
            resolved_pods.len()
        ));
        for (local_file, remote_dest) in &files_to_copy {
            println!("  - {} → {}", local_file.display(), remote_dest);
        }
    } else {
        print_info(&format!(
            "📁 Downloading from {} pod(s) to {}",
            resolved_pods.len(),
            local_path
        ));
    }

    for (pod, original_ref) in &resolved_pods {
        println!("  - Target: {} ({})", pod.huid, original_ref);
    }
    println!();

    // Execute SCP for each pod
    let mut success_count = 0;
    let mut failure_count = 0;

    for (pod, original_ref) in resolved_pods {
        // Get SSH connection details - using the same approach as ssh.rs
        debug!("Processing pod {} ({})", pod.huid, original_ref);
        debug!("pod.ssh_cmd is Some: {}", pod.ssh_cmd.is_some());

        let ssh_cmd = pod.ssh_cmd.as_ref().ok_or_else(|| {
            CliError::InvalidInput(format!("Pod {} has no SSH connection info", pod.huid))
        })?;

        debug!("SSH command value: '{}'", ssh_cmd);
        debug!("SSH command length: {}", ssh_cmd.len());
        debug!("SSH command bytes: {:?}", ssh_cmd.as_bytes());

        debug!("SSH command from pod: '{}'", ssh_cmd);

        // Check if SSH command is empty
        if ssh_cmd.trim().is_empty() {
            error!("SSH command is empty for pod {}", pod.huid);

            // Try to construct SSH command from other pod fields if available
            // This is a workaround for when ssh_cmd is not properly populated
            // You might need to adjust this based on your Pod struct
            return Err(CliError::InvalidInput(format!(
                "Pod {} has empty SSH connection info. Please check if the pod is properly configured.",
                pod.huid
            )));
        }

        // Use the local parse_ssh_command that handles edge cases
        let (host, port, user) = parse_ssh_command(ssh_cmd)?;

        debug!(
            "Parsed SSH details - Host: {}, Port: {}, User: {}",
            host, port, user
        );

        // Workaround: If port is 22 and SSH command contains "-p", extract the actual port
        // (Similar to exec.rs workaround)
        let mut final_port = port;
        if port == 22 && ssh_cmd.contains("-p ") {
            if let Some(port_start) = ssh_cmd.find("-p ") {
                let port_str = &ssh_cmd[port_start + 3..];
                if let Some(port_end) = port_str.find(|c: char| !c.is_numeric()) {
                    if let Ok(parsed_port) = port_str[..port_end].parse::<u16>() {
                        debug!(
                            "Parser returned port 22, but found -p {} in command. Using port {}.",
                            parsed_port, parsed_port
                        );
                        final_port = parsed_port;
                    }
                } else if let Ok(parsed_port) = port_str.trim().parse::<u16>() {
                    debug!(
                        "Parser returned port 22, but found -p {} in command. Using port {}.",
                        parsed_port, parsed_port
                    );
                    final_port = parsed_port;
                }
            }
        }

        if is_upload {
            // Upload files to this pod
            let mut pod_success = true;

            for (local_file, remote_dest) in &files_to_copy {
                // Create remote directory if needed
                let remote_dir = if remote_dest.contains('/') {
                    Path::new(remote_dest)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                } else {
                    None
                };

                if let Some(dir) = remote_dir {
                    if !dir.is_empty() && dir != "~" && dir != "." {
                        let mkdir_cmd = Command::new("ssh")
                            .arg("-i")
                            .arg(&private_key_path)
                            .arg("-p")
                            .arg(final_port.to_string())
                            .arg("-o")
                            .arg("StrictHostKeyChecking=no")
                            .arg("-o")
                            .arg("UserKnownHostsFile=/dev/null")
                            .arg(format!("{}@{}", user, host))
                            .arg(format!("mkdir -p {}", shell_quote(&dir)))
                            .output();

                        if let Err(e) = mkdir_cmd {
                            warn!("Failed to create remote directory: {}", e);
                        }
                    }
                }

                // Execute SCP
                print_info(&format!(
                    "📤 Copying {} → {} ({}):{}",
                    local_file.file_name().unwrap().to_string_lossy(),
                    pod.huid,
                    original_ref,
                    remote_dest
                ));

                let mut scp_cmd = Command::new("scp");
                scp_cmd
                    .arg("-i")
                    .arg(&private_key_path)
                    .arg("-P")
                    .arg(final_port.to_string())
                    .arg("-o")
                    .arg("StrictHostKeyChecking=no")
                    .arg("-o")
                    .arg("UserKnownHostsFile=/dev/null")
                    .arg(local_file)
                    .arg(format!("{}@{}:{}", user, host, remote_dest))
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                match scp_cmd.output() {
                    Ok(output) => {
                        if output.status.success() {
                            print_success("  ✅ Done");
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            print_error(&format!("  ❌ Failed: {}", stderr.trim()));
                            pod_success = false;
                            break;
                        }
                    }
                    Err(e) => {
                        print_error(&format!("  ❌ Failed to execute SCP: {}", e));
                        pod_success = false;
                        break;
                    }
                }
            }

            if pod_success {
                success_count += 1;
            } else {
                failure_count += 1;
            }
        } else {
            // Download from pod
            print_info(&format!(
                "📥 Downloading {} ({}):{} → {}",
                pod.huid, original_ref, remote_path, local_path
            ));

            let mut scp_cmd = Command::new("scp");
            scp_cmd
                .arg("-i")
                .arg(&private_key_path)
                .arg("-P")
                .arg(final_port.to_string())
                .arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg("-o")
                .arg("UserKnownHostsFile=/dev/null")
                .arg(format!("{}@{}:{}", user, host, remote_path))
                .arg(&local_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            match scp_cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        print_success("  ✅ Done");
                        success_count += 1;
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        print_error(&format!("  ❌ Failed: {}", stderr.trim()));
                        failure_count += 1;
                    }
                }
                Err(e) => {
                    print_error(&format!("  ❌ Failed to execute SCP: {}", e));
                    failure_count += 1;
                }
            }
        }
    }

    // Summary
    println!();
    print_info(&format!(
        "📊 Copy Summary: {} pods successful, {} pods failed",
        success_count, failure_count
    ));

    Ok(())
}

/// Check if a command exists in PATH
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Parse SCP arguments to determine direction and paths
fn parse_scp_args(source: &str, destination: &str) -> Result<(bool, String, String, String)> {
    // Check if source or destination contains pod reference (no colon or colon at end means pod reference)
    // Format: pod_target:path or just pod_target

    if source.contains(':') && !source.ends_with(':') {
        // Download: pod:remote_path local_path
        let parts: Vec<&str> = source.splitn(2, ':').collect();
        if parts.len() == 2 {
            return Ok((
                false,
                parts[0].to_string(),
                destination.to_string(),
                parts[1].to_string(),
            ));
        }
    } else if destination.contains(':') && !destination.ends_with(':') {
        // Upload: local_path pod:remote_path
        let parts: Vec<&str> = destination.splitn(2, ':').collect();
        if parts.len() == 2 {
            return Ok((
                true,
                parts[0].to_string(),
                source.to_string(),
                parts[1].to_string(),
            ));
        }
    } else {
        // Upload: pod_target local_file [remote_path]
        // In this case, source is pod_target and destination is local file
        // Remote path will be determined later
        return Ok((
            true,
            source.to_string(),
            destination.to_string(),
            String::new(),
        ));
    }

    Err(CliError::InvalidInput(
        "Invalid SCP syntax. Use: lium scp <pod> <local_file> [<remote_path>] or lium scp <pod>:<remote_file> <local_path>".to_string()
    ))
}

/// Quote a string for shell execution, preserving ~ for home expansion
fn shell_quote(s: &str) -> String {
    if s.starts_with('~') {
        s.to_string()
    } else {
        // Simple quoting - in production, use a proper shell escaping library
        format!("'{}'", s.replace('\'', "'\"'\"'"))
    }
}
