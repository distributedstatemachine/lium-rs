use crate::commands::{ls, ps, up};
use crate::config::Config;
use crate::errors::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lium")]
#[command(about = "A CLI tool for Celium Compute")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize configuration
    Init,
    /// List available executors
    Ls(ls::LsArgs),
    /// Start a new pod
    Up(up::UpArgs),
    /// List active pods
    Ps(ps::PsArgs),
    /// Execute command in pod(s)
    Exec {
        /// Pod HUID(s), index(es), or "all"
        #[arg(value_name = "POD_TARGET")]
        pods: Vec<String>,
        /// Command to execute
        #[arg(last = true, value_name = "COMMAND")]
        command: Vec<String>,
        /// Path to script file to upload and execute
        #[arg(long)]
        script: Option<std::path::PathBuf>,
        /// Environment variables (KEY=VALUE format)
        #[arg(long)]
        env: Vec<String>,
    },
    /// SSH into pod
    Ssh {
        /// Pod HUID or index
        pod: String,
    },
    /// Copy files to/from pod(s)
    Scp {
        /// Pod HUID(s), index(es), or source path
        #[arg(value_name = "SOURCE")]
        source: String,
        /// Destination path or pod targets
        #[arg(value_name = "DESTINATION")]
        destination: String,
        /// Copy wallet files (coldkey)
        #[arg(long)]
        coldkey: Option<String>,
        /// Copy wallet files (hotkey)
        #[arg(long)]
        hotkey: Option<String>,
    },
    /// Sync files with pod using rsync
    Rsync {
        /// Source path
        source: String,
        /// Destination path
        destination: String,
        /// Additional rsync options
        #[arg(short, long)]
        options: Option<String>,
    },
    /// Stop and remove pod(s)
    Down {
        /// Pod HUID(s), index(es), or "all"
        #[arg(value_name = "POD_TARGET")]
        pods: Vec<String>,
        /// Stop all pods
        #[arg(long)]
        all: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
    /// Manage Docker images
    Image {
        #[command(subcommand)]
        action: ImageCommands,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Funding and billing
    Fund {
        #[command(subcommand)]
        action: FundCommands,
    },
    /// Theme management
    Theme {
        #[command(subcommand)]
        action: ThemeCommands,
    },
}

#[derive(Subcommand)]
pub enum ImageCommands {
    /// List available templates
    List,
    /// Create new template
    Create {
        /// Template name
        name: String,
        /// Docker image
        image: String,
        /// Docker image tag
        #[arg(short, long)]
        tag: Option<String>,
    },
    /// Delete template
    Delete {
        /// Template ID
        id: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },
    /// Reset configuration to defaults
    Reset,
    /// Initialize configuration interactively
    Init,
}

#[derive(Subcommand)]
pub enum FundCommands {
    /// Show wallet balance
    Balance,
    /// Add funds to wallet
    Add {
        /// Amount to add
        amount: f64,
    },
    /// Show billing history
    History,
}

#[derive(Subcommand)]
pub enum ThemeCommands {
    /// List available themes
    List,
    /// Set theme
    Set {
        /// Theme name
        name: String,
    },
}

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::new()?;

    match cli.command {
        Commands::Init => handle_init().await,
        Commands::Ls(args) => ls::handle_ls(args, &config).await,
        Commands::Up(args) => up::handle_up(args, &config).await,
        Commands::Ps(args) => ps::handle_ps(args, &config).await,
        Commands::Exec {
            pods,
            command,
            script,
            env,
        } => handle_exec(pods, command, script, env, &config).await,
        Commands::Ssh { pod } => handle_ssh(pod, &config).await,
        Commands::Scp {
            source,
            destination,
            coldkey,
            hotkey,
        } => handle_scp(source, destination, coldkey, hotkey, &config).await,
        Commands::Rsync {
            source,
            destination,
            options,
        } => handle_rsync(source, destination, options, &config).await,
        Commands::Down { pods, all, yes } => handle_down(pods, all, yes, &config).await,
        Commands::Image { action } => handle_image(action, &config).await,
        Commands::Config { action } => handle_config(action).await,
        Commands::Fund { action } => handle_fund(action, &config).await,
        Commands::Theme { action } => handle_theme(action, &config).await,
    }
}

