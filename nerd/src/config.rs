use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_url: String,
    pub token: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:3000/api".to_string(),
            token: None,
        }
    }
}

fn config_path() -> Result<PathBuf> {
    let path = dirs::config_dir()
        .context("config directory not found")?
        .join("nerdtime")
        .join("config.toml");
    Ok(path)
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        let config = Config::default();
        save(&config)?;
        return Ok(config);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config: {}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("failed to parse config: {}", path.display()))?;
    Ok(config)
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}
