//! # Lium CLI
//!
//! Command-line interface for Lium (Celium Compute).
//! This crate provides the CLI structure, argument parsing, and command routing.

// Re-export all modules
pub mod commands;
pub mod config;
pub mod display;
pub mod helpers;
pub mod resolvers;
pub mod storage;

// Re-export common types
pub use config::Config;

use clap::{Parser, Subcommand};
use thiserror::Error;

/// Application-level errors for the CLI
#[derive(Error, Debug)]
pub enum CliError {
    #[error("Core domain error: {0}")]
    Core(#[from] lium_core::LiumError),

    #[error("API error: {0}")]
    Api(#[from] lium_api::ApiError),

    #[error("Utils error: {0}")]
    Utils(#[from] lium_utils::UtilsError),

    #[error("Config error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Command failed: {0}")]
    Command(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("Other: {0}")]
    Other(String),
}

impl From<dialoguer::Error> for CliError {
    fn from(err: dialoguer::Error) -> Self {
        CliError::Other(format!("Input error: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, CliError>;

/// Main CLI struct
#[derive(Parser)]
#[command(name = "lium")]
#[command(about = "A CLI tool for Celium Compute")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// All available CLI commands
#[derive(Subcommand)]
pub enum Commands {
    /// Initialize configuration
    Init,
    /// List available executors
    Ls(commands::ls::LsArgs),
    /// Start a new pod
    Up(commands::up::UpArgs),
    /// List active pods
    Ps(commands::ps::PsArgs),
    /// Execute command in pod(s)
    #[command(
        trailing_var_arg = true,
        about = "Execute a command on one or more running pods via SSH",
        long_about = "Execute a command on one or more running pods via SSH.\n\n\
        Examples:\n  \
        lium exec 1 ls -la\n  \
        lium exec 1,2,3 nvidia-smi\n  \
        lium exec all uptime\n  \
        lium exec 1 --script script.py\n  \
        lium exec 3 --env API_KEY=secret echo \\$API_KEY"
    )]
    Exec {
        /// Arguments: <POD_TARGETS> [OPTIONS] [COMMAND...]
        #[arg(value_name = "ARGS")]
        args: Vec<String>,
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
    #[command(
        trailing_var_arg = true,
        about = "Sync files with pod using rsync",
        long_about = "Sync directories with pod(s) using rsync.\n\n\
        Examples:\n  \
        lium rsync ~/project/ 1,2:/home/project/ -v\n  \
        lium rsync 1:/home/project/ ~/backup/ -z\n  \
        lium rsync all:/home/logs/ ~/collected/\n  \
        lium rsync ~/data/ all:/workspace/ --delete --exclude '*.tmp'"
    )]
    Rsync {
        /// Arguments: <SOURCE> <DESTINATION> [OPTIONS...]
        #[arg(value_name = "ARGS")]
        args: Vec<String>,
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
    #[command(
        about = "Build and manage Docker images",
        long_about = "Build Docker images and register them with Lium.\n\n\
        Examples:\n  \
        lium image my_image .\n  \
        lium image my_app ./app -f Dockerfile.prod\n  \
        lium image list\n  \
        lium image create my-template ubuntu:22.04\n  \
        lium image delete <id>"
    )]
    Image {
        #[command(subcommand)]
        action: Option<ImageCommands>,
        /// Image name (when building)
        #[arg(value_name = "IMAGE_NAME", required_unless_present = "action")]
        image_name: Option<String>,
        /// Build path (when building)
        #[arg(value_name = "PATH", required_unless_present = "action")]
        path: Option<String>,
        /// Dockerfile location
        #[arg(short = 'f', long, value_name = "DOCKERFILE")]
        dockerfile: Option<String>,
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

/// Main CLI runner - clean routing without massive handlers
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::new()?;

    match cli.command {
        Commands::Init => commands::init::handle().await,
        Commands::Ls(args) => commands::ls::handle(args, &config).await,
        Commands::Up(args) => commands::up::handle(args, &config).await,
        Commands::Ps(args) => commands::ps::handle(args, &config).await,
        Commands::Exec { args } => {
            // Manually parse the exec arguments
            if args.is_empty() {
                return Err(CliError::InvalidInput(
                    "No pod targets specified".to_string(),
                ));
            }

            let pod_targets = args[0].clone();
            let mut command = Vec::new();
            let mut script = None;
            let mut env = Vec::new();

            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--script" | "-s" => {
                        if i + 1 < args.len() {
                            script = Some(args[i + 1].clone());
                            i += 2;
                        } else {
                            return Err(CliError::InvalidInput(
                                "--script requires a value".to_string(),
                            ));
                        }
                    }
                    "--env" | "-e" => {
                        if i + 1 < args.len() {
                            env.push(args[i + 1].clone());
                            i += 2;
                        } else {
                            return Err(CliError::InvalidInput(
                                "--env requires a value".to_string(),
                            ));
                        }
                    }
                    _ => {
                        // Everything else is part of the command
                        command.extend_from_slice(&args[i..]);
                        break;
                    }
                }
            }

            let exec_args = commands::exec::ExecArgs {
                pod_targets,
                command,
                script,
                env,
            };

            commands::exec::handle(exec_args, &config).await
        }
        Commands::Ssh { pod } => commands::ssh::handle(pod, &config).await,
        Commands::Scp {
            source,
            destination,
            coldkey,
            hotkey,
        } => commands::scp::handle(source, destination, coldkey, hotkey, &config).await,
        Commands::Rsync { args } => {
            // Manually parse rsync arguments
            if args.len() < 2 {
                return Err(CliError::InvalidInput(
                    "Rsync requires source and destination arguments".to_string(),
                ));
            }

            let source = args[0].clone();
            let destination = args[1].clone();
            let options = args[2..].to_vec();

            commands::rsync::handle(source, destination, options, &config).await
        }
        Commands::Down { pods, all, yes } => commands::down::handle(pods, all, yes, &config).await,
        Commands::Image {
            action,
            image_name,
            path,
            dockerfile,
        } => {
            match action {
                Some(cmd) => commands::image::handle_subcommand(cmd, &config).await,
                None => {
                    // Build image mode
                    if let (Some(name), Some(build_path)) = (image_name, path) {
                        commands::image::handle_build(name, build_path, dockerfile, &config).await
                    } else {
                        Err(CliError::InvalidInput(
                            "Image name and path are required for building".to_string(),
                        ))
                    }
                }
            }
        }
        Commands::Config { action } => commands::config::handle(action, &config).await,
        Commands::Fund { action } => commands::fund::handle(action, &config).await,
        Commands::Theme { action } => commands::theme::handle(action, &config).await,
    }
}

// TODO: Add command aliases and shortcuts
// TODO: Add shell completion support
// TODO: Add command history and caching
// TODO: Add batch operations support
