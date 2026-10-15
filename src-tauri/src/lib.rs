pub mod auth;
pub mod config;
pub mod downloader;
pub mod launcher;
pub mod pinger;

use auth::{AuthStatus, MinecraftProfile};
use config::AppConfig;
use pinger::ServerStatus;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

#[tauri::command]
fn get_config() -> AppConfig {
    config::load_config()
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    config::save_config(&config)
}

#[tauri::command]
async fn get_auth_status() -> AuthStatus {
    if let Some(profile) = auth::load_saved_profile() {
        // If expired or about to expire in 5 minutes, attempt background refresh
        let now_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if profile.expires_at <= now_sec + 300 {
            let cfg = config::load_config();
            if !cfg.azure_client_id.trim().is_empty() {
                if let Ok(refreshed) = auth::refresh_saved_session(&cfg.azure_client_id, &profile.refresh_token).await {
                    return AuthStatus {
                        is_authenticated: true,
                        profile: Some(refreshed),
                    };
                }
            }
        }

        AuthStatus {
            is_authenticated: true,
            profile: Some(profile),
        }
    } else {
        AuthStatus {
            is_authenticated: false,
            profile: None,
        }
    }
}

#[tauri::command]
async fn login(client_id: Option<String>) -> Result<MinecraftProfile, String> {
    let mut cfg = config::load_config();
    let cid = match client_id {
        Some(c) if !c.trim().is_empty() => {
            cfg.azure_client_id = c.trim().to_string();
            let _ = config::save_config(&cfg);
            cfg.azure_client_id
        }
        _ => {
            if cfg.azure_client_id.trim().is_empty() {
                config::DEFAULT_AZURE_CLIENT_ID.to_string()
            } else {
                cfg.azure_client_id
            }
        }
    };

    auth::start_login_flow(cid).await
}

#[tauri::command]
fn logout() -> Result<(), String> {
    auth::clear_profile()
}

#[tauri::command]
async fn get_server_status() -> ServerStatus {
    pinger::query_server_status().await
}

#[tauri::command]
async fn sync_modpack(app: AppHandle, force_repair: bool) -> Result<String, String> {
    let cfg = config::load_config();
    let game_dir = PathBuf::from(&cfg.game_dir);

    // 1. Ensure Adoptium Java 17 JRE
    let runtime_dir = downloader::get_runtime_dir();
    let _ = downloader::ensure_java(&app, &runtime_dir).await?;

    // 2. Synchronize modpack files from GitHub release manifest
    if !cfg.release_manifest_url.trim().is_empty() {
        downloader::check_and_sync_modpack(&app, &game_dir, &cfg.release_manifest_url, force_repair).await
    } else {
        Ok("1.0.0 (Local)".to_string())
    }
}

#[tauri::command]
async fn launch_game(app: AppHandle) -> Result<u32, String> {
    let cfg = config::load_config();
    let game_dir = PathBuf::from(&cfg.game_dir);

    // Verify auth
    let profile = auth::load_saved_profile().ok_or_else(|| "Not signed in. Please sign in to launch.".to_string())?;

    // Ensure Java 17
    let runtime_dir = downloader::get_runtime_dir();
    let java_bin = downloader::ensure_java(&app, &runtime_dir).await?;

    // Launch Forge process
    launcher::launch_minecraft(&java_bin, &game_dir, &profile, cfg.ram_gb, &cfg.jvm_flags)
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        let _ = std::fs::create_dir_all(p);
    }
    open::that(p).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_auth_status,
            login,
            logout,
            get_server_status,
            sync_modpack,
            launch_game,
            open_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
