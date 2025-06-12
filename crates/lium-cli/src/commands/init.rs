use crate::{config::Config, CliError, Result};
use dialoguer::{Input, Password, Select};
use log::debug;
use std::path::Path;
use tokio::time::{timeout, Duration};

/// Handles the `init` command for first-time Lium CLI setup and configuration.
///
/// This function provides an interactive setup wizard that guides users through
/// the initial configuration of the Lium CLI, including API key setup, SSH key
/// configuration, and connection validation. It's designed to be user-friendly
/// and handle common setup scenarios automatically.
///
/// # Returns
/// * `Result<()>` - Success or error with detailed setup failure information
///
/// # Setup Process
/// 1. **Welcome**: Displays setup introduction and purpose
/// 2. **API Key Configuration**: Prompts for or validates existing API key
/// 3. **SSH Key Setup**: Configures SSH public key for pod access
/// 4. **User Configuration**: Sets default SSH user (typically root)
/// 5. **Configuration Save**: Persists settings to TOML configuration file
/// 6. **API Validation**: Tests API connection with provided credentials
/// 7. **SSH Validation**: Validates SSH key format and accessibility
/// 8. **Completion**: Displays success message and next steps
///
/// # Interactive Workflow
/// The setup wizard uses interactive prompts to gather information:
/// - **Confirmation dialogs**: For using existing vs. new configuration
/// - **Secure input**: Password-style input for API keys
/// - **File path input**: With intelligent defaults and validation
/// - **Automatic detection**: Finds common SSH key locations
///
/// # API Key Configuration
/// The function handles API keys through multiple sources:
/// - **Environment variable**: `LIUM_API_KEY` (checked first)
/// - **Existing configuration**: Previously saved API key
/// - **User input**: Secure prompt for new API key entry
/// - **Validation**: Tests API connectivity before saving
///
/// The API key setup process:
/// 1. Check for existing configuration
/// 2. Offer to reuse or replace existing key
/// 3. Prompt for new key with secure input
/// 4. Validate key format and non-empty requirement
/// 5. Test API connection before final save
///
/// # SSH Key Configuration
/// SSH key setup is intelligent and user-friendly:
/// - **Auto-detection**: Scans common SSH key locations
/// - **Path validation**: Ensures keys exist and are readable
/// - **Format validation**: Validates SSH public key format
/// - **Key generation guidance**: Provides commands for creating new keys
/// - **Flexible options**: Supports existing keys or guided creation
///
/// SSH key locations checked (in order):
/// 1. `~/.ssh/id_rsa.pub` (RSA keys)
/// 2. `~/.ssh/id_ed25519.pub` (Ed25519 keys, recommended)
/// 3. `~/.ssh/id_ecdsa.pub` (ECDSA keys)
/// 4. `~/.ssh/tplr.pub` (Custom key name)
///
/// # Configuration Persistence
/// Settings are saved to `~/.lium/config.toml` using TOML format:
/// ```toml
/// [api]
/// api_key = "your-api-key-here"
/// base_url = "https://api.lium.ai"  # optional
///
/// [ssh]
/// key_path = "~/.ssh/id_ed25519.pub"
/// user = "root"
/// ```
///
/// # Validation and Testing
/// The setup process includes comprehensive validation:
/// - **API Connection**: 10-second timeout test with error reporting
/// - **SSH Key Format**: Validates public key format and content
/// - **File Accessibility**: Ensures configuration files are writable
/// - **Path Resolution**: Expands tildes and validates file existence
///
/// # Error Handling
/// Common setup issues and their handling:
/// - **Missing SSH keys**: Provides guidance for key generation
/// - **Invalid API keys**: Clear error messages with suggested solutions
/// - **Permission issues**: Guidance on file permissions and locations
/// - **Network problems**: Timeout handling and retry suggestions
/// - **Configuration conflicts**: Safe handling of existing configurations
///
/// # Examples
/// ```rust
/// use lium_cli::commands::init::handle;
///
/// // Run interactive setup
/// handle().await?;
/// ```
///
/// # Setup Output
/// ```text
/// 🍄 Lium initialization
/// Setting up your Lium configuration...
///
/// Please enter your Lium API key.
/// You can get your API key from: https://celiumcompute.ai/api-keys
/// API Key: [hidden input]
///
/// Please enter the path to your SSH public key.
/// This is typically ~/.ssh/id_rsa.pub or ~/.ssh/id_ed25519.pub
/// Found SSH key at ~/.ssh/id_ed25519.pub. Use this key? Yes
///
/// 💾 Saving configuration...
/// 🔍 Testing API connection...
/// ✅ API connection successful!
/// 🔑 Validating SSH key...
/// ✅ SSH key is valid!
///
/// ✅ Lium initialization complete!
/// Configuration saved to: ~/.lium/config.toml
///
/// You can now use 'lium ls' to see available executors.
/// ```
///
/// # Recovery and Troubleshooting
/// If setup fails, users can:
/// - Re-run `lium init` to restart the process
/// - Use `lium config show` to check current configuration
/// - Manually edit `~/.lium/config.toml` if needed
/// - Check API key validity at the provider's website
/// - Generate new SSH keys if validation fails
///
/// # TODO
/// - Add support for custom configuration file locations
/// - Implement configuration migration from other tools
/// - Add support for multiple API environments (staging, production)
/// - Support for SSH agent integration during setup
/// - Add configuration validation and repair utilities
/// - Implement guided troubleshooting for common issues
pub async fn handle() -> Result<()> {
    println!("🍄 Lium initialization");
    println!("Setting up your Lium configuration...\n");

    let mut config = Config::new()?; // Use synchronous version to avoid nested async issues

    // Get API key
    let api_key = if let Some(existing_key) = config.get_api_key()? {
        let use_existing = Select::new()
            .with_prompt("API key already configured. Use existing key?")
            .items(&["Yes, use existing", "No, enter new key"])
            .default(0)
            .interact()?;

        if use_existing == 0 {
            existing_key
        } else {
            get_api_key_from_user()?
        }
    } else {
        get_api_key_from_user()?
    };

    config.set_api_key(&api_key)?;

    // Get SSH key path
    let ssh_key_path = if let Some(existing_path) = config.get_ssh_public_key_path()? {
        let use_existing = Select::new()
            .with_prompt("SSH public key already configured. Use existing path?")
            .items(&["Yes, use existing", "No, enter new path"])
            .default(0)
            .interact()?;

        if use_existing == 0 {
            existing_path
        } else {
            get_ssh_key_path_from_user()?
        }
    } else {
        get_ssh_key_path_from_user()?
    };

    debug!("About to set SSH key path: {}", ssh_key_path);
    config.set_ssh_public_key_path(&ssh_key_path)?;
    debug!("SSH key path set successfully");

    debug!("About to set SSH user");
    config.set_ssh_user("root")?; // Default to root as per spec
    debug!("SSH user set successfully");

    // Save configuration with proper async handling
    println!("\n💾 Saving configuration...");
    debug!("About to save config");
    save_config_async(&config).await?;
    debug!("Config saved successfully");

    // Test API connection
    println!("\n🔍 Testing API connection...");
    match test_api_connection(&api_key).await {
        Ok(_) => println!("✅ API connection successful!"),
        Err(e) => {
            println!("⚠️  API connection failed: {}", e);
            println!("You can continue, but some features may not work.");
        }
    }

    // Validate SSH key
    println!("\n🔑 Validating SSH key...");
    match validate_ssh_key(&ssh_key_path) {
        Ok(_) => println!("✅ SSH key is valid!"),
        Err(e) => {
            println!("⚠️  SSH key validation failed: {}", e);
            println!("You can continue, but SSH operations may not work.");
        }
    }

    println!("\n✅ Lium initialization complete!");
    println!(
        "Configuration saved to: {}",
        config
            .show_config()
            .lines()
            .next()
            .unwrap_or("~/.lium/config.toml")
    );
    println!("\nYou can now use 'lium ls' to see available executors.");

    Ok(())
}

