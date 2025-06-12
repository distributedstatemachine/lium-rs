use crate::{
    config::Config,
    helpers::{parse_ssh_command, resolve_pod_targets},
    CliError, Result,
};
use lium_api::LiumApiClient;
use std::process::Command;

/// Handles the `ssh` command to establish interactive SSH connections to pods.
///
/// This function provides a seamless way to connect to cloud GPU pods using SSH,
/// handling connection details, key management, and providing a native terminal
/// experience. It's designed for interactive use where users need direct shell
/// access to their pods.
///
/// # Arguments
/// * `pod_target` - Single pod identifier (HUID, index, or name)
/// * `config` - User configuration containing SSH keys and API credentials
///
/// # Returns
/// * `Result<()>` - Success or error with connection failure details
///
/// # Process Flow
/// 1. **Target Resolution**: Resolves the pod target to a specific pod instance
/// 2. **Connection Validation**: Ensures the pod has SSH connection information
/// 3. **SSH Configuration**: Extracts host, port, and user from pod SSH command
/// 4. **Key Validation**: Verifies SSH private key availability and accessibility
/// 5. **Connection**: Establishes SSH connection with proper configuration
/// 6. **Interactive Session**: Transfers control to the SSH client for user interaction
///
/// # Pod Target Resolution
/// The pod target can be specified as:
/// - **Pod index**: Numeric reference from `lium ps` output (e.g., "1", "3")
/// - **Pod HUID**: Hardware unique identifier (e.g., "exec-abc123")
/// - **Pod name**: User-defined or auto-generated name (e.g., "my-training-pod")
///
/// Unlike other commands, SSH requires exactly one target pod since it establishes
/// an interactive session that cannot be multiplexed across multiple pods.
///
/// # SSH Configuration
/// The function automatically configures SSH with appropriate settings for cloud environments:
/// - **Private Key**: Uses configured private key from user settings
/// - **Host Key Checking**: Disabled for cloud pods (security consideration)
/// - **Known Hosts**: Uses /dev/null to avoid host key conflicts
/// - **Port Forwarding**: Supports custom ports from pod SSH commands
/// - **Connection Timeout**: Relies on SSH client defaults
///
/// # Security Considerations
/// - Only uses configured private keys (no password authentication)
/// - Disables host key checking for cloud environments
/// - SSH commands are not logged to avoid exposing connection details
/// - Uses standard SSH client security features
///
/// # Error Conditions
/// - Pod target not found or inaccessible
/// - Multiple pods match the target (ambiguous reference)
/// - Pod has no SSH connection information
/// - SSH private key not found or not readable
/// - SSH connection failure (network, authentication, etc.)
/// - SSH client not available on the system
///
/// # Examples
/// ```rust
/// use lium_cli::commands::ssh::handle;
/// use lium_cli::config::Config;
///
/// let config = Config::new()?;
///
/// // Connect to pod by index
/// handle("1".to_string(), &config).await?;
///
/// // Connect to pod by HUID
/// handle("exec-abc123".to_string(), &config).await?;
///
/// // Connect to pod by name
/// handle("my-training-pod".to_string(), &config).await?;
/// ```
///
/// # Interactive Experience
/// Once connected, users have full shell access to the pod with:
/// - Complete terminal functionality (colors, cursor movement, etc.)
/// - Access to all installed software and tools
/// - GPU access and CUDA tools (if available)
/// - File system access for data and model management
/// - Network connectivity for downloading packages or data
///
/// # Connection Details Display
/// Before establishing the connection, the function displays:
/// ```text
/// 🔗 Connecting to pod exec-abc123 (my-training-pod)...
/// Host: gpu-host.example.com:2222, User: root
/// ```
///
/// # Common Use Cases
/// - **Development**: Interactive coding and debugging
/// - **Monitoring**: Real-time system and GPU monitoring
/// - **Data Management**: File operations and data preprocessing
/// - **Model Training**: Interactive training sessions and experiments
/// - **System Administration**: Package installation and configuration
///
/// # Troubleshooting
/// Common connection issues and solutions:
/// - **Key not found**: Configure SSH key with `lium config set ssh.key_path`
/// - **Permission denied**: Ensure private key has correct permissions (600)
/// - **Connection timeout**: Check network connectivity and pod status
/// - **Host unreachable**: Verify pod is running and accessible
///
/// # TODO
/// - Add support for SSH connection options (compression, ciphers, etc.)
/// - Implement connection retry logic for transient failures
/// - Add support for SSH tunneling and port forwarding
/// - Support for SSH agent integration
/// - Add connection history and favorites
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