async fn handle_init() -> Result<()> {
    use crate::display::{print_info, print_success, prompt_input};
    use std::path::Path;

    print_info("Initializing Lium configuration...");

    let api_key = prompt_input("Enter your Celium API key:", None)?;

    let mut config = Config::new()?;
    config.set_api_key(&api_key)?;

    // SSH key setup
    let default_key_path = format!(
        "{}/.ssh/id_rsa.pub",
        std::env::var("HOME").unwrap_or_else(|_| "/root".to_string())
    );

    let ssh_key_path = prompt_input("Enter SSH public key path:", Some(&default_key_path))?;

    // Check if the key file exists
    let key_path = Path::new(&ssh_key_path);
    if !key_path.exists() {
        return Err(crate::errors::LiumError::InvalidInput(format!(
            "SSH key file not found: {}. Please create an SSH key pair first.",
            ssh_key_path
        )));
    }

    config.set_ssh_public_key_path(&ssh_key_path)?;
    config.set_ssh_user("root")?; // Default user as per spec

    // Test API connection
    print_info("Testing API connection...");
    let client = crate::api::LiumApiClient::from_config()?;
    match client.test_connection().await {
        Ok(_) => print_success("API connection successful!"),
        Err(e) => {
            print_info(&format!("Warning: API connection test failed: {}", e));
            print_info("Your configuration has been saved, but please verify your API key.");
        }
    }

    config.save()?;
    print_success("Configuration saved successfully!");

    Ok(())
}

async fn handle_exec(
    pod_targets: Vec<String>,
    command: Vec<String>,
    script_path: Option<std::path::PathBuf>,
    env_vars: Vec<String>,
    config: &Config,
) -> Result<()> {
    use crate::api::LiumApiClient;
    use crate::display::{print_error, print_info, print_warning};
    use crate::helpers::{extract_ssh_details, resolve_pod_targets};
    use crate::ssh_utils;
    use std::collections::HashMap;

    // Validate input
    if pod_targets.is_empty() {
        return Err(crate::errors::LiumError::InvalidInput(
            "No pod targets specified".to_string(),
        ));
    }

    if command.is_empty() && script_path.is_none() {
        return Err(crate::errors::LiumError::InvalidInput(
            "No command or script specified. Use either command args or --script".to_string(),
        ));
    }

    if !command.is_empty() && script_path.is_some() {
        return Err(crate::errors::LiumError::InvalidInput(
            "Cannot specify both command and script. Use one or the other.".to_string(),
        ));
    }

    let client = LiumApiClient::from_config()?;

    // Resolve pod targets
    let resolved_pods = resolve_pod_targets(&client, &pod_targets).await?;

    // Parse environment variables
    let mut env_map = HashMap::new();
    for env_var in env_vars {
        if let Some((key, value)) = env_var.split_once('=') {
            env_map.insert(key.to_string(), value.to_string());
        } else {
            print_warning(&format!(
                "Invalid environment variable format: {}. Use KEY=VALUE",
                env_var
            ));
        }
    }

    // Determine command to execute
    let command_str = if let Some(script_path) = script_path {
        // Read script file
        let script_content =
            std::fs::read_to_string(&script_path).map_err(crate::errors::LiumError::Io)?;

        // Upload script and execute it
        format!("cat > /tmp/lium_script.sh << 'EOF'\n{}\nEOF\nchmod +x /tmp/lium_script.sh\n/tmp/lium_script.sh", script_content)
    } else {
        command.join(" ")
    };

    print_info(&format!("Executing on {} pod(s):", resolved_pods.len()));

    let private_key_path = config.get_ssh_private_key_path()?;
    let mut failed_pods = Vec::new();
    let mut success_count = 0;

    // Execute on all pods (concurrently if multiple)
    if resolved_pods.len() == 1 {
        // Single pod - stream output directly
        let (pod, identifier) = &resolved_pods[0];
        print_info(&format!("Pod: {} ({})", pod.huid, identifier));

        let (host, port, user) = extract_ssh_details(pod)?;
        let result = ssh_utils::execute_remote_command(
            &host,
            port,
            &user,
            &private_key_path,
            &command_str,
            if env_map.is_empty() {
                None
            } else {
                Some(env_map)
            },
        )
        .await;

        match result {
            Ok((_stdout, _stderr, exit_code)) => {
                if exit_code != 0 {
                    print_error(&format!("Command failed with exit code: {}", exit_code));
                    std::process::exit(exit_code);
                }
                success_count += 1;
            }
            Err(e) => {
                print_error(&format!("Failed to execute on {}: {}", pod.huid, e));
                failed_pods.push(pod.huid.clone());
            }
        }
    } else {
        // Multiple pods - execute concurrently and collect results
        use futures::future::join_all;

        let futures = resolved_pods.iter().map(|(pod, identifier)| async {
            let (host, port, user) = match extract_ssh_details(pod) {
                Ok(details) => details,
                Err(e) => return (pod.huid.clone(), identifier.clone(), Err(e)),
            };

            let result = ssh_utils::execute_remote_command(
                &host,
                port,
                &user,
                &private_key_path,
                &command_str,
                if env_map.is_empty() {
                    None
                } else {
                    Some(env_map.clone())
                },
            )
            .await;

            (pod.huid.clone(), identifier.clone(), result)
        });

        let results = join_all(futures).await;

        for (huid, identifier, result) in results {
            match result {
                Ok((stdout, stderr, exit_code)) => {
                    println!("=== {} ({}) ===", huid, identifier);
                    if !stdout.is_empty() {
                        println!("{}", stdout);
                    }
                    if !stderr.is_empty() {
                        eprintln!("{}", stderr);
                    }
                    if exit_code == 0 {
                        success_count += 1;
                    } else {
                        print_error(&format!("Exit code: {}", exit_code));
                        failed_pods.push(huid);
                    }
                    println!();
                }
                Err(e) => {
                    print_error(&format!("Failed to execute on {}: {}", huid, e));
                    failed_pods.push(huid);
                }
            }
        }
    }

    // Summary
    if failed_pods.is_empty() {
        print_info(&format!(
            "Command executed successfully on {} pod(s)",
            success_count
        ));
    } else {
        print_error(&format!(
            "Command failed on {} pod(s): {}. Succeeded on {} pod(s).",
            failed_pods.len(),
            failed_pods.join(", "),
            success_count
        ));
        std::process::exit(1);
    }

    Ok(())
}

