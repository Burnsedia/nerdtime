use std::sync::Mutex;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppConfig {
    pub api_url: Option<String>,
    pub token: Option<String>,
}

impl AppConfig {
    pub fn load() -> Self {
        let config_path = dirs::config_dir().map(|p| p.join("nerdtime").join("config.toml"));

        if let Some(path) = config_path {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        AppConfig::default()
    }
}

pub struct AppState {
    pub conn: Mutex<nerdtime_db::Connection>,
    pub config: AppConfig,
}

impl AppState {
    pub fn new() -> Result<Self, anyhow::Error> {
        let conn = nerdtime_db::get_connection()?;
        let config = AppConfig::load();
        Ok(AppState {
            conn: Mutex::new(conn),
            config,
        })
    }
}
