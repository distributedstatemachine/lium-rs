use crate::{config::Config, CliError, Result};
use dialoguer::{Input, Password, Select};
use std::path::Path;

/// Handle the init command for first-time setup
pub async fn handle() -> Result<()> {
    println!("🚀 Lium initialization");
    println!("Setting up your Lium configuration...\n");

    let mut config = Config::new()?;

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

    config.set_ssh_public_key_path(&ssh_key_path)?;
    config.set_ssh_user("root")?; // Default to root as per spec

    // Save configuration
    config.save()?;

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
            .unwrap_or("~/.lium/config.ini")
    );
    println!("\nYou can now use 'lium ls' to see available executors.");

    Ok(())
}

/// Get API key from user input
fn get_api_key_from_user() -> Result<String> {
    println!("Please enter your Lium API key.");
    println!("You can get your API key from: https://celium.ai/dashboard/api-keys");

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

    let default_paths = vec![
        "~/.ssh/id_rsa.pub",
        "~/.ssh/id_ed25519.pub",
        "~/.ssh/id_ecdsa.pub",
    ];

    // Check if any default paths exist
    for default_path in &default_paths {
        let expanded = expand_path(default_path)?;
        if expanded.exists() {
            let use_default = Select::new()
                .with_prompt(format!("Found SSH key at {}. Use this key?", default_path))
                .items(&["Yes, use this key", "No, enter different path"])
                .default(0)
                .interact()?;

            if use_default == 0 {
                return Ok(default_path.to_string());
            }
            break;
        }
    }

    // Get custom path
    let ssh_key_path: String = Input::new()
        .with_prompt("SSH public key path")
        .with_initial_text("~/.ssh/id_rsa.pub")
        .interact()?;

    if ssh_key_path.trim().is_empty() {
        return Err(CliError::InvalidInput(
            "SSH key path cannot be empty".to_string(),
        ));
    }

    Ok(ssh_key_path.trim().to_string())
}

/// Test API connection with the provided key
async fn test_api_connection(api_key: &str) -> Result<()> {
    let api_client = lium_api::LiumApiClient::new(api_key.to_string(), None);
    api_client.test_connection().await?;
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

/// Expand path with tilde
fn expand_path(path: &str) -> Result<std::path::PathBuf> {
    if path.starts_with('~') {
        if let Some(home_dir) = dirs::home_dir() {
            Ok(home_dir.join(path.strip_prefix("~/").unwrap_or(&path[1..])))
        } else {
            Err(CliError::InvalidInput(
                "Cannot determine home directory".to_string(),
            ))
        }
    } else {
        Ok(Path::new(path).to_path_buf())
    }
}