async fn handle_ssh(pod: String, config: &Config) -> Result<()> {
    use crate::api::LiumApiClient;
    use crate::display::print_info;
    use crate::helpers::{extract_ssh_details, resolve_single_pod_target};
    use crate::ssh_utils;

    let client = LiumApiClient::from_config()?;

    // Resolve the pod
    let pod_info = resolve_single_pod_target(&client, &pod).await?;

    print_info(&format!("Connecting to pod: {}", pod_info.huid));

    // Extract SSH connection details
    let (host, port, user) = extract_ssh_details(&pod_info)?;
    let private_key_path = config.get_ssh_private_key_path()?;

    // Execute interactive SSH
    ssh_utils::execute_ssh_interactive(&host, port, &user, &private_key_path)?;

    Ok(())
}

async fn handle_scp(
    source: String,
    destination: String,
    coldkey: Option<String>,
    hotkey: Option<String>,
    _config: &Config,
) -> Result<()> {
    use crate::api::LiumApiClient;
    use crate::display::print_info;
    use crate::helpers::{extract_ssh_details, resolve_single_pod_target};
    use crate::ssh_utils;
    use std::path::Path;

    let client = LiumApiClient::from_config()?;

    // Handle wallet file copying if specified
    if coldkey.is_some() || hotkey.is_some() {
        return handle_wallet_copy(source, coldkey, hotkey, &client, _config).await;
    }

    // Parse source and destination to determine if it's upload or download
    let (pod_target, local_path, remote_path, is_upload) = if source.contains(':') {
        // Download: pod:remote_path -> local_path
        let parts: Vec<&str> = source.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(crate::errors::LiumError::InvalidInput(
                "Invalid source format. Use pod:remote_path".to_string(),
            ));
        }
        (
            parts[0].to_string(),
            destination,
            parts[1].to_string(),
            false,
        )
    } else if destination.contains(':') {
        // Upload: local_path -> pod:remote_path
        let parts: Vec<&str> = destination.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(crate::errors::LiumError::InvalidInput(
                "Invalid destination format. Use pod:remote_path".to_string(),
            ));
        }
        (parts[0].to_string(), source, parts[1].to_string(), true)
    } else {
        return Err(crate::errors::LiumError::InvalidInput(
            "Either source or destination must specify pod with pod:path format".to_string(),
        ));
    };

    // Resolve the pod
    let pod_info = resolve_single_pod_target(&client, &pod_target).await?;

    print_info(&format!(
        "{} {} {} pod: {}",
        if is_upload {
            "Uploading to"
        } else {
            "Downloading from"
        },
        if is_upload { &local_path } else { &remote_path },
        if is_upload { "to" } else { "from" },
        pod_info.huid
    ));

    // Extract SSH connection details
    let (host, port, user) = extract_ssh_details(&pod_info)?;
    let private_key_path = _config.get_ssh_private_key_path()?;

    // Ensure remote directory exists if uploading
    if is_upload {
        let remote_dir = Path::new(&remote_path).parent();
        if let Some(dir) = remote_dir {
            let dir_str = dir.to_string_lossy();
            if !dir_str.is_empty() && dir_str != "/" {
                print_info(&format!("Creating remote directory: {}", dir_str));
                let mkdir_cmd = format!("mkdir -p '{}'", dir_str);
                let _ = ssh_utils::execute_remote_command(
                    &host,
                    port,
                    &user,
                    &private_key_path,
                    &mkdir_cmd,
                    None,
                )
                .await;
            }
        }
    }

    // Execute SCP
    ssh_utils::execute_scp_command(
        &host,
        port,
        &user,
        &private_key_path,
        &local_path,
        &remote_path,
        is_upload,
    )?;

    Ok(())
}

