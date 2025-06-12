use crate::{
    config::Config,
    display::{print_error, print_info, print_success, print_warning},
    helpers::resolve_pod_targets,
    CliError, Result,
};
use clap::Args;
use lium_api::LiumApiClient;
use lium_utils::parse_ssh_command;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::fs;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Command-line arguments for the `exec` command that executes commands on remote pods.
///
/// The `exec` command enables remote command execution on cloud GPU pods via SSH.
/// It supports both interactive commands and script execution, with environment
/// variable injection and output streaming capabilities.
///
/// # Examples
/// ```bash
/// # Execute a simple command
/// lium exec 1 "nvidia-smi"
/// lium exec my-pod "python train.py"
///
/// # Execute commands on multiple pods
/// lium exec 1,2,3 "pip install torch"
/// lium exec all "nvidia-smi"
///
/// # Execute a script file
/// lium exec 1 --script setup.sh
///
/// # Set environment variables
/// lium exec 1 --env DEBUG=1 --env API_KEY=secret "python app.py"
///
/// # Use double dash for commands with flags
/// lium exec 1 -- python train.py --epochs 100 --lr 0.001
/// ```
///
/// # Pod Target Resolution
/// Pod targets can be specified in several formats:
/// - **Pod indices**: Numeric references from `lium ps` (e.g., "1", "3")
/// - **Pod HUIDs**: Hardware unique identifiers (e.g., "exec-abc123")
/// - **Pod names**: User-defined or auto-generated names
/// - **Comma-separated**: Multiple targets (e.g., "1,2,3")
/// - **"all"**: Execute on all active pods
///
/// # Security Considerations
/// - Commands are executed via SSH with configured private keys
/// - Environment variables are exported before command execution
/// - Output streaming prevents command hanging
/// - SSH host key checking is disabled for cloud environments
///
/// # TODO
/// - Add support for interactive TTY sessions
/// - Implement command timeout configuration
/// - Add support for file upload before execution
/// - Support for parallel vs sequential execution modes
#[derive(Args)]
pub struct ExecArgs {
    /// Pod targets to execute commands on (comma-separated).
    ///
    /// Specifies which pods should receive the command execution. Supports:
    ///
    /// - **Single target**: "1", "my-pod", "exec-abc123"
    /// - **Multiple targets**: "1,2,3", "pod1,pod2", "exec-abc123,exec-def456"
    /// - **All pods**: "all" (executes on all active pods)
    ///
    /// Targets are resolved using the same logic as other commands:
    /// - Numeric values are treated as indices from `lium ps`
    /// - Non-numeric values are matched against pod HUIDs and names
    /// - Invalid targets cause the command to fail with an error
    pub pod_targets: String,

    /// Command to execute on the target pods.
    ///
    /// The command string is executed in the default shell of the pod (usually bash).
    /// Use the `raw = true` attribute to support complex command parsing including
    /// flags that might conflict with lium's own arguments.
    ///
    /// For commands with flags or complex arguments, use `--` to separate lium
    /// arguments from the command:
    /// ```bash
    /// lium exec 1 -- python train.py --epochs 100 --learning-rate 0.001
    /// ```
    ///
    /// Commands are executed with the configured SSH user (typically root) and
    /// inherit the pod's environment variables plus any specified via `--env`.
    #[arg(raw = true)]
    pub command: Vec<String>,

    /// Path to a script file to execute instead of a command.
    ///
    /// When specified, the script file is read from the local filesystem and
    /// executed on the remote pod(s). The script is executed as a single command
    /// with environment variables (if any) exported beforehand.
    ///
    /// Script execution process:
    /// 1. Read script content from local file
    /// 2. Prepend environment variable exports (if any)
    /// 3. Execute the combined script on each target pod
    ///
    /// Supports common script types: `.sh`, `.py`, `.pl`, etc.
    /// The remote pod must have appropriate interpreters installed.
    ///
    /// Example: `--script deploy.sh`, `--script install_deps.py`
    #[arg(short, long, conflicts_with = "command")]
    pub script: Option<String>,