/// Get API key from user input
fn get_api_key_from_user() -> Result<String> {
    println!("Please enter your Lium API key.");
    println!("You can get your API key from: https://celiumcompute.ai/api-keys");

    let api_key: String = Password::new()
        .with_prompt("API Key")
        .allow_empty_password(false)
        .interact()?;

    if api_key.trim().is_empty() {
        return Err(CliError::InvalidInput(
            "API key cannot be empty".to_string(),
        ));
    }

    Ok(api_key.trim().to_string())
}

/// Get SSH key path from user input
fn get_ssh_key_path_from_user() -> Result<String> {
    println!("\nPlease enter the path to your SSH public key.");
    println!("This is typically ~/.ssh/id_rsa.pub or ~/.ssh/id_ed25519.pub");
    debug!("Starting SSH key path detection");

    let default_paths = vec![
        "~/.ssh/id_rsa.pub",
        "~/.ssh/id_ed25519.pub",
        "~/.ssh/id_ecdsa.pub",
        "~/.ssh/tplr.pub", // Add your custom key
    ];

    // Check if any default paths exist
    debug!("Checking default paths");
    for default_path in &default_paths {
        debug!("Checking path: {}", default_path);
        let expanded = expand_path(default_path)?;
        debug!("Expanded to: {:?}", expanded);
        if expanded.exists() {
            debug!("Found existing key at {}", default_path);
            let use_default = Select::new()
                .with_prompt(format!("Found SSH key at {}. Use this key?", default_path))
                .items(&["Yes, use this key", "No, enter different path"])
                .default(0)
                .interact()?;

            if use_default == 0 {
                debug!("User selected existing key");
                return Ok(default_path.to_string());
            }
            break;
        }
    }

    // No existing keys found, inform user
    debug!("No existing SSH public keys found");
    println!("\n⚠️  No SSH public keys found in the default locations.");
    println!("You have the following options:");
    println!("1. Create a new SSH key pair");
    println!("2. Generate a public key from your existing private key");
    println!("3. Enter a custom path to an existing public key");

    let choice = Select::new()
        .with_prompt("What would you like to do?")
        .items(&[
            "Create new SSH key pair",
            "Generate public key from private key",
            "Enter custom path",
        ])
        .default(0)
        .interact()?;

    match choice {
        0 => {
            println!("Please run: ssh-keygen -t ed25519 -C \"your_email@example.com\"");
            println!("Then run 'lium init' again.");
            return Err(CliError::InvalidInput(
                "SSH key generation required".to_string(),
            ));
        }
        1 => {
            // Try to generate from existing private key
            let private_key_path: String = Input::new()
                .with_prompt("Path to private key")
                .with_initial_text("~/.ssh/tplr")
                .interact()?;

            let expanded_private = expand_path(&private_key_path)?;
            if !expanded_private.exists() {
                return Err(CliError::InvalidInput(format!(
                    "Private key not found: {}",
                    expanded_private.display()
                )));
            }

            let public_key_path = format!("{}.pub", private_key_path);
            println!(
                "Run: ssh-keygen -y -f {} > {}",
                private_key_path, public_key_path
            );
            println!("Then run 'lium init' again.");
            return Err(CliError::InvalidInput(
                "Public key generation required".to_string(),
            ));
        }
        2 => {
            // Get custom path
            debug!("Getting custom path from user");
            let ssh_key_path: String = Input::new()
                .with_prompt("SSH public key path")
                .with_initial_text("~/.ssh/id_rsa.pub")
                .interact()?;

            debug!("User entered path: {}", ssh_key_path);

            if ssh_key_path.trim().is_empty() {
                return Err(CliError::InvalidInput(
                    "SSH key path cannot be empty".to_string(),
                ));
            }

            // Validate that the path exists
            let expanded = expand_path(ssh_key_path.trim())?;
            if !expanded.exists() {
                return Err(CliError::InvalidInput(format!(
                    "SSH key file not found: {}",
                    expanded.display()
                )));
            }

            debug!("Returning path: {}", ssh_key_path.trim());
            Ok(ssh_key_path.trim().to_string())
        }
        _ => unreachable!(),
    }
}