/// Handle wallet file copying to multiple pods
async fn handle_wallet_copy(
    pod_targets: String,
    coldkey: Option<String>,
    hotkey: Option<String>,
    client: &crate::api::LiumApiClient,
    _config: &Config,
) -> Result<()> {
    use crate::display::{print_error, print_info, print_success};
    use crate::helpers::{extract_ssh_details, resolve_pod_targets};
    use crate::ssh_utils;
    use std::path::PathBuf;

    // Parse pod targets
    let targets: Vec<String> = pod_targets
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let resolved_pods = resolve_pod_targets(client, &targets).await?;

    if resolved_pods.is_empty() {
        return Err(crate::errors::LiumError::InvalidInput(
            "No valid pod targets found".to_string(),
        ));
    }

    // Determine wallet file paths
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let mut files_to_copy = Vec::new();

    if let Some(coldkey_name) = coldkey {
        let coldkey_path = PathBuf::from(&home_dir)
            .join(".bittensor")
            .join("wallets")
            .join(&coldkey_name)
            .join("coldkey");
        files_to_copy.push((
            coldkey_path,
            format!(".bittensor/wallets/{}/coldkey", coldkey_name),
        ));
    }

    if let Some(hotkey_name) = hotkey {
        let hotkey_path = PathBuf::from(&home_dir)
            .join(".bittensor")
            .join("wallets")
            .join("default")
            .join("hotkeys")
            .join(&hotkey_name);
        files_to_copy.push((
            hotkey_path,
            format!(".bittensor/wallets/default/hotkeys/{}", hotkey_name),
        ));
    }

    if files_to_copy.is_empty() {
        return Err(crate::errors::LiumError::InvalidInput(
            "No wallet files specified. Use --coldkey or --hotkey".to_string(),
        ));
    }

    // Verify local files exist
    for (local_path, _) in &files_to_copy {
        if !local_path.exists() {
            return Err(crate::errors::LiumError::InvalidInput(format!(
                "Wallet file not found: {}",
                local_path.display()
            )));
        }
    }

    let private_key_path = _config.get_ssh_private_key_path()?;
    let mut failed_pods = Vec::new();
    let mut success_count = 0;

    print_info(&format!(
        "Copying wallet files to {} pod(s):",
        resolved_pods.len()
    ));

    // Copy to each pod
    for (pod, identifier) in resolved_pods {
        print_info(&format!("Copying to pod {} ({})...", pod.huid, identifier));

        let (host, port, user) = match extract_ssh_details(&pod) {
            Ok(details) => details,
            Err(e) => {
                print_error(&format!(
                    "Failed to get SSH details for {}: {}",
                    pod.huid, e
                ));
                failed_pods.push(pod.huid);
                continue;
            }
        };

        let mut pod_success = true;

        // Create .bittensor directory structure
        let mkdir_cmd = "mkdir -p ~/.bittensor/wallets/default/hotkeys";
        if let Err(e) = ssh_utils::execute_remote_command(
            &host,
            port,
            &user,
            &private_key_path,
            mkdir_cmd,
            None,
        )
        .await
        {
            print_error(&format!(
                "Failed to create wallet directory on {}: {}",
                pod.huid, e
            ));
            failed_pods.push(pod.huid);
            continue;
        }

        // Copy each file
        for (local_path, remote_rel_path) in &files_to_copy {
            let remote_path = format!("~/{}", remote_rel_path);

            match ssh_utils::execute_scp_command(
                &host,
                port,
                &user,
                &private_key_path,
                &local_path.to_string_lossy(),
                &remote_path,
                true,
            ) {
                Ok(_) => {
                    print_info(&format!(
                        "  ✓ Copied {}",
                        local_path.file_name().unwrap().to_string_lossy()
                    ));
                }
                Err(e) => {
                    print_error(&format!(
                        "  ✗ Failed to copy {}: {}",
                        local_path.file_name().unwrap().to_string_lossy(),
                        e
                    ));
                    pod_success = false;
                }
            }
        }

        if pod_success {
            print_success(&format!("Successfully copied wallet files to {}", pod.huid));
            success_count += 1;
        } else {
            failed_pods.push(pod.huid);
        }
    }

    // Summary
    if failed_pods.is_empty() {
        print_success(&format!(
            "Successfully copied wallet files to {} pod(s)",
            success_count
        ));
    } else {
        print_error(&format!(
            "Failed to copy to {} pod(s): {}. Succeeded on {} pod(s).",
            failed_pods.len(),
            failed_pods.join(", "),
            success_count
        ));
    }

    Ok(())
}

