use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::models::WatchlistEntry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub steam_paths: Vec<String>,
    #[serde(default)]
    pub epic_paths: Vec<String>,
    #[serde(default)]
    pub gog_paths: Vec<String>,
    #[serde(default = "default_target_fps")]
    pub target_fps: u32,
    #[serde(default = "default_prefer_dlss")]
    pub prefer_dlss: bool,
    #[serde(default = "default_cache_ttl_hours")]
    pub cache_ttl_hours: u64,
}

fn default_target_fps() -> u32 {
    60
}
fn default_prefer_dlss() -> bool {
    true
}
fn default_cache_ttl_hours() -> u64 {
    24
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            steam_paths: Vec::new(),
            epic_paths: Vec::new(),
            gog_paths: Vec::new(),
            target_fps: default_target_fps(),
            prefer_dlss: default_prefer_dlss(),
            cache_ttl_hours: default_cache_ttl_hours(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nvopt")
}

pub fn cache_dir() -> PathBuf {
    config_dir().join("cache")
}

pub fn load_config() -> Result<AppConfig> {
    let path = config_dir().join("config.toml");
    if path.exists() {
        let contents = std::fs::read_to_string(&path)?;
        let config: AppConfig = toml::from_str(&contents)?;
        Ok(config)
    } else {
        let config = AppConfig::default();
        save_config(&config)?;
        Ok(config)
    }
}

pub fn save_config(config: &AppConfig) -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("config.toml");
    let contents = toml::to_string_pretty(config)?;
    std::fs::write(path, contents)?;
    Ok(())
}

pub fn load_watchlist() -> Result<Vec<WatchlistEntry>> {
    let path = config_dir().join("watchlist.toml");
    if path.exists() {
        let contents = std::fs::read_to_string(&path)?;
        let wrapper: WatchlistFile = toml::from_str(&contents)?;
        Ok(wrapper.games)
    } else {
        Ok(Vec::new())
    }
}

pub fn save_watchlist(entries: &[WatchlistEntry]) -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("watchlist.toml");
    let wrapper = WatchlistFile {
        games: entries.to_vec(),
    };
    let contents = toml::to_string_pretty(&wrapper)?;
    std::fs::write(path, contents)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct WatchlistFile {
    #[serde(default)]
    games: Vec<WatchlistEntry>,
}