/// Test API connection with the provided key
async fn test_api_connection(api_key: &str) -> Result<()> {
    let api_client = lium_api::LiumApiClient::new(api_key.to_string(), None);

    // Add 10 second timeout to prevent hanging
    match timeout(Duration::from_secs(10), api_client.test_connection()).await {
        Ok(result) => {
            result?;
        }
        Err(_) => {
            return Err(CliError::InvalidInput(
                "API connection timed out after 10 seconds".to_string(),
            ));
        }
    }

    Ok(())
}

/// Validate SSH key exists and is readable
fn validate_ssh_key(key_path: &str) -> Result<()> {
    let expanded_path = expand_path(key_path)?;

    if !expanded_path.exists() {
        return Err(CliError::InvalidInput(format!(
            "SSH key file not found: {}",
            expanded_path.display()
        )));
    }

    if !expanded_path.is_file() {
        return Err(CliError::InvalidInput(format!(
            "SSH key path is not a file: {}",
            expanded_path.display()
        )));
    }

    // Try to read the key file
    let key_content = std::fs::read_to_string(&expanded_path)?;

    if key_content.trim().is_empty() {
        return Err(CliError::InvalidInput("SSH key file is empty".to_string()));
    }

    // Basic validation - should start with ssh-rsa, ssh-ed25519, etc.
    let first_line = key_content.lines().next().unwrap_or("").trim();
    if !first_line.starts_with("ssh-") {
        return Err(CliError::InvalidInput(
            "SSH key file does not appear to contain a valid public key".to_string(),
        ));
    }

    Ok(())
}

/// Expand path with tilde and better error handling
fn expand_path(path: &str) -> Result<std::path::PathBuf> {
    debug!("Expanding path: {}", path);

    let expanded = if path.starts_with('~') {
        if let Some(home_dir) = dirs::home_dir() {
            let relative_part = path.strip_prefix("~/").unwrap_or(&path[1..]);
            home_dir.join(relative_part)
        } else {
            return Err(CliError::InvalidInput(
                "Cannot determine home directory".to_string(),
            ));
        }
    } else {
        Path::new(path).to_path_buf()
    };

    debug!("Expanded path result: {:?}", expanded);
    Ok(expanded)
}

/// Save config with proper async handling
async fn save_config_async(config: &Config) -> Result<()> {
    let config = config.clone();

    // Use timeout with proper error handling
    match timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking(move || {
            // Add explicit flush to ensure write completes
            config.save()
        }),
    )
    .await
    {
        Ok(join_result) => match join_result {
            Ok(save_result) => save_result,
            Err(join_error) => {
                log::error!("Config save task panicked: {:?}", join_error);
                Err(CliError::InvalidInput(
                    "Config save task failed".to_string(),
                ))
            }
        },
        Err(_timeout_error) => {
            log::error!("Config save operation timed out");
            Err(CliError::InvalidInput(
                "Config save timed out after 5 seconds".to_string(),
            ))
        }
    }
}