async fn handle_rsync(
    source: String,
    destination: String,
    options: Option<String>,
    config: &Config,
) -> Result<()> {
    use crate::api::LiumApiClient;
    use crate::display::print_info;
    use crate::helpers::{extract_ssh_details, resolve_single_pod_target};
    use crate::ssh_utils;

    let client = LiumApiClient::from_config()?;

    // Parse source and destination to determine if it's upload or download
    let (pod_target, local_path, remote_path, is_upload) = if source.contains(':') {
        // Download: pod:remote_path -> local_path
        let parts: Vec<&str> = source.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(crate::errors::LiumError::InvalidInput(
                "Invalid source format. Use pod:remote_path".to_string(),
            ));
        }
        (
            parts[0].to_string(),
            destination,
            parts[1].to_string(),
            false,
        )
    } else if destination.contains(':') {
        // Upload: local_path -> pod:remote_path
        let parts: Vec<&str> = destination.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(crate::errors::LiumError::InvalidInput(
                "Invalid destination format. Use pod:remote_path".to_string(),
            ));
        }
        (parts[0].to_string(), source, parts[1].to_string(), true)
    } else {
        return Err(crate::errors::LiumError::InvalidInput(
            "Either source or destination must specify pod with pod:path format".to_string(),
        ));
    };

    // Resolve the pod
    let pod_info = resolve_single_pod_target(&client, &pod_target).await?;

    print_info(&format!(
        "Syncing {} {} pod: {}",
        if is_upload { "to" } else { "from" },
        if is_upload { &local_path } else { &remote_path },
        pod_info.huid
    ));

    // Extract SSH connection details
    let (host, port, user) = extract_ssh_details(&pod_info)?;
    let private_key_path = config.get_ssh_private_key_path()?;

    // Execute rsync
    ssh_utils::execute_rsync_command(
        &host,
        port,
        &user,
        &private_key_path,
        &local_path,
        &remote_path,
        options.as_deref(),
        is_upload,
    )?;

    Ok(())
}

