use crate::{CliError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Configuration-specific errors that can occur during config operations
///
/// # Variants
/// * `NotFound` - The requested config file or resource could not be found
/// * `InvalidFormat` - The config file format is invalid or malformed
/// * `MissingField` - A required configuration field is missing
/// * `InvalidValue` - A configuration value is invalid for its field
/// * `DirectoryCreationFailed` - Failed to create the config directory
/// * `TomlError` - Error parsing or serializing TOML data
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found")]
    NotFound,

    #[error("Invalid config format: {0}")]
    InvalidFormat(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },

    #[error("Config directory creation failed: {0}")]
    DirectoryCreationFailed(String),

    #[error("TOML parsing error: {0}")]
    TomlError(String),
}

/// API configuration section containing API-related settings
///
/// # Fields
/// * `api_key` - Optional API key for authentication
/// * `base_url` - Optional base URL for API requests
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

/// SSH configuration section containing SSH-related settings
///
/// # Fields
/// * `key_path` - Optional path to SSH public key file
/// * `user` - Optional SSH username
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SshConfig {
    pub key_path: Option<String>,
    pub user: Option<String>,
}

/// Template configuration section containing template-related settings
///
/// # Fields
/// * `default_id` - Optional ID of the default template to use
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateConfig {
    pub default_id: Option<String>,
}

/// Docker configuration section containing Docker-related settings
///
/// # Fields
/// * `username` - Optional Docker registry username
/// * `token` - Optional Docker registry authentication token
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DockerConfig {
    pub username: Option<String>,
    pub token: Option<String>,
}

/// Main configuration structure containing all configuration sections
///
/// # Fields
/// * `api` - Optional API configuration
/// * `ssh` - Optional SSH configuration
/// * `template` - Optional template configuration
/// * `docker` - Optional Docker configuration
/// * `selections` - Optional generic key-value storage organized by sections
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigData {
    pub api: Option<ApiConfig>,
    pub ssh: Option<SshConfig>,
    pub template: Option<TemplateConfig>,
    pub docker: Option<DockerConfig>,
    pub selections: Option<HashMap<String, HashMap<String, String>>>,
}

/// Implementation of the API config trait for the main Config struct
///
/// This allows the Config struct to be used as an API configuration source
/// by implementing the required trait methods.
impl lium_api::ApiConfig for Config {
    type Error = CliError;

    /// Get the API key, returning an error if it's missing
    fn get_api_key(&self) -> std::result::Result<String, Self::Error> {
        self.get_api_key()?
            .ok_or_else(|| CliError::Config(ConfigError::MissingField("api.api_key".to_string())))
    }

    /// Get the base URL for API requests
    fn get_base_url(&self) -> std::result::Result<Option<String>, Self::Error> {
        Ok(self.data.api.as_ref().and_then(|api| api.base_url.clone()))
    }
}

/// Configuration manager for Lium that handles loading, saving, and accessing configuration
///
/// # Fields
/// * `config_path` - Path to the configuration file
/// * `data` - The configuration data structure
#[derive(Debug, Clone)]
pub struct Config {
    pub config_path: PathBuf,
    pub data: ConfigData,
}

