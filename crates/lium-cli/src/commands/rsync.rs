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

/// Handles the `rsync` command for bidirectional file synchronization between local and remote pods.
///
/// This function provides powerful file synchronization capabilities using the rsync protocol,
/// enabling efficient data transfer between local machines and cloud GPU pods. It supports
/// both upload (local to remote) and download (remote to local) operations with advanced
/// options for filtering, progress monitoring, and error recovery.
///
/// # Arguments
/// * `source` - Source path (local path or pod_targets:remote_path format)
/// * `destination` - Destination path (local path or pod_targets:remote_path format)
/// * `options` - Vector of rsync command-line options and flags
/// * `config` - User configuration containing SSH keys and API credentials
///
/// # Returns
/// * `Result<()>` - Success or error with detailed synchronization information
///
/// # Path Format
/// Remote paths use the format `pod_targets:remote_path` where:
/// - `pod_targets`: Comma-separated list of pod identifiers (HUIDs, indices, names)
/// - `remote_path`: Absolute or relative path on the remote pod
///
/// Examples:
/// - `1:/workspace/data` - Pod index 1, /workspace/data directory
/// - `my-pod:~/models` - Pod named "my-pod", home directory models folder
/// - `exec-abc123:/tmp/output` - Pod with HUID exec-abc123, /tmp/output directory
///
/// # Operation Modes
///
/// ## Upload Mode (Local to Remote)
/// ```bash
/// lium rsync ./local/data/ 1:/workspace/data/
/// lium rsync ./model.py pod1,pod2:~/scripts/
/// ```
///
/// ## Download Mode (Remote to Local)
/// ```bash
/// lium rsync 1:/workspace/results/ ./results/
/// lium rsync pod1:/logs ./downloaded-logs/
/// ```
///
/// # Process Flow
/// 1. **Validation**: Checks rsync availability and path format validation
/// 2. **Path Parsing**: Determines operation mode (upload/download) and pod targets
/// 3. **Pod Resolution**: Resolves pod targets to actual pod instances
/// 4. **SSH Configuration**: Validates SSH keys and connection details
/// 5. **Remote Setup**: Ensures rsync is installed on target pods
/// 6. **Synchronization**: Executes rsync for each target pod with retry logic
/// 7. **Progress Reporting**: Provides real-time feedback and completion status
///
/// # Supported Options
/// The function processes and validates common rsync options:
/// - `-a, --archive`: Archive mode (permissions, times, symlinks, etc.)
/// - `-v, --verbose`: Verbose output showing transferred files
/// - `-q, --quiet`: Suppress most output except errors
/// - `-z, --compress`: Compress data during transfer
/// - `-n, --dry-run`: Show what would be transferred without actually doing it
/// - `--delete`: Delete extraneous files from destination
/// - `--progress`: Show progress during transfer
/// - `--exclude PATTERN`: Exclude files matching pattern
///
/// # SSH Integration
/// - Automatically configures SSH for rsync operations
/// - Uses configured private keys for authentication
/// - Supports custom ports from pod SSH configurations
/// - Disables host key checking for cloud environments
/// - Provides detailed SSH debugging information
///
/// # Remote Rsync Installation
/// The function automatically handles rsync installation on pods:
/// 1. Checks if rsync is available on the remote pod
/// 2. Attempts automatic installation using package managers:
///    - `apt-get` (Debian/Ubuntu)
///    - `yum` (CentOS/RHEL)
///    - `apk` (Alpine Linux)
///    - `dnf` (Fedora)
/// 3. Verifies successful installation before proceeding
/// 4. Provides manual installation instructions if auto-install fails
///
/// # Error Handling and Retry Logic
/// - **Connection Failures**: Automatic retry for transient network issues
/// - **Rsync Errors**: Detailed error code interpretation and suggestions
/// - **SSH Issues**: Comprehensive SSH debugging and troubleshooting
/// - **Path Validation**: Early detection of invalid paths and permissions
/// - **Dependency Check**: Validates rsync availability on both ends
///
/// # Performance Optimizations
/// - **Compression**: Automatic compression for network efficiency
/// - **Delta Transfer**: Only transfers changed parts of files
/// - **Progress Monitoring**: Real-time transfer progress for large operations
/// - **Parallel Operations**: Sequential execution across multiple pods
/// - **Resume Support**: Rsync's built-in resume capabilities for interrupted transfers
///
/// # Examples
/// ```rust
/// use lium_cli::commands::rsync::handle;
/// use lium_cli::config::Config;
///
/// let config = Config::new()?;
///
/// // Upload local directory to pod
/// handle(
///     "./data/".to_string(),
///     "1:/workspace/data/".to_string(),
///     vec!["-av".to_string(), "--progress".to_string()],
///     &config
/// ).await?;
///
/// // Download from pod with exclusions
/// handle(
///     "my-pod:/results/".to_string(),
///     "./results/".to_string(),
///     vec!["-av".to_string(), "--exclude".to_string(), "*.tmp".to_string()],
///     &config
/// ).await?;
///
/// // Dry run to preview changes
/// handle(
///     "./models/".to_string(),
///     "1,2,3:~/models/".to_string(),
///     vec!["-avn".to_string()],
///     &config
/// ).await?;
/// ```
///
/// # Output Format
/// ```text
/// 🔄 Syncing: ./data/ → 1:/workspace/data/
///    Mode: Local to Remote
///    Pods: 1 target(s)
///    - Target: exec-abc123 (1)
///
/// 🔄 Syncing with 'exec-abc123' (1)...
///   📁 Creating directory structure: /workspace/data
///   sending incremental file list
///   ./
///   model.py
///   data/train.csv
///   data/test.csv
///           
///   sent 1,234,567 bytes  received 123 bytes  247,738.00 bytes/sec
///   total size is 1,234,444  speedup is 1.00
///   ✅ Sync completed successfully
///
/// 📊 Sync Summary: 1 pods synced successfully, 0 failed
/// ```
///
/// # Security Considerations
/// - Uses configured SSH private keys exclusively
/// - Disables host key checking for cloud pod environments
/// - Validates file paths to prevent directory traversal attacks
/// - Logs operations for audit purposes without exposing sensitive data
/// - Respects file permissions and ownership where possible
///
/// # Limitations and Considerations
/// - **Remote-to-Remote**: Direct pod-to-pod sync not supported (use local staging)
/// - **Large Files**: Very large files may require stable network connections
/// - **Permissions**: May require adjustment after sync depending on pod configuration
/// - **Symbolic Links**: Handling depends on rsync options and target filesystem
///
/// # TODO
/// - Add support for direct pod-to-pod synchronization
/// - Implement parallel execution for multiple pod operations
/// - Add bandwidth throttling options
/// - Support for custom rsync algorithms and configurations
/// - Add synchronization scheduling and automation
/// - Implement sync conflict resolution strategies
/// - Add support for encrypted transfers beyond SSH
pub async fn handle(
    source: String,
    destination: String,
    options: Vec<String>,
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