async fn handle_down(
    pod_targets: Vec<String>,
    all: bool,
    yes: bool,
    _config: &Config,
) -> Result<()> {
    use crate::api::LiumApiClient;
    use crate::display::{print_error, print_info, print_success, print_warning, prompt_confirm};
    use crate::helpers::{get_executor_id_from_pod, resolve_pod_targets};

    let client = LiumApiClient::from_config()?;

    let resolved_pods = if all {
        // Get all pods
        let all_pods = client.get_pods().await?;
        all_pods
            .into_iter()
            .map(|pod| (pod, "all".to_string()))
            .collect()
    } else if pod_targets.is_empty() {
        return Err(crate::errors::LiumError::InvalidInput(
            "No pod targets specified. Use pod identifiers or --all".to_string(),
        ));
    } else {
        resolve_pod_targets(&client, &pod_targets).await?
    };

    if resolved_pods.is_empty() {
        print_info("No pods found to stop.");
        return Ok(());
    }

    // Show what will be stopped
    println!("Pods to be stopped:");
    for (pod, identifier) in &resolved_pods {
        println!("  {} ({}) - Status: {}", pod.huid, identifier, pod.status);
    }

    // Confirm unless -y flag
    if !yes {
        let confirm = prompt_confirm(
            &format!(
                "Are you sure you want to stop {} pod(s)?",
                resolved_pods.len()
            ),
            false,
        )?;

        if !confirm {
            print_info("Operation cancelled.");
            return Ok(());
        }
    }

    // Stop each pod
    let mut failed_pods = Vec::new();
    let mut success_count = 0;

    for (pod, identifier) in resolved_pods {
        print_info(&format!("Stopping pod {} ({})...", pod.huid, identifier));

        match get_executor_id_from_pod(&pod) {
            Ok(executor_id) => match client.unrent_pod(&executor_id).await {
                Ok(_) => {
                    print_success(&format!("Successfully stopped pod {}", pod.huid));
                    success_count += 1;
                }
                Err(e) => {
                    print_error(&format!("Failed to stop pod {}: {}", pod.huid, e));
                    failed_pods.push(pod.huid);
                }
            },
            Err(e) => {
                print_error(&format!(
                    "Could not determine executor for pod {}: {}",
                    pod.huid, e
                ));
                failed_pods.push(pod.huid);
            }
        }
    }

    // Summary
    if failed_pods.is_empty() {
        print_success(&format!("Successfully stopped {} pod(s)", success_count));
    } else {
        print_warning(&format!(
            "Stopped {} pod(s), failed to stop {} pod(s): {}",
            success_count,
            failed_pods.len(),
            failed_pods.join(", ")
        ));
    }

    Ok(())
}

async fn handle_image(action: ImageCommands, config: &Config) -> Result<()> {
    use crate::display::{print_error, print_info, print_success, prompt_confirm, prompt_input};

    match action {
        ImageCommands::List => {
            let client = crate::api::LiumApiClient::from_config()?;

            print_info("Fetching available templates...");

            match client.get_templates().await {
                Ok(templates) => {
                    if templates.is_empty() {
                        print_info("No templates available.");
                        return Ok(());
                    }

                    // Display templates in simple format for now
                    println!("Available templates:");
                    for (i, template) in templates.iter().enumerate() {
                        println!(
                            "{}. {} - {} ({})",
                            i + 1,
                            template.id,
                            template.name,
                            template.docker_image
                        );
                    }
                }
                Err(e) => {
                    print_error(&format!("Failed to fetch templates: {}", e));
                    return Err(e);
                }
            }
        }

        ImageCommands::Create { name, image, tag } => {
            use crate::docker_utils::{
                build_and_push_image, check_docker_available, validate_image_name,
            };
            use std::path::Path;

            // Check if Docker is available
            check_docker_available()?;

            // Validate image name
            validate_image_name(&image)?;

            // Get Docker credentials from config or prompt
            let (docker_user, docker_token) = match config.get_docker_credentials()? {
                Some((user, token)) => (user, token),
                None => {
                    print_info("Docker credentials not found in config.");
                    let user = prompt_input("Docker Hub username:", None)?;
                    let token = prompt_input("Docker Hub token/password:", None)?;

                    let save_creds = prompt_confirm("Save Docker credentials to config?", true)?;
                    if save_creds {
                        let mut config_copy = config.clone();
                        config_copy.set_docker_credentials(&user, &token)?;
                        config_copy.save()?;
                        print_success("Docker credentials saved to config.");
                    }

                    (user, token)
                }
            };

            // Prompt for Dockerfile path
            let dockerfile_path = prompt_input(
                "Path to Dockerfile (or directory containing Dockerfile):",
                Some("./Dockerfile"),
            )?;
            let dockerfile_path = Path::new(&dockerfile_path);

            let dockerfile_path = if dockerfile_path.is_dir() {
                dockerfile_path.join("Dockerfile")
            } else {
                dockerfile_path.to_path_buf()
            };

            if !dockerfile_path.exists() {
                return Err(crate::errors::LiumError::InvalidInput(format!(
                    "Dockerfile not found: {}",
                    dockerfile_path.display()
                )));
            }

            // Build image name with tag
            let full_image_name = if let Some(ref tag) = tag {
                format!("{}:{}", image, tag)
            } else {
                format!("{}:latest", image)
            };

            print_info(&format!("Building and pushing image: {}", full_image_name));

            // Build and push image
            let digest = build_and_push_image(
                &full_image_name,
                &dockerfile_path,
                &docker_user,
                &docker_token,
            )
            .await?;

            // Post image to Lium API
            let client = crate::api::LiumApiClient::from_config()?;
            let tag_to_use = tag.unwrap_or_else(|| "latest".to_string());

            match client.post_image(&image, &digest, &tag_to_use).await {
                Ok(_) => {
                    print_success(&format!(
                        "Image '{}' successfully registered with Lium!",
                        name
                    ));
                    print_info("You can now use this image with 'lium up --image <template_id>'");
                }
                Err(e) => {
                    print_error(&format!("Failed to register image with Lium: {}", e));
                    return Err(e);
                }
            }
        }

        ImageCommands::Delete { id } => {
            print_error("Image deletion not implemented yet");
            print_info(&format!("Would delete template: {}", id));
        }
    }

    Ok(())
}