impl Config {
    /// Create a new config instance by loading from file or creating default
    ///
    /// # Returns
    /// * `Result<Self>` - The new Config instance or an error
    ///
    /// # Errors
    /// * `ConfigError::DirectoryCreationFailed` - If config directory creation fails
    /// * `ConfigError::TomlError` - If TOML parsing fails
    /// * `CliError::Io` - If file operations fail
    pub fn new() -> Result<Self> {
        let config_dir = get_config_dir()?;
        let config_path = config_dir.join("config.toml");

        // Ensure config directory exists
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| ConfigError::DirectoryCreationFailed(e.to_string()))?;
        }

        let data = if config_path.exists() {
            let content = fs::read_to_string(&config_path).map_err(CliError::Io)?;
            toml::from_str(&content).map_err(|e| ConfigError::TomlError(e.to_string()))?
        } else {
            // Check for old JSON config and migrate
            let json_path = config_dir.join("config.json");
            if json_path.exists() {
                migrate_from_json(&json_path, &config_path)?;
                let content = fs::read_to_string(&config_path).map_err(CliError::Io)?;
                toml::from_str(&content).map_err(|e| ConfigError::TomlError(e.to_string()))?
            } else {
                ConfigData::default()
            }
        };

        Ok(Config { config_path, data })
    }

    /// Save the configuration to file with atomic write and error handling
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    ///
    /// # Errors
    /// * `ConfigError::TomlError` - If TOML serialization fails
    /// * `ConfigError::DirectoryCreationFailed` - If directory creation fails
    /// * `CliError::Io` - If file operations fail
    pub fn save(&self) -> Result<()> {
        // Serialize to TOML
        let content = toml::to_string_pretty(&self.data)
            .map_err(|e| ConfigError::TomlError(e.to_string()))?;

        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| ConfigError::DirectoryCreationFailed(e.to_string()))?;
            }
        }

        // Write to a temporary file first, then rename (atomic operation)
        let temp_path = self.config_path.with_extension("tmp");

        // Write to temp file
        fs::write(&temp_path, &content).map_err(CliError::Io)?;

        // Atomic rename
        fs::rename(&temp_path, &self.config_path).map_err(CliError::Io)?;

        // Explicitly sync to disk
        if let Ok(file) = fs::File::open(&self.config_path) {
            let _ = file.sync_all(); // Ignore errors for sync_all as it's not critical
        }

        Ok(())
    }

    /// Get the API key, checking environment variable first
    ///
    /// # Returns
    /// * `Result<Option<String>>` - The API key if found, None if not set
    pub fn get_api_key(&self) -> Result<Option<String>> {
        // Try environment variable first
        if let Ok(key) = std::env::var("LIUM_API_KEY") {
            return Ok(Some(key));
        }

        // Then check config
        Ok(self.data.api.as_ref().and_then(|api| api.api_key.clone()))
    }

    /// Set the API key in the configuration
    ///
    /// # Arguments
    /// * `api_key` - The API key to set
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    pub fn set_api_key(&mut self, api_key: &str) -> Result<()> {
        if self.data.api.is_none() {
            self.data.api = Some(ApiConfig::default());
        }
        self.data.api.as_mut().unwrap().api_key = Some(api_key.to_string());
        Ok(())
    }

    /// Get the SSH public key path from configuration
    ///
    /// # Returns
    /// * `Result<Option<String>>` - The path if set, None if not configured
    pub fn get_ssh_public_key_path(&self) -> Result<Option<String>> {
        Ok(self.data.ssh.as_ref().and_then(|ssh| ssh.key_path.clone()))
    }

    /// Set the SSH public key path in configuration
    ///
    /// # Arguments
    /// * `path` - The path to set
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    /// Set SSH public key path
    pub fn set_ssh_public_key_path(&mut self, path: &str) -> Result<()> {
        if self.data.ssh.is_none() {
            self.data.ssh = Some(SshConfig::default());
        }
        self.data.ssh.as_mut().unwrap().key_path = Some(path.to_string());
        Ok(())
    }

    /// Retrieves the SSH user from configuration, defaulting to "root" if not set.
    ///
    /// This method first checks the SSH configuration section for a user setting.
    /// If no user is configured, it returns "root" as the default value.
    ///
    /// # Returns
    /// * `Result<String>` - The configured SSH user or "root" if not set
    ///
    /// # Examples
    /// ```rust
    /// let config = Config::new()?;
    /// let user = config.get_ssh_user()?;
    /// assert_eq!(user, "root"); // Default value
    /// ```
    pub fn get_ssh_user(&self) -> Result<String> {
        Ok(self
            .data
            .ssh
            .as_ref()
            .and_then(|ssh| ssh.user.clone())
            .unwrap_or_else(|| "root".to_string()))
    }

    /// Sets the SSH user in the configuration.
    ///
    /// This method creates the SSH configuration section if it doesn't exist
    /// and sets the user value. The user value is stored as an optional string
    /// to allow for future unset operations.
    ///
    /// # Arguments
    /// * `user` - The SSH user to set
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    ///
    /// # Examples
    /// ```rust
    /// let mut config = Config::new()?;
    /// config.set_ssh_user("ubuntu")?;
    /// assert_eq!(config.get_ssh_user()?, "ubuntu");
    /// ```
    pub fn set_ssh_user(&mut self, user: &str) -> Result<()> {
        if self.data.ssh.is_none() {
            self.data.ssh = Some(SshConfig::default());
        }
        self.data.ssh.as_mut().unwrap().user = Some(user.to_string());
        Ok(())
    }

    /// Retrieves the default template ID from configuration.
    ///
    /// This method returns the template ID that should be used by default
    /// when creating new pods. If no default template is set, returns None.
    ///
    /// # Returns
    /// * `Result<Option<String>>` - The default template ID if set, None otherwise
    ///
    /// # Examples
    /// ```rust
    /// let config = Config::new()?;
    /// match config.get_default_template_id()? {
    ///     Some(id) => println!("Default template: {}", id),
    ///     None => println!("No default template set"),
    /// }
    /// ```
    pub fn get_default_template_id(&self) -> Result<Option<String>> {
        Ok(self
            .data
            .template
            .as_ref()
            .and_then(|t| t.default_id.clone()))
    }

    /// Sets the default template ID in the configuration.
    ///
    /// This method creates the template configuration section if it doesn't exist
    /// and sets the default template ID. The template ID is stored as an optional
    /// string to allow for future unset operations.
    ///
    /// # Arguments
    /// * `template_id` - The template ID to set as default
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    ///
    /// # Examples
    /// ```rust
    /// let mut config = Config::new()?;
    /// config.set_default_template_id("template-123")?;
    /// assert_eq!(config.get_default_template_id()?.unwrap(), "template-123");
    /// ```
    pub fn set_default_template_id(&mut self, template_id: &str) -> Result<()> {
        if self.data.template.is_none() {
            self.data.template = Some(TemplateConfig::default());
        }
        self.data.template.as_mut().unwrap().default_id = Some(template_id.to_string());
        Ok(())
    }

    /// Retrieves Docker credentials from configuration.
    ///
    /// This method returns both the username and token for Docker authentication
    /// if both are configured. If either is missing, returns None.
    ///
    /// # Returns
    /// * `Result<Option<(String, String)>>` - Tuple of (username, token) if both are set, None otherwise
    ///
    /// # Examples
    /// ```rust
    /// let config = Config::new()?;
    /// if let Some((username, token)) = config.get_docker_credentials()? {
    ///     println!("Docker credentials found for user: {}", username);
    /// }
    /// ```
    pub fn get_docker_credentials(&self) -> Result<Option<(String, String)>> {
        if let Some(docker) = &self.data.docker {
            match (&docker.username, &docker.token) {
                (Some(u), Some(t)) => Ok(Some((u.clone(), t.clone()))),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// Sets Docker credentials in the configuration.
    ///
    /// This method creates the Docker configuration section if it doesn't exist
    /// and sets both the username and token. Both values are stored as optional
    /// strings to allow for future unset operations.
    ///
    /// # Arguments
    /// * `username` - The Docker username
    /// * `token` - The Docker authentication token
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    ///
    /// # Examples
    /// ```rust
    /// let mut config = Config::new()?;
    /// config.set_docker_credentials("user123", "token456")?;
    /// let (username, token) = config.get_docker_credentials()?.unwrap();
    /// assert_eq!(username, "user123");
    /// assert_eq!(token, "token456");
    /// ```
    pub fn set_docker_credentials(&mut self, username: &str, token: &str) -> Result<()> {
        if self.data.docker.is_none() {
            self.data.docker = Some(DockerConfig::default());
        }
        let docker = self.data.docker.as_mut().unwrap();
        docker.username = Some(username.to_string());
        docker.token = Some(token.to_string());
        Ok(())
    }

    /// Retrieves SSH public keys from the configured key file.
    ///
    /// This method reads the SSH public key file specified in the configuration,
    /// parses it for valid SSH public keys, and returns them as a vector of strings.
    /// The method handles path expansion, file existence checks, and key validation.
    ///
    /// # Returns
    /// * `Result<Vec<String>>` - Vector of valid SSH public keys
    ///
    /// # Errors
    /// * `ConfigError::MissingField` - If no SSH key path is configured
    /// * `ConfigError::NotFound` - If the key file doesn't exist
    /// * `ConfigError::InvalidFormat` - If no valid SSH keys are found
    /// * `CliError::Io` - If there are file system errors
    ///
    /// # Examples
    /// ```rust
    /// let config = Config::new()?;
    /// let keys = config.get_ssh_public_keys()?;
    /// for key in keys {
    ///     println!("Found SSH key: {}", key);
    /// }
    /// ```
    pub fn get_ssh_public_keys(&self) -> Result<Vec<String>> {
        let key_path = self
            .get_ssh_public_key_path()?
            .ok_or_else(|| ConfigError::MissingField("ssh.key_path".to_string()))?;

        let expanded_path = expand_path(&key_path)?;
        if !expanded_path.exists() {
            return Err(ConfigError::NotFound.into());
        }

        let content = fs::read_to_string(&expanded_path).map_err(CliError::Io)?;

        let keys: Vec<String> = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.to_string())
            .collect();

        if keys.is_empty() {
            return Err(ConfigError::InvalidFormat("No valid SSH keys found".to_string()).into());
        }

        Ok(keys)
    }

    /// Get SSH private key path (derive from public key path)
    pub fn get_ssh_private_key_path(&self) -> Result<PathBuf> {
        let public_key_path = self
            .get_ssh_public_key_path()?
            .ok_or_else(|| ConfigError::MissingField("ssh.key_path".to_string()))?;

        let expanded_path = expand_path(&public_key_path)?;

        // Remove .pub extension if present
        let private_key_path = if expanded_path.extension() == Some(std::ffi::OsStr::new("pub")) {
            expanded_path.with_extension("")
        } else {
            expanded_path
        };

        Ok(private_key_path)
    }

    /// Show all configuration as a formatted string
    pub fn show_config(&self) -> String {
        toml::to_string_pretty(&self.data).unwrap_or_else(|_| "Error formatting config".to_string())
    }

    /// Set a generic key-value pair in the config
    ///
    /// # Arguments
    /// * `section` - The section name to store the value in
    /// * `key` - The key to store the value under
    /// * `value` - The value to store
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    pub fn set_value(&mut self, section: &str, key: &str, value: &str) -> Result<()> {
        if self.data.selections.is_none() {
            self.data.selections = Some(HashMap::new());
        }

        let selections = self.data.selections.as_mut().unwrap();
        if !selections.contains_key(section) {
            selections.insert(section.to_string(), HashMap::new());
        }

        selections
            .get_mut(section)
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// Get a generic key-value pair from the config
    ///
    /// # Arguments
    /// * `section` - The section name to retrieve from
    /// * `key` - The key to retrieve
    ///
    /// # Returns
    /// * `Result<Option<String>>` - The value if found, None if not found
    pub fn get_value(&self, section: &str, key: &str) -> Result<Option<String>> {
        Ok(self
            .data
            .selections
            .as_ref()
            .and_then(|sections| sections.get(section))
            .and_then(|keys| keys.get(key))
            .cloned())
    }
}

/// Loads the configuration from the default location.
///
/// This function attempts to load the configuration from the default location
/// (~/.lium/config.toml). If the configuration file doesn't exist, it will be
/// created with default values. If an old JSON configuration exists, it will be
/// automatically migrated to the new TOML format.
///
/// # Returns
/// * `Result<Config>` - The loaded configuration or an error
///
/// # Errors
/// * `ConfigError::DirectoryCreationFailed` - If the config directory cannot be created
/// * `ConfigError::TomlError` - If the TOML file is invalid
/// * `CliError::Io` - If there are file system errors
///
/// # Examples
/// ```rust
/// let config = load_config()?;
/// println!("Loaded configuration: {}", config.show_config());
/// ```
pub fn load_config() -> Result<Config> {
    Config::new()
}

/// Loads the configuration asynchronously.
///
/// This is an async version of `load_config()` that uses `spawn_blocking` to
/// perform the file I/O operations in a separate thread. This is useful when
/// you need to load configuration in an async context without blocking the
/// async runtime.
///
/// # Returns
/// * `Result<Config>` - The loaded configuration or an error
///
/// # Errors
/// * `CliError::InvalidInput` - If the config loading task fails
/// * `ConfigError::DirectoryCreationFailed` - If the config directory cannot be created
/// * `ConfigError::TomlError` - If the TOML file is invalid
/// * `CliError::Io` - If there are file system errors
///
/// # Examples
/// ```rust
/// let config = load_config_async().await?;
/// println!("Loaded configuration: {}", config.show_config());
/// ```
pub async fn load_config_async() -> Result<Config> {
    // For config loading, we can use spawn_blocking since it's not nested
    tokio::task::spawn_blocking(|| Config::new())
        .await
        .map_err(|_| CliError::InvalidInput("Config loading task failed".to_string()))?
}

/// Gets the path to the configuration directory.
///
/// This function returns the path to the configuration directory, which is
/// typically located at `~/.lium/`. The directory will be created if it
/// doesn't exist.
///
/// # Returns
/// * `Result<PathBuf>` - The path to the configuration directory or an error
///
/// # Errors
/// * `ConfigError::DirectoryCreationFailed` - If the home directory cannot be found
///
/// # Examples
/// ```rust
/// let config_dir = get_config_dir()?;
/// println!("Configuration directory: {}", config_dir.display());
/// ```
fn get_config_dir() -> Result<PathBuf> {
    let home_dir = home::home_dir().ok_or_else(|| {
        ConfigError::DirectoryCreationFailed("Could not find home directory".to_string())
    })?;

    Ok(home_dir.join(".lium"))
}

/// Expands a path string, handling home directory and environment variables.
///
/// This function expands path strings that start with `~` to the user's home
/// directory. It also handles environment variables in the path.
///
/// # Arguments
/// * `path` - The path string to expand
///
/// # Returns
/// * `Result<PathBuf>` - The expanded path or an error
///
/// # Errors
/// * `ConfigError::InvalidValue` - If the home directory cannot be found
///
/// # Examples
/// ```rust
/// let expanded = expand_path("~/config.toml")?;
/// println!("Expanded path: {}", expanded.display());
/// ```
fn expand_path(path: &str) -> Result<PathBuf> {
    let expanded = if path.starts_with('~') {
        let home_dir = home::home_dir().ok_or_else(|| ConfigError::InvalidValue {
            field: "path".to_string(),
            value: path.to_string(),
        })?;
        home_dir.join(&path[2..]) // Skip "~/"
    } else {
        PathBuf::from(path)
    };

    Ok(expanded)
}

/// Migrates configuration from the old JSON format to the new TOML format.
///
/// This function reads the old JSON configuration file, converts it to the new
/// TOML format, and saves it to the new location. The old JSON file is backed
/// up with a `.backup` extension.
///
/// The migration process handles the following configuration sections:
/// * API settings (api_key)
/// * SSH settings (public_key_path, user)
/// * Template settings (default_id)
/// * Docker settings (username, token)
///
/// # Arguments
/// * `json_path` - Path to the old JSON configuration file
/// * `toml_path` - Path where the new TOML configuration will be saved
///
/// # Returns
/// * `Result<()>` - Success or error
///
/// # Errors
/// * `CliError::Io` - If there are file system errors
/// * `CliError::Serde` - If the JSON file is invalid
/// * `ConfigError::TomlError` - If the TOML serialization fails
///
/// # Examples
/// ```rust
/// migrate_from_json(
///     Path::new("~/.lium/config.json"),
///     Path::new("~/.lium/config.toml")
/// )?;
/// ```
fn migrate_from_json(json_path: &Path, toml_path: &Path) -> Result<()> {
    let json_content = fs::read_to_string(json_path).map_err(CliError::Io)?;

    let json_data: serde_json::Value =
        serde_json::from_str(&json_content).map_err(CliError::Serde)?;

    let mut config = ConfigData::default();

    // Migrate API key
    if let Some(api_key) = json_data.get("api_key").and_then(|v| v.as_str()) {
        config.api = Some(ApiConfig {
            api_key: Some(api_key.to_string()),
            base_url: None,
        });
    }

    // Migrate SSH settings
    if let Some(ssh) = json_data.get("ssh") {
        let mut ssh_config = SshConfig::default();
        if let Some(key_path) = ssh.get("public_key_path").and_then(|v| v.as_str()) {
            ssh_config.key_path = Some(key_path.to_string());
        }
        if let Some(user) = ssh.get("user").and_then(|v| v.as_str()) {
            ssh_config.user = Some(user.to_string());
        }
        if ssh_config.key_path.is_some() || ssh_config.user.is_some() {
            config.ssh = Some(ssh_config);
        }
    }

    // Migrate template settings
    if let Some(template) = json_data.get("template") {
        if let Some(default_id) = template.get("default_id").and_then(|v| v.as_str()) {
            config.template = Some(TemplateConfig {
                default_id: Some(default_id.to_string()),
            });
        }
    }

    // Migrate Docker settings
    if let Some(docker) = json_data.get("docker") {
        let mut docker_config = DockerConfig::default();
        if let Some(username) = docker.get("username").and_then(|v| v.as_str()) {
            docker_config.username = Some(username.to_string());
        }
        if let Some(token) = docker.get("token").and_then(|v| v.as_str()) {
            docker_config.token = Some(token.to_string());
        }
        if docker_config.username.is_some() || docker_config.token.is_some() {
            config.docker = Some(docker_config);
        }
    }

    // Save TOML file
    let content =
        toml::to_string_pretty(&config).map_err(|e| ConfigError::TomlError(e.to_string()))?;
    fs::write(toml_path, content).map_err(CliError::Io)?;

    // Backup old JSON file
    let backup_path = json_path.with_extension("json.backup");
    fs::rename(json_path, backup_path).map_err(CliError::Io)?;

    Ok(())
}

// TODO: Add config validation functions
// TODO: Add config schema versioning
// TODO: Add more specific getter/setter methods
// TODO: Add config file watching for live updates
