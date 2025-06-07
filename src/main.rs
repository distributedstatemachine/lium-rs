use clap::{Parser, Subcommand};
use lium_rs::cli::run_cli;
use lium_rs::display::print_error;

#[derive(Parser)]
#[command(name = "lium")]
#[command(about = "Lium CLI for managing Celium Compute GPU pods")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Lium configuration
    Init,
    /// List available executors  
    Ls {
        /// GPU type to filter by
        gpu_type: Option<String>,
    },
    /// Start a pod
    Up {
        /// Executor targets (HUIDs, UUIDs, or indices)
        targets: Vec<String>,
        /// Pod name prefix
        #[arg(long)]
        prefix: Option<String>,
        /// Template ID
        #[arg(long)]
        image: Option<String>,
        /// Skip confirmations
        #[arg(short = 'y')]
        yes: bool,
    },
    /// List active pods
    Ps {
        /// Pod targets to show details for
        targets: Vec<String>,
    },
    /// Execute command on pods
    Exec {
        /// Pod targets
        targets: Vec<String>,
        /// Command to execute
        command: String,
        /// Script file to execute
        #[arg(long)]
        script: Option<String>,
        /// Environment variables (KEY=VALUE)
        #[arg(long)]
        env: Vec<String>,
    },
    /// SSH into a pod
    Ssh {
        /// Pod target
        target: String,
    },
    /// Copy files to/from pods
    Scp {
        /// Pod targets or source
        targets: Vec<String>,
        /// Local path or remote path
        path: String,
        /// Remote path (if copying to pod)
        remote_path: Option<String>,
    },
    /// Sync directories with pods
    Rsync {
        /// Source path
        source: String,
        /// Destination path
        destination: String,
        /// Additional rsync options
        options: Vec<String>,
    },
    /// Stop pods
    Down {
        /// Pod targets to stop
        targets: Vec<String>,
        /// Stop all pods
        #[arg(long)]
        all: bool,
        /// Skip confirmations
        #[arg(short = 'y')]
        yes: bool,
    },
    /// Build and push Docker image
    Image {
        /// Image name
        image_name: String,
        /// Path to Dockerfile directory
        dockerfile_path: String,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands,
    },
    /// Fund account with TAO
    Fund {
        /// Wallet name
        #[arg(long)]
        wallet: String,
        /// Amount of TAO to fund
        #[arg(long)]
        tao: f64,
    },
    /// Set theme
    Theme {
        /// Theme name
        theme_name: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Get configuration value
    Get {
        /// Section name
        section: String,
        /// Key name
        key: String,
    },
    /// Set configuration value
    Set {
        /// Section name
        section: String,
        /// Key name
        key: String,
        /// Value to set
        value: String,
    },
    /// Remove configuration value
    Unset {
        /// Section name
        section: String,
        /// Key name
        key: String,
    },
    /// Show configuration file path
    Path,
}

#[tokio::main]
async fn main() {
    match run_cli().await {
        Ok(()) => {
            // Success - no additional output needed
        }
        Err(e) => {
            print_error(&format!("Error: {}", e));
            std::process::exit(1);
        }
    }
}