async fn handle_config(_action: ConfigCommands) -> Result<()> {
    use crate::config::Config;
    use crate::display::{print_error, print_info, print_success, prompt_input, prompt_select};

    match _action {
        ConfigCommands::Show => {
            let config = Config::new()?;
            println!("{}", config.show_config());
        }

        ConfigCommands::Set { key, value } => {
            let mut config = Config::new()?;

            // Parse key into section.key format
            let parts: Vec<&str> = key.split('.').collect();
            let (section, config_key) = match parts.len() {
                1 => {
                    // Handle common single keys by mapping to section.key
                    match parts[0] {
                        "api_key" => ("api", "api_key"),
                        "ssh_user" => ("ssh", "user"),
                        "ssh_key_path" => ("ssh", "key_path"),
                        "default_template" => ("template", "default_id"),
                        "docker_username" => ("docker", "username"),
                        "docker_token" => ("docker", "token"),
                        _ => {
                            print_error(&format!("Unknown config key: {}. Use format 'section.key' or known single keys like 'api_key'", key));
                            return Err(crate::errors::LiumError::InvalidInput(format!(
                                "Unknown config key: {}",
                                key
                            )));
                        }
                    }
                }
                2 => (parts[0], parts[1]),
                _ => {
                    print_error("Config key must be in format 'section.key' or a known single key");
                    return Err(crate::errors::LiumError::InvalidInput(format!(
                        "Invalid config key format: {}",
                        key
                    )));
                }
            };

            config.set_value(section, config_key, &value)?;
            config.save()?;
            print_success(&format!("Config value '{}' set to '{}'", key, value));
        }

        ConfigCommands::Get { key } => {
            let config = Config::new()?;

            // Parse key into section.key format
            let parts: Vec<&str> = key.split('.').collect();
            let (section, config_key) = match parts.len() {
                1 => {
                    // Handle common single keys
                    match parts[0] {
                        "api_key" => ("api", "api_key"),
                        "ssh_user" => ("ssh", "user"),
                        "ssh_key_path" => ("ssh", "key_path"),
                        "default_template" => ("template", "default_id"),
                        "docker_username" => ("docker", "username"),
                        "docker_token" => ("docker", "token"),
                        _ => {
                            print_error(&format!("Unknown config key: {}. Use format 'section.key' or known single keys", key));
                            return Err(crate::errors::LiumError::InvalidInput(format!(
                                "Unknown config key: {}",
                                key
                            )));
                        }
                    }
                }
                2 => (parts[0], parts[1]),
                _ => {
                    print_error("Config key must be in format 'section.key' or a known single key");
                    return Err(crate::errors::LiumError::InvalidInput(format!(
                        "Invalid config key format: {}",
                        key
                    )));
                }
            };

            match config.get_value(section, config_key)? {
                Some(value) => println!("{}: {}", key, value),
                None => println!("{}: not set", key),
            }
        }

        ConfigCommands::Reset => {
            let confirm = crate::display::prompt_confirm(
                "Are you sure you want to reset all configuration to defaults?",
                false,
            )?;

            if confirm {
                // Delete config file to reset to defaults
                let config_dir = home::home_dir()
                    .ok_or_else(|| {
                        crate::errors::LiumError::InvalidInput(
                            "Could not find home directory".to_string(),
                        )
                    })?
                    .join(".lium");
                let config_file = config_dir.join("config.ini");

                if config_file.exists() {
                    std::fs::remove_file(config_file).map_err(crate::errors::LiumError::Io)?;
                    print_success("Configuration reset to defaults");
                } else {
                    print_info("Configuration was already at defaults");
                }
            } else {
                print_info("Configuration reset cancelled");
            }
        }

        ConfigCommands::Init => {
            print_info("Initializing Lium configuration...");

            let mut config = Config::new()?;

            // Get API key
            let api_key = prompt_input("Lium API key (leave empty to skip):", None)?;
            if !api_key.is_empty() {
                config.set_api_key(&api_key)?;
            }

            // Get SSH key path
            let ssh_key_path = prompt_input("SSH public key path:", Some("~/.ssh/id_rsa.pub"))?;
            config.set_ssh_public_key_path(&ssh_key_path)?;

            // Get SSH user
            let ssh_user = prompt_input("SSH user:", Some("root"))?;
            config.set_ssh_user(&ssh_user)?;

            // Try to get templates and set default
            if !api_key.is_empty() {
                let client = crate::api::LiumApiClient::new(api_key.clone(), None);
                match client.get_templates().await {
                    Ok(templates) if !templates.is_empty() => {
                        let template_names: Vec<String> = templates
                            .iter()
                            .map(|t| format!("{} ({})", t.name, t.id))
                            .collect();

                        let selected_idx =
                            prompt_select("Select default template:", &template_names)?;
                        config.set_default_template_id(&templates[selected_idx].id)?;
                    }
                    _ => {
                        print_info("Could not fetch templates, you can set a default later");
                    }
                }
            }

            config.save()?;
            print_success("Configuration initialized successfully");
            print_info("You can modify settings using 'lium config set <key> <value>'");
            print_info("Example: lium config set api.api_key your_key_here");
        }
    }

    Ok(())
}

