use crate::{CliError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Configuration-specific errors
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

/// API configuration section
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

/// SSH configuration section
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SshConfig {
    pub key_path: Option<String>,
    pub user: Option<String>,
}

/// Template configuration section
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateConfig {
    pub default_id: Option<String>,
}

/// Docker configuration section
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DockerConfig {
    pub username: Option<String>,
    pub token: Option<String>,
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigData {
    pub api: Option<ApiConfig>,
    pub ssh: Option<SshConfig>,
    pub template: Option<TemplateConfig>,
    pub docker: Option<DockerConfig>,
}

/// Implement the API config trait for our main Config struct
impl lium_api::ApiConfig for Config {
    type Error = CliError;

    fn get_api_key(&self) -> std::result::Result<String, Self::Error> {
        self.get_api_key()?
            .ok_or_else(|| CliError::Config(ConfigError::MissingField("api.api_key".to_string())))
    }

    fn get_base_url(&self) -> std::result::Result<Option<String>, Self::Error> {
        Ok(self.data.api.as_ref().and_then(|api| api.base_url.clone()))
    }
}

/// Configuration manager for Lium
#[derive(Debug, Clone)]
pub struct Config {
    config_path: PathBuf,
    data: ConfigData,
}

impl Config {
    /// Create a new config instance
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

    /// Save the configuration to file
    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(&self.data)
            .map_err(|e| ConfigError::TomlError(e.to_string()))?;
        fs::write(&self.config_path, content).map_err(CliError::Io)?;
        Ok(())
    }

    /// Get API key
    pub fn get_api_key(&self) -> Result<Option<String>> {
        // Try environment variable first
        if let Ok(key) = std::env::var("LIUM_API_KEY") {
            return Ok(Some(key));
        }

        // Then check config
        Ok(self.data.api.as_ref().and_then(|api| api.api_key.clone()))
    }

    /// Set API key
    pub fn set_api_key(&mut self, api_key: &str) -> Result<()> {
        if self.data.api.is_none() {
            self.data.api = Some(ApiConfig::default());
        }
        self.data.api.as_mut().unwrap().api_key = Some(api_key.to_string());
        Ok(())
    }

    /// Get SSH public key path
    pub fn get_ssh_public_key_path(&self) -> Result<Option<String>> {
        Ok(self.data.ssh.as_ref().and_then(|ssh| ssh.key_path.clone()))
    }

    /// Set SSH public key path
    pub fn set_ssh_public_key_path(&mut self, path: &str) -> Result<()> {
        if self.data.ssh.is_none() {
            self.data.ssh = Some(SshConfig::default());
        }
        self.data.ssh.as_mut().unwrap().key_path = Some(path.to_string());
        Ok(())
    }

    /// Get SSH user
    pub fn get_ssh_user(&self) -> Result<String> {
        Ok(self
            .data
            .ssh
            .as_ref()
            .and_then(|ssh| ssh.user.clone())
            .unwrap_or_else(|| "root".to_string()))
    }

    /// Set SSH user
    pub fn set_ssh_user(&mut self, user: &str) -> Result<()> {
        if self.data.ssh.is_none() {
            self.data.ssh = Some(SshConfig::default());
        }
        self.data.ssh.as_mut().unwrap().user = Some(user.to_string());
        Ok(())
    }

    /// Get default template ID
    pub fn get_default_template_id(&self) -> Result<Option<String>> {
        Ok(self
            .data
            .template
            .as_ref()
            .and_then(|t| t.default_id.clone()))
    }

    /// Set default template ID
    pub fn set_default_template_id(&mut self, template_id: &str) -> Result<()> {
        if self.data.template.is_none() {
            self.data.template = Some(TemplateConfig::default());
        }
        self.data.template.as_mut().unwrap().default_id = Some(template_id.to_string());
        Ok(())
    }

    /// Get Docker credentials
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

    /// Set Docker credentials
    pub fn set_docker_credentials(&mut self, username: &str, token: &str) -> Result<()> {
        if self.data.docker.is_none() {
            self.data.docker = Some(DockerConfig::default());
        }
        let docker = self.data.docker.as_mut().unwrap();
        docker.username = Some(username.to_string());
        docker.token = Some(token.to_string());
        Ok(())
    }

    /// Get SSH public keys from file
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

    /// Set a generic key-value pair (used for storing selection data)
    /// TODO: Implement proper key-value storage
    pub fn set_value(&mut self, _section: &str, _key: &str, _value: &str) -> Result<()> {
        // For now, we'll just ignore these calls
        // In a full implementation, we'd store these in a separate section of the TOML
        Ok(())
    }

    /// Get a generic key-value pair (used for retrieving selection data)
    /// TODO: Implement proper key-value storage
    pub fn get_value(&self, _section: &str, _key: &str) -> Result<Option<String>> {
        // For now, we'll just return None (no stored data)
        // In a full implementation, we'd retrieve these from a separate section of the TOML
        Ok(None)
    }
}

/// Load configuration
pub fn load_config() -> Result<Config> {
    Config::new()
}

/// Get configuration directory path
fn get_config_dir() -> Result<PathBuf> {
    let home_dir = home::home_dir().ok_or_else(|| {
        ConfigError::DirectoryCreationFailed("Could not find home directory".to_string())
    })?;

    Ok(home_dir.join(".lium"))
}

/// Expand path (handle ~ and environment variables)
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

/// Migrate from old JSON config to TOML format
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
