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
    /// Execute command in pod
    Exec {
        /// Pod HUID or index
        pod: String,
        /// Command to execute
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// SSH into pod
    Ssh {
        /// Pod HUID or index
        pod: String,
    },
    /// Copy files to/from pod
    Scp {
        /// Source path
        source: String,
        /// Destination path
        destination: String,
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
    /// Stop and remove pod
    Down {
        /// Pod HUID or index
        pod: String,
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
        Commands::Exec { pod, command } => handle_exec(pod, command, &config).await,
        Commands::Ssh { pod } => handle_ssh(pod, &config).await,
        Commands::Scp {
            source,
            destination,
        } => handle_scp(source, destination, &config).await,
        Commands::Rsync {
            source,
            destination,
            options,
        } => handle_rsync(source, destination, options, &config).await,
        Commands::Down { pod, yes } => handle_down(pod, yes, &config).await,
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

async fn handle_exec(pod: String, command: Vec<String>, config: &Config) -> Result<()> {
    use crate::api::LiumApiClient;
    use crate::display::{print_error, print_info};
    use crate::helpers::{extract_ssh_details, resolve_single_pod_target};
    use crate::ssh_utils;

    if command.is_empty() {
        return Err(crate::errors::LiumError::InvalidInput(
            "No command specified".to_string(),
        ));
    }

    let client = LiumApiClient::from_config()?;

    // Resolve the pod
    let pod_info = resolve_single_pod_target(&client, &pod).await?;

    print_info(&format!("Executing command on pod: {}", pod_info.huid));

    // Extract SSH connection details
    let (host, port, user) = extract_ssh_details(&pod_info)?;
    let private_key_path = config.get_ssh_private_key_path()?;

    // Join command arguments
    let command_str = command.join(" ");

    // Execute the command
    let (_stdout, _stderr, exit_code) = ssh_utils::execute_remote_command(
        &host,
        port,
        &user,
        &private_key_path,
        &command_str,
        None, // TODO: Add support for --env flags
    )
    .await?;

    if exit_code != 0 {
        print_error(&format!("Command failed with exit code: {}", exit_code));
    }

    // Note: stdout/stderr are already streamed during execution
    std::process::exit(exit_code);
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

async fn handle_scp(source: String, destination: String, config: &Config) -> Result<()> {
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
    let private_key_path = config.get_ssh_private_key_path()?;

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

async fn handle_down(pod: String, yes: bool, _config: &Config) -> Result<()> {
    use crate::api::LiumApiClient;
    use crate::display::{print_info, print_success, prompt_confirm};
    use crate::helpers::resolve_single_pod_target;

    let client = LiumApiClient::from_config()?;

    // Resolve the pod
    let pod_info = resolve_single_pod_target(&client, &pod).await?;

    print_info(&format!(
        "Stopping pod: {} ({})",
        pod_info.huid, pod_info.name
    ));

    if !yes {
        let confirm = prompt_confirm(
            &format!("Are you sure you want to stop pod '{}'?", pod_info.huid),
            false,
        )?;

        if !confirm {
            print_info("Operation cancelled.");
            return Ok(());
        }
    }

    // Get executor ID from pod info
    let executor_id = pod_info
        .executor
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or(&pod_info.id); // Fallback to pod ID if executor ID not found

    match client.unrent_pod(executor_id).await {
        Ok(_) => {
            print_success(&format!("Successfully stopped pod: {}", pod_info.huid));
        }
        Err(e) => {
            return Err(crate::errors::LiumError::OperationFailed(format!(
                "Failed to stop pod: {}",
                e
            )));
        }
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
                    std::fs::remove_file(config_file)
                        .map_err(|e| crate::errors::LiumError::Io(e))?;
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