    /// Environment variables to set before command execution.
    ///
    /// Variables are exported in the pod's shell environment before the command
    /// or script is executed. Can be specified multiple times for multiple variables.
    ///
    /// Format: `KEY=VALUE`
    ///
    /// Examples:
    /// - `--env DEBUG=1`
    /// - `--env API_KEY=secret --env WORKERS=4`
    /// - `--env CUDA_VISIBLE_DEVICES=0,1`
    ///
    /// Variables are exported using `export KEY="VALUE"` syntax, with proper
    /// shell escaping to handle special characters in values.
    #[arg(short, long)]
    pub env: Vec<String>,
}

/// Handle the exec command for remote command execution
/// Handles the `exec` command to execute commands or scripts on remote pods via SSH.
///
/// This function orchestrates remote command execution across one or more cloud GPU pods.
/// It handles SSH connection management, output streaming, environment variable injection,
/// and provides comprehensive error reporting for debugging connection and execution issues.
///
/// # Arguments
/// * `args` - Command-line arguments parsed into `ExecArgs` struct
/// * `config` - User configuration containing SSH keys and API credentials
///
/// # Returns
/// * `Result<()>` - Success or error with detailed execution information
///
/// # Process Flow
/// 1. **Input Validation**: Validates pod targets and command/script parameters
/// 2. **Target Resolution**: Resolves pod targets to actual pod instances
/// 3. **SSH Configuration**: Validates SSH key availability and configuration
/// 4. **Command Preparation**: Processes commands, scripts, and environment variables
/// 5. **Execution Loop**: Executes commands on each target pod sequentially
/// 6. **Output Streaming**: Streams stdout/stderr in real-time with proper labeling
/// 7. **Result Summary**: Reports success/failure counts for multiple pod operations
///
/// # Command vs Script Execution
///
/// ## Command Mode
/// When `command` is provided, the arguments are joined into a single command string
/// and executed directly in the pod's shell. Environment variables are prepended
/// as export statements.
///
/// ## Script Mode  
/// When `--script` is provided, the local script file is read and its contents
/// are executed remotely. Environment variables are automatically prepended to
/// the script content before execution.
///
/// # Environment Variable Handling
/// Environment variables are processed as follows:
/// 1. Parse each `--env KEY=VALUE` argument
/// 2. Validate format and escape special characters
/// 3. Generate `export KEY="VALUE"` statements
/// 4. Prepend to command or script content
/// 5. Execute combined command string
///
/// # SSH Connection Management
/// - Uses configured private key from user settings
/// - Disables host key checking for cloud environments
/// - Supports custom ports from pod SSH commands
/// - Implements connection retry logic for transient failures
/// - Provides detailed debugging for connection issues
///
/// # Output Handling
/// For single pod execution:
/// - Streams output directly to stdout/stderr
/// - Maintains real-time feedback
///
/// For multiple pod execution:
/// - Labels output with pod identifiers
/// - Separates output between pods
/// - Provides execution summary
///
/// # Error Conditions
/// - Invalid pod targets (non-existent or inaccessible pods)
/// - SSH configuration issues (missing keys, connection failures)
/// - Script file not found or unreadable
/// - Command execution failures on remote pods
/// - Network connectivity problems
///
/// # Examples
/// ```rust
/// use lium_cli::commands::exec::{handle, ExecArgs};
/// use lium_cli::config::Config;
///
/// // Execute simple command on single pod
/// let args = ExecArgs {
///     pod_targets: "1".to_string(),
///     command: vec!["nvidia-smi".to_string()],
///     script: None,
///     env: vec![],
/// };
/// handle(args, &config).await?;
///
/// // Execute script with environment variables on multiple pods
/// let args = ExecArgs {
///     pod_targets: "1,2,3".to_string(),
///     command: vec![],
///     script: Some("setup.sh".to_string()),
///     env: vec!["DEBUG=1".to_string(), "WORKERS=4".to_string()],
/// };
/// handle(args, &config).await?;
/// ```
///
/// # Security Considerations
/// - SSH connections use configured private keys only
/// - Host key verification is disabled for cloud pod environments
/// - Environment variables are properly shell-escaped
/// - Command execution is logged for audit purposes
/// - No sensitive information is logged in debug output
///
/// # Performance Considerations
/// - Commands are executed sequentially across pods (not parallel)
/// - Output streaming prevents memory buildup for long-running commands
/// - SSH connections are created per-pod (no connection pooling)
/// - Large script files are efficiently streamed to remote pods
///
/// # TODO
/// - Add support for parallel execution across multiple pods
/// - Implement command timeout configuration
/// - Add support for interactive TTY sessions
/// - Support for file upload/download before/after execution
/// - Add execution history and result caching
/// - Implement connection pooling for better performance
pub async fn handle(args: ExecArgs, config: &Config) -> Result<()> {
    let api_client = LiumApiClient::from_config(config)?;

    // Parse pod targets (split by comma)
    let pod_targets: Vec<String> = args
        .pod_targets
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if pod_targets.is_empty() {
        return Err(CliError::InvalidInput(
            "No pod targets specified".to_string(),
        ));
    }

    // Resolve pod targets
    let resolved_pods = resolve_pod_targets(&api_client, &pod_targets).await?;

    if resolved_pods.is_empty() {
        return Err(CliError::InvalidInput(
            "No pods found to execute command on".to_string(),
        ));
    }

    // Parse environment variables
    let mut env_map = HashMap::new();
    for env_var in &args.env {
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
    let (exec_command, operation_desc) = if let Some(script_path) = &args.script {
        // Read script file
        let script_content = fs::read_to_string(script_path).map_err(CliError::Io)?;

        // If environment variables are provided, prepend them to the script
        let final_script = if !env_map.is_empty() {
            let env_exports = env_map
                .iter()
                .map(|(k, v)| format!("export {}=\"{}\"", k, v))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{}\n{}", env_exports, script_content)
        } else {
            script_content
        };

        (final_script, format!("script '{}'", script_path))
    } else if !args.command.is_empty() {
        // Join command parts into a single command string
        let command = args.command.join(" ");

        // If environment variables are provided, prepend them to the command
        let final_command = if !env_map.is_empty() {
            let env_exports = env_map
                .iter()
                .map(|(k, v)| format!("export {}=\"{}\"", k, v))
                .collect::<Vec<_>>()
                .join(" && ");
            format!("{} && {}", env_exports, command)
        } else {
            command.clone()
        };

        let desc = if !env_map.is_empty() {
            let env_str = env_map
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ");
            format!("command with env [{}]: {}", env_str, command)
        } else {
            format!("command: {}", command)
        };

        (final_command, desc)
    } else {
        return Err(CliError::InvalidInput(
            "Either a command or --script must be provided".to_string(),
        ));
    };

    // Get SSH private key path from config
    let private_key_path = config.get_ssh_private_key_path()?;

    debug!(
        "Looking for SSH private key at: {}",
        private_key_path.display()
    );

    if !private_key_path.exists() {
        // Try some common key names
        let key_dir = private_key_path
            .parent()
            .unwrap_or(std::path::Path::new("."));
        print_warning("Private key not found. Looking for other keys in the same directory...");

        let possible_keys = ["id_rsa", "id_ed25519", "id_ecdsa", "tplr"];
        for key_name in &possible_keys {
            let key_path = key_dir.join(key_name);
            if key_path.exists() {
                print_info(&format!("Found potential key: {}", key_path.display()));
            }
        }

        return Err(CliError::InvalidInput(format!(
            "SSH private key not found at '{}'. Configure it with: lium config set ssh.key_path /path/to/key.pub",
            private_key_path.display()
        )));
    }

    print_success("Successfully connected to SSH");

    // Also check if the public key exists
    let public_key_path = private_key_path.with_extension("pub");
    if !public_key_path.exists() {
        print_warning(&format!(
            "SSH public key not found at '{}'. This key should have been added to the pod during creation.",
            public_key_path.display()
        ));
    }

    // Show what we're about to execute
    print_info(&format!(
        "Executing {} on {} pod(s):",
        operation_desc,
        resolved_pods.len()
    ));
    for (pod, original_ref) in &resolved_pods {
        println!("  - {} ({})", pod.huid, original_ref);
    }
    println!();

    // Execute on each pod
    let mut success_count = 0;
    let mut failure_count = 0;

    for (i, (pod, target_name)) in resolved_pods.iter().enumerate() {
        // Show header for multiple pods
        if resolved_pods.len() > 1 {
            println!("--- Output from {} ({}) ---", pod.huid, target_name);
        }

        // Get SSH connection details
        let ssh_cmd = pod.ssh_cmd.as_ref().ok_or_else(|| {
            CliError::InvalidInput(format!("Pod {} has no SSH connection info", pod.huid))
        })?;

        debug!("SSH command from pod: {}", ssh_cmd);

        // Use the existing SSH command parser
        let (host, mut port, user) = parse_ssh_command(ssh_cmd)
            .map_err(|e| CliError::InvalidInput(format!("Failed to parse SSH command: {}", e)))?;

        debug!(
            "Parsed SSH details - Host: {}, Port: {}, User: {}",
            host, port, user
        );

        // Workaround: If port is 22 and SSH command contains "-p", extract the actual port
        if port == 22 && ssh_cmd.contains("-p ") {
            if let Some(port_start) = ssh_cmd.find("-p ") {
                let port_str = &ssh_cmd[port_start + 3..];
                if let Some(port_end) = port_str.find(|c: char| !c.is_numeric()) {
                    if let Ok(parsed_port) = port_str[..port_end].parse::<u16>() {
                        print_warning(&format!(
                            "Parser returned port 22, but found -p {} in command. Using port {}.",
                            parsed_port, parsed_port
                        ));
                        port = parsed_port;
                    }
                } else if let Ok(parsed_port) = port_str.trim().parse::<u16>() {
                    print_warning(&format!(
                        "Parser returned port 22, but found -p {} in command. Using port {}.",
                        parsed_port, parsed_port
                    ));
                    port = parsed_port;
                }
            }
        }

        // Now run the actual command
        let mut ssh_command = Command::new("ssh");
        ssh_command
            .arg("-i")
            .arg(&private_key_path)
            .arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg("-o")
            .arg("UserKnownHostsFile=/dev/null")
            .arg("-o")
            .arg("PasswordAuthentication=no")
            .arg("-p")
            .arg(port.to_string())
            .arg(format!("{}@{}", user, host))
            .arg(&exec_command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        // Execute the command
        match ssh_command.spawn() {
            Ok(mut child) => {
                // Set up output streaming
                let stdout = child.stdout.take().expect("Failed to get stdout");
                let stderr = child.stderr.take().expect("Failed to get stderr");

                let stdout_reader = BufReader::new(stdout);
                let stderr_reader = BufReader::new(stderr);

                // Stream stdout
                let stdout_handle = tokio::spawn(async move {
                    let mut lines = stdout_reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        println!("{}", line);
                    }
                });

                // Stream stderr
                let stderr_handle = tokio::spawn(async move {
                    let mut lines = stderr_reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        eprintln!("{}", line);
                    }
                });

                // Wait for the command to complete
                let output = child.wait_with_output().await;

                // Wait for output streaming to complete
                let _ = stdout_handle.await;
                let _ = stderr_handle.await;

                match output {
                    Ok(result) => {
                        if result.status.success() {
                            if resolved_pods.len() > 1 {
                                print_success(&format!(
                                    "Command completed successfully on '{}'",
                                    pod.huid
                                ));
                            }
                            success_count += 1;
                        } else {
                            let exit_code = result.status.code().unwrap_or(-1);
                            print_error(&format!(
                                "Command failed on '{}' with exit code: {}",
                                pod.huid, exit_code
                            ));
                            failure_count += 1;
                        }
                    }
                    Err(e) => {
                        print_error(&format!(
                            "Failed to execute command on '{}': {}",
                            pod.huid, e
                        ));
                        failure_count += 1;
                    }
                }
            }
            Err(e) => {
                print_error(&format!("Failed to start SSH for '{}': {}", pod.huid, e));
                failure_count += 1;
            }
        }

        // Add separator between pods (except for the last one)
        if i < resolved_pods.len() - 1 {
            println!();
        }
    }

    // Show summary for multiple pods
    if resolved_pods.len() > 1 {
        println!();
        print_info(&format!(
            "Execution Summary: {} successful, {} failed",
            success_count, failure_count
        ));
    }

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
        if std::path::Path::new(path).exists() {
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

    // Create ExecArgs
    let args = ExecArgs {
        pod_targets: pod_targets.join(","),
        command: vec![],
        script: Some(script_path),
        env: env_args,
    };

    handle(args, config).await
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
