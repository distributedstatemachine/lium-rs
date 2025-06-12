use crate::{
    config::Config,
    display::{print_error, print_info, print_success, print_warning},
    helpers::{parse_ssh_command, resolve_pod_targets},
    CliError, Result,
};
use lium_api::LiumApiClient;
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Handle the rsync command for directory synchronization
pub async fn handle(
    source: String,
    destination: String,
    mut options: Vec<String>,
    config: &Config,
) -> Result<()> {
    // Check if rsync is available
    if !command_exists("rsync") {
        return Err(CliError::InvalidInput(
            "Error: 'rsync' command not found in your system PATH. \
             Please install rsync (available on most Unix-like systems)."
                .to_string(),
        ));
    }

    let api_client = LiumApiClient::from_config(config)?;

    // Parse source and destination to identify pod targets
    let (source_pods, source_path) = parse_rsync_path(&source)?;
    let (dest_pods, dest_path) = parse_rsync_path(&destination)?;

    // Validate that exactly one side is remote
    if source_pods.is_some() && dest_pods.is_some() {
        return Err(CliError::InvalidInput(
            "Remote-to-remote sync between pods not yet supported. \
             Use a local intermediate directory or run sync in two steps."
                .to_string(),
        ));
    }

    if source_pods.is_none() && dest_pods.is_none() {
        return Err(CliError::InvalidInput(
            "At least one path must be remote (pod_targets:path format).".to_string(),
        ));
    }

    // Determine operation mode
    let (pod_targets_str, remote_path, local_path, is_upload) = if let Some(pods) = source_pods {
        // Download: remote to local
        (pods, source_path, destination.clone(), false)
    } else {
        // Upload: local to remote
        (dest_pods.unwrap(), dest_path, source.clone(), true)
    };

    // Validate local path
    let local_path_obj = Path::new(&local_path);
    if is_upload && !local_path_obj.exists() {
        return Err(CliError::InvalidInput(format!(
            "Local source path '{}' does not exist",
            local_path
        )));
    }

    // Parse pod targets (support comma-separated list)
    let pod_targets: Vec<String> = pod_targets_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Resolve pod targets
    let resolved_pods = resolve_pod_targets(&api_client, &pod_targets).await?;

    if resolved_pods.is_empty() {
        return Err(CliError::InvalidInput("No valid pods found.".to_string()));
    }

    // Get SSH private key path
    let private_key_path = config.get_ssh_private_key_path()?;
    if !private_key_path.exists() {
        return Err(CliError::InvalidInput(format!(
            "SSH private key not found at '{}'. Configure it with: lium config set ssh.key_path /path/to/key",
            private_key_path.display()
        )));
    }

    // Process rsync options
    let mut rsync_args = Vec::new();
    let mut has_archive = false;
    let mut has_verbose = false;
    let mut has_quiet = false;
    let mut exclude_patterns = Vec::new();
    let mut delete_flag = false;
    let mut dry_run = false;
    let mut progress = false;

    // Parse options
    let mut i = 0;
    while i < options.len() {
        match options[i].as_str() {
            "-a" | "--archive" => has_archive = true,
            "-v" | "--verbose" => has_verbose = true,
            "-q" | "--quiet" => has_quiet = true,
            "-z" | "--compress" => rsync_args.push("-z".to_string()),
            "-n" | "--dry-run" => {
                dry_run = true;
                rsync_args.push("--dry-run".to_string());
            }
            "--delete" => {
                delete_flag = true;
                rsync_args.push("--delete".to_string());
            }
            "--progress" => {
                progress = true;
                rsync_args.push("--progress".to_string());
            }
            "--exclude" => {
                if i + 1 < options.len() {
                    exclude_patterns.push(options[i + 1].clone());
                    rsync_args.push("--exclude".to_string());
                    rsync_args.push(options[i + 1].clone());
                    i += 1;
                }
            }
            opt if opt.starts_with("--exclude=") => {
                let pattern = opt.trim_start_matches("--exclude=");
                exclude_patterns.push(pattern.to_string());
                rsync_args.push(opt.to_string());
            }
            _ => rsync_args.push(options[i].clone()),
        }
        i += 1;
    }

    // Add default options if not specified
    if !has_archive {
        rsync_args.insert(0, "-a".to_string()); // Archive mode by default
    }
    if !has_verbose && !has_quiet {
        rsync_args.insert(0, "-v".to_string()); // Show files by default
    }

    // Show operation summary
    let operation_desc = if dry_run {
        "DRY RUN of sync"
    } else {
        "Syncing"
    };
    print_info(&format!(
        "🔄 {}: {} → {}",
        operation_desc, source, destination
    ));
    print_info(&format!(
        "   Mode: {}",
        if is_upload {
            "Local to Remote"
        } else {
            "Remote to Local"
        }
    ));
    print_info(&format!("   Pods: {} target(s)", resolved_pods.len()));

    if !exclude_patterns.is_empty() {
        print_info(&format!("   Excluding: {}", exclude_patterns.join(", ")));
    }
    if delete_flag {
        print_warning("   ⚠️  Delete mode enabled - will remove extraneous files from destination");
    }

    for (pod, original_ref) in &resolved_pods {
        println!("   - Target: {} ({})", pod.huid, original_ref);
    }
    println!();

    // Execute rsync for each pod
    let mut success_count = 0;
    let mut failure_count = 0;

    for (pod, original_ref) in &resolved_pods {
        // Get SSH connection details
        let ssh_cmd = pod.ssh_cmd.as_ref().ok_or_else(|| {
            CliError::InvalidInput(format!("Pod {} has no SSH connection info", pod.huid))
        })?;

        // Check if SSH command is empty
        if ssh_cmd.trim().is_empty() {
            print_warning(&format!(
                "⚠️  Pod '{}' ({}) has empty SSH connection info, skipping...",
                pod.huid, original_ref
            ));
            failure_count += 1;
            continue;
        }

        // Parse SSH command
        let (host, port, user) = parse_ssh_command(ssh_cmd)?;

        // Check if rsync is installed on the remote pod (for upload) or source pod (for download)
        if is_upload || (!is_upload && resolved_pods.len() == 1) {
            debug!("Checking if rsync is installed on remote pod...");

            let check_rsync_cmd = Command::new("ssh")
                .arg("-i")
                .arg(&private_key_path)
                .arg("-p")
                .arg(port.to_string())
                .arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg("-o")
                .arg("UserKnownHostsFile=/dev/null")
                .arg(format!("{}@{}", user, host))
                .arg("which rsync")
                .output();

            let rsync_installed = check_rsync_cmd
                .map(|output| output.status.success())
                .unwrap_or(false);

            if !rsync_installed {
                print_warning(&format!(
                    "⚠️  rsync not found on pod '{}' ({}), attempting to install...",
                    pod.huid, original_ref
                ));

                // Try to detect the package manager and install rsync
                let install_commands = vec![
                    // Try apt-get first (Debian/Ubuntu)
                    "apt-get update > /dev/null 2>&1 && apt-get install -y rsync > /dev/null 2>&1",
                    // Try yum (CentOS/RHEL)
                    "yum install -y rsync > /dev/null 2>&1",
                    // Try apk (Alpine)
                    "apk add --no-cache rsync > /dev/null 2>&1",
                    // Try dnf (Fedora)
                    "dnf install -y rsync > /dev/null 2>&1",
                ];

                let mut install_success = false;
                for install_cmd in &install_commands {
                    print_info(&format!("  📦 Trying to install rsync..."));

                    let install_result = Command::new("ssh")
                        .arg("-i")
                        .arg(&private_key_path)
                        .arg("-p")
                        .arg(port.to_string())
                        .arg("-o")
                        .arg("StrictHostKeyChecking=no")
                        .arg("-o")
                        .arg("UserKnownHostsFile=/dev/null")
                        .arg(format!("{}@{}", user, host))
                        .arg(install_cmd)
                        .output();

                    if let Ok(output) = install_result {
                        if output.status.success() {
                            // Verify installation
                            let verify_cmd = Command::new("ssh")
                                .arg("-i")
                                .arg(&private_key_path)
                                .arg("-p")
                                .arg(port.to_string())
                                .arg("-o")
                                .arg("StrictHostKeyChecking=no")
                                .arg("-o")
                                .arg("UserKnownHostsFile=/dev/null")
                                .arg(format!("{}@{}", user, host))
                                .arg("which rsync")
                                .output();

                            if verify_cmd.map(|o| o.status.success()).unwrap_or(false) {
                                print_success("  ✅ rsync installed successfully");
                                install_success = true;
                                break;
                            }
                        }
                    }
                }

                if !install_success {
                    return Err(CliError::InvalidInput(format!(
                        "Failed to install rsync on pod '{}' ({}). Please install it manually with:\n  \
                        lium exec {} 'apt-get update && apt-get install -y rsync'",
                        pod.huid, original_ref, original_ref
                    )));
                }
            } else {
                debug!("rsync is already installed on the remote pod");
            }
        }

        // Build SSH options for rsync
        let ssh_options = format!(
            "ssh -i {} -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -p {}",
            shell_quote(&private_key_path.to_string_lossy()),
            port
        );

        // Build complete rsync command
        let mut rsync_cmd = Command::new("rsync");
        rsync_cmd.args(&rsync_args);
        rsync_cmd.arg("-e");
        rsync_cmd.arg(&ssh_options);

        if is_upload {
            // Create remote directory if needed (unless in dry-run mode)
            let remote_dir = Path::new(&remote_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string());

            if let Some(dir) = remote_dir {
                if !dir.is_empty() && dir != "~" && dir != "." && !dry_run {
                    print_info(&format!("  📁 Creating directory structure: {}", dir));

                    let mkdir_result = Command::new("ssh")
                        .arg("-i")
                        .arg(&private_key_path)
                        .arg("-p")
                        .arg(port.to_string())
                        .arg("-o")
                        .arg("StrictHostKeyChecking=no")
                        .arg("-o")
                        .arg("UserKnownHostsFile=/dev/null")
                        .arg(format!("{}@{}", user, host))
                        .arg(format!("mkdir -p {}", shell_quote(&dir)))
                        .output();

                    if let Err(e) = mkdir_result {
                        print_warning(&format!(
                            "⚠️  Failed to create directory '{}' on '{}' ({}): {}",
                            dir, pod.huid, original_ref, e
                        ));
                    }
                } else if !dir.is_empty() && dir != "~" && dir != "." && dry_run {
                    print_info(&format!("  📁 Would create directory structure: {}", dir));
                }
            }

            print_info(&format!(
                "🔄 Syncing with '{}' ({})...",
                pod.huid, original_ref
            ));
            rsync_cmd.arg(&local_path);
            rsync_cmd.arg(format!("{}@{}:{}", user, host, remote_path));
        } else {
            // Download from pod
            print_info(&format!(
                "🔄 Syncing from '{}' ({})...",
                pod.huid, original_ref
            ));
            rsync_cmd.arg(format!("{}@{}:{}", user, host, remote_path));
            rsync_cmd.arg(&local_path);
        }

        // Execute rsync with retry logic
        let mut pod_success = false;
        let retry_attempts = 3;

        for attempt in 0..retry_attempts {
            if attempt > 0 {
                print_info(&format!(
                    "  🔄 Retry attempt {}/{}",
                    attempt + 1,
                    retry_attempts
                ));
            }

            let output = if has_quiet {
                rsync_cmd
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
            } else {
                rsync_cmd.status().map(|status| std::process::Output {
                    status,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            };

            match output {
                Ok(result) => {
                    if result.status.success() {
                        print_success("  ✅ Sync completed successfully");
                        pod_success = true;
                        success_count += 1;
                        break;
                    } else {
                        let exit_code = result.status.code().unwrap_or(-1);
                        print_error(&format!("  ❌ Sync failed (exit code {})", exit_code));

                        if !result.stderr.is_empty() {
                            let stderr = String::from_utf8_lossy(&result.stderr);
                            let error_lines: Vec<&str> = stderr.lines().collect();
                            for line in error_lines.iter().take(3) {
                                eprintln!("     {}", line);
                            }
                            if error_lines.len() > 3 {
                                eprintln!("     ... ({} more lines)", error_lines.len() - 3);
                            }
                        }

                        // Don't retry on certain errors
                        if matches!(exit_code, 1..=6 | 10..=14 | 20 | 21) {
                            print_warning(
                                "  ⚠️  Error type suggests retrying won't help, skipping retries",
                            );
                            break;
                        }

                        if attempt < retry_attempts - 1 {
                            print_info("  ⏳ Will retry in a moment...");
                        }
                    }
                }
                Err(e) => {
                    print_error(&format!("  ❌ Failed to execute rsync: {}", e));
                    break;
                }
            }
        }

        if !pod_success {
            failure_count += 1;
        }
    }

    // Summary
    println!();
    if dry_run {
        print_info(&format!(
            "📊 Dry Run Summary: {} pods would sync successfully, {} failed",
            success_count, failure_count
        ));
    } else {
        print_info(&format!(
            "📊 Sync Summary: {} pods synced successfully, {} failed",
            success_count, failure_count
        ));
    }

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

/// Parse rsync path to determine if it's remote (pod_targets:path format)
fn parse_rsync_path(path: &str) -> Result<(Option<String>, String)> {
    // Check if this is a remote path (pod_targets:path format)
    if path.contains(':') && !path.starts_with('/') && !path.starts_with('~') {
        // Split on first colon
        if let Some((pod_part, path_part)) = path.split_once(':') {
            // Check if this looks like a Windows drive letter (single alphabetic character)
            if pod_part.len() == 1 && pod_part.chars().all(|c| c.is_alphabetic()) {
                // This is a Windows path like C:\something, treat as local
                Ok((None, path.to_string()))
            } else {
                // This is a remote path
                Ok((Some(pod_part.to_string()), path_part.to_string()))
            }
        } else {
            Ok((None, path.to_string()))
        }
    } else {
        // Local path
        Ok((None, path.to_string()))
    }
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
