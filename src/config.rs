use crate::errors::{ConfigError, LiumError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration manager for Lium
#[derive(Debug, Clone)]
pub struct Config {
    config_path: PathBuf,
    data: ini::Ini,
}

impl Config {
    /// Create a new config instance
    pub fn new() -> Result<Self> {
        let config_dir = get_config_dir()?;
        let config_path = config_dir.join("config.ini");

        // Ensure config directory exists
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| ConfigError::DirectoryCreationFailed(e.to_string()))?;
        }

        let data = if config_path.exists() {
            ini::Ini::load_from_file(&config_path)
                .map_err(|e| ConfigError::IniError(e.to_string()))?
        } else {
            // Check for old JSON config and migrate
            let json_path = config_dir.join("config.json");
            if json_path.exists() {
                migrate_from_json(&json_path, &config_path)?;
                ini::Ini::load_from_file(&config_path)
                    .map_err(|e| ConfigError::IniError(e.to_string()))?
            } else {
                ini::Ini::new()
            }
        };

        Ok(Config { config_path, data })
    }

    /// Save the configuration to file
    pub fn save(&self) -> Result<()> {
        self.data
            .write_to_file(&self.config_path)
            .map_err(|e| ConfigError::IniError(e.to_string()))?;
        Ok(())
    }

    /// Get a configuration value
    pub fn get_value(&self, section: &str, key: &str) -> Result<Option<String>> {
        Ok(self
            .data
            .get_from(Some(section), key)
            .map(|s| s.to_string()))
    }

    /// Set a configuration value
    pub fn set_value(&mut self, section: &str, key: &str, value: &str) -> Result<()> {
        self.data.with_section(Some(section)).set(key, value);
        Ok(())
    }

    /// Remove a configuration value
    pub fn unset_value(&mut self, section: &str, key: &str) -> Result<()> {
        if let Some(section_map) = self.data.section_mut(Some(section)) {
            section_map.remove(key);
        }
        Ok(())
    }

    /// Get API key
    pub fn get_api_key(&self) -> Result<Option<String>> {
        // Try environment variable first
        if let Ok(key) = std::env::var("LIUM_API_KEY") {
            return Ok(Some(key));
        }

        // Then check config
        self.get_value("api", "api_key")
    }

    /// Set API key
    pub fn set_api_key(&mut self, api_key: &str) -> Result<()> {
        self.set_value("api", "api_key", api_key)
    }

    /// Get SSH public key path
    pub fn get_ssh_public_key_path(&self) -> Result<Option<String>> {
        self.get_value("ssh", "key_path")
    }

    /// Set SSH public key path
    pub fn set_ssh_public_key_path(&mut self, path: &str) -> Result<()> {
        self.set_value("ssh", "key_path", path)
    }

    /// Get SSH user
    pub fn get_ssh_user(&self) -> Result<String> {
        Ok(self
            .get_value("ssh", "user")?
            .unwrap_or_else(|| "root".to_string()))
    }

    /// Set SSH user
    pub fn set_ssh_user(&mut self, user: &str) -> Result<()> {
        self.set_value("ssh", "user", user)
    }

    /// Get default template ID
    pub fn get_default_template_id(&self) -> Result<Option<String>> {
        self.get_value("template", "default_id")
    }

    /// Set default template ID
    pub fn set_default_template_id(&mut self, template_id: &str) -> Result<()> {
        self.set_value("template", "default_id", template_id)
    }

    /// Get Docker credentials
    pub fn get_docker_credentials(&self) -> Result<Option<(String, String)>> {
        let username = self.get_value("docker", "username")?;
        let token = self.get_value("docker", "token")?;

        match (username, token) {
            (Some(u), Some(t)) => Ok(Some((u, t))),
            _ => Ok(None),
        }
    }

    /// Set Docker credentials
    pub fn set_docker_credentials(&mut self, username: &str, token: &str) -> Result<()> {
        self.set_value("docker", "username", username)?;
        self.set_value("docker", "token", token)?;
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

        let content = fs::read_to_string(&expanded_path).map_err(LiumError::Io)?;

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
        let mut output = String::new();

        for (section_name, section) in self.data.iter() {
            if let Some(section_name) = section_name {
                output.push_str(&format!("[{}]\n", section_name));
            }

            for (key, value) in section.iter() {
                output.push_str(&format!("{} = {}\n", key, value));
            }
            output.push('\n');
        }

        output
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

/// Migrate from old JSON config to INI format
fn migrate_from_json(json_path: &Path, ini_path: &Path) -> Result<()> {
    let json_content = fs::read_to_string(json_path).map_err(LiumError::Io)?;

    let json_data: serde_json::Value =
        serde_json::from_str(&json_content).map_err(LiumError::Serde)?;

    let mut ini = ini::Ini::new();

    // Migrate API key
    if let Some(api_key) = json_data.get("api_key").and_then(|v| v.as_str()) {
        ini.with_section(Some("api")).set("api_key", api_key);
    }

    // Migrate SSH settings
    if let Some(ssh) = json_data.get("ssh") {
        if let Some(key_path) = ssh.get("public_key_path").and_then(|v| v.as_str()) {
            ini.with_section(Some("ssh")).set("key_path", key_path);
        }
        if let Some(user) = ssh.get("user").and_then(|v| v.as_str()) {
            ini.with_section(Some("ssh")).set("user", user);
        }
    }

    // Migrate template settings
    if let Some(template) = json_data.get("template") {
        if let Some(default_id) = template.get("default_id").and_then(|v| v.as_str()) {
            ini.with_section(Some("template"))
                .set("default_id", default_id);
        }
    }

    // Migrate Docker settings
    if let Some(docker) = json_data.get("docker") {
        if let Some(username) = docker.get("username").and_then(|v| v.as_str()) {
            ini.with_section(Some("docker")).set("username", username);
        }
        if let Some(token) = docker.get("token").and_then(|v| v.as_str()) {
            ini.with_section(Some("docker")).set("token", token);
        }
    }

    // Save INI file
    ini.write_to_file(ini_path)
        .map_err(|e| ConfigError::IniError(e.to_string()))?;

    // Backup old JSON file
    let backup_path = json_path.with_extension("json.backup");
    fs::rename(json_path, backup_path).map_err(LiumError::Io)?;

    Ok(())
}

// TODO: Add config validation functions
// TODO: Add config schema versioning
// TODO: Add more specific getter/setter methods
// TODO: Add config file watching for live updates