async fn handle_fund(_action: FundCommands, _config: &Config) -> Result<()> {
    use crate::api::LiumApiClient;
    use crate::display::{print_error, print_info};

    match _action {
        FundCommands::Balance => {
            let client = LiumApiClient::from_config()?;

            print_info("Fetching wallet balance...");

            match client.get_funding_wallets().await {
                Ok(wallets) => {
                    println!("Funding Wallets:");
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&wallets)
                            .unwrap_or_else(|_| "Failed to format wallets".to_string())
                    );
                }
                Err(e) => {
                    print_error(&format!("Failed to fetch wallet balance: {}", e));
                    return Err(e);
                }
            }
        }

        FundCommands::Add { amount } => {
            print_error("Adding funds is not yet implemented");
            print_info(&format!("Would add ${:.2} to wallet", amount));
            print_info("This requires Bittensor integration which is complex in Rust");
            print_info("Please use the web interface to add funds for now");
        }

        FundCommands::History => {
            print_error("Billing history is not yet implemented");
            print_info("This feature requires additional API endpoints");
            print_info("Please use the web interface to view billing history for now");
        }
    }

    Ok(())
}

async fn handle_theme(_action: ThemeCommands, _config: &Config) -> Result<()> {
    use crate::display::{print_error, print_info, print_success};

    match _action {
        ThemeCommands::List => {
            println!("Available themes:");
            let themes = ["default", "dark", "light", "blue", "green", "purple"];
            for (i, theme) in themes.iter().enumerate() {
                println!("{}. {}", i + 1, theme);
            }
        }

        ThemeCommands::Set { name } => {
            let valid_themes = ["default", "dark", "light", "blue", "green", "purple"];

            if valid_themes.contains(&name.as_str()) {
                // TODO: Actually implement theme setting in config
                print_success(&format!("Theme set to '{}'", name));
                print_info("Note: Theme functionality is not fully implemented yet");
                print_info("This would typically modify terminal colors and display preferences");
            } else {
                print_error(&format!("Invalid theme: {}", name));
                print_info("Available themes: default, dark, light, blue, green, purple");
                return Err(crate::errors::LiumError::InvalidInput(format!(
                    "Invalid theme: {}",
                    name
                )));
            }
        }
    }

    Ok(())
}

// TODO: Implement remaining command handlers
// TODO: Add command aliases and shortcuts
// TODO: Add shell completion support
// TODO: Add command history and caching
// TODO: Add batch operations support
