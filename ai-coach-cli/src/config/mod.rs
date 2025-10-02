use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,

    #[serde(default)]
    pub auth: AuthConfig,

    #[serde(default)]
    pub sync: SyncConfig,

    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub workouts: WorkoutsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,

    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub token: String,

    #[serde(default)]
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    #[serde(default = "default_true")]
    pub auto_sync: bool,

    #[serde(default = "default_conflict_resolution")]
    pub conflict_resolution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_date_format")]
    pub date_format: String,

    #[serde(default = "default_time_format")]
    pub time_format: String,

    #[serde(default = "default_true")]
    pub show_sync_status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutsConfig {
    #[serde(default = "default_distance_unit")]
    pub default_distance_unit: String,

    #[serde(default = "default_duration_unit")]
    pub default_duration_unit: String,
}

// Default value functions
fn default_base_url() -> String {
    "http://localhost:3000".to_string()
}

fn default_timeout() -> u64 {
    30
}

fn default_true() -> bool {
    true
}

fn default_conflict_resolution() -> String {
    "server_wins".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_date_format() -> String {
    "%Y-%m-%d".to_string()
}

fn default_time_format() -> String {
    "24h".to_string()
}

fn default_distance_unit() -> String {
    "km".to_string()
}

fn default_duration_unit() -> String {
    "minutes".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api: ApiConfig::default(),
            auth: AuthConfig::default(),
            sync: SyncConfig::default(),
            ui: UiConfig::default(),
            workouts: WorkoutsConfig::default(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            timeout_seconds: default_timeout(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            refresh_token: String::new(),
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            auto_sync: default_true(),
            conflict_resolution: default_conflict_resolution(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            date_format: default_date_format(),
            time_format: default_time_format(),
            show_sync_status: default_true(),
        }
    }
}

impl Default for WorkoutsConfig {
    fn default() -> Self {
        Self {
            default_distance_unit: default_distance_unit(),
            default_duration_unit: default_duration_unit(),
        }
    }
}

impl Config {
    /// Get config directory path (~/.ai-coach/)
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        Ok(home.join(".ai-coach"))
    }

    /// Get config file path (~/.ai-coach/config.toml)
    pub fn config_file() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let config_file = Self::config_file()?;

        if !config_file.exists() {
            tracing::info!("Config file not found, using defaults");
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&config_file).context("Failed to read config file")?;

        let config: Config = toml::from_str(&contents).context("Failed to parse config file")?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        let config_file = Self::config_file()?;
        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&config_file, contents).context("Failed to write config file")?;

        Ok(())
    }

    /// Check if user is authenticated
    pub fn is_authenticated(&self) -> bool {
        !self.auth.token.is_empty()
    }

    /// Update auth tokens
    pub fn set_tokens(&mut self, token: String, refresh_token: String) {
        self.auth.token = token;
        self.auth.refresh_token = refresh_token;
    }

    /// Clear auth tokens
    pub fn clear_tokens(&mut self) {
        self.auth.token.clear();
        self.auth.refresh_token.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.api.base_url, "http://localhost:3000");
        assert_eq!(config.api.timeout_seconds, 30);
        assert!(config.sync.auto_sync);
        assert_eq!(config.sync.conflict_resolution, "server_wins");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();

        assert_eq!(config.api.base_url, deserialized.api.base_url);
        assert_eq!(config.ui.theme, deserialized.ui.theme);
    }
}
