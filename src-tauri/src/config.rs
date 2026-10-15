use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ram_gb: u32,
    pub jvm_flags: String,
    pub game_dir: String,
    pub azure_client_id: String,
    pub auto_connect: bool,
    pub release_manifest_url: String,
}

pub const DEFAULT_AZURE_CLIENT_ID: &str = "43e4ff18-9c63-455f-9224-a124a6b71086";

impl Default for AppConfig {
    fn default() -> Self {
        let default_game_dir = get_default_game_dir().to_string_lossy().to_string();
        Self {
            ram_gb: 8,
            jvm_flags: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch".to_string(),
            game_dir: default_game_dir,
            azure_client_id: DEFAULT_AZURE_CLIENT_ID.to_string(),
            auto_connect: false,
            release_manifest_url: "https://raw.githubusercontent.com/JaviDaBomb/Create-Client/main/manifest.json".to_string(),
        }
    }
}

pub fn get_default_game_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "create", "CreateClient") {
        proj_dirs.data_dir().join("game")
    } else {
        PathBuf::from("./create_client_game")
    }
}

pub fn get_config_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "create", "CreateClient") {
        let dir = proj_dirs.config_dir().to_path_buf();
        let _ = fs::create_dir_all(&dir);
        dir
    } else {
        PathBuf::from(".")
    }
}

pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(mut config) = serde_json::from_str::<AppConfig>(&content) {
                if config.azure_client_id.trim().is_empty() {
                    config.azure_client_id = DEFAULT_AZURE_CLIENT_ID.to_string();
                    let _ = save_config(&config);
                }
                return config;
            }
        }
    }
    let default = AppConfig::default();
    let _ = save_config(&default);
    default
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let data = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}
