pub mod auth;
pub mod config;
pub mod downloader;
pub mod launcher;
pub mod pinger;
pub mod updater;

use auth::AuthStatus;
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
async fn start_device_login() -> Result<auth::DeviceCodeInfo, String> {
    auth::start_device_flow().await
}

#[tauri::command]
async fn poll_device_login(device_code: String) -> Result<Option<auth::MinecraftProfile>, String> {
    auth::poll_device_flow(&device_code).await
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

    // 2. Ensure Minecraft Assets
    let _ = downloader::ensure_assets(&game_dir).await;

    // 3. Synchronize modpack files from GitHub release manifest
    let sync_result = if !cfg.release_manifest_url.trim().is_empty() {
        downloader::check_and_sync_modpack(&app, &game_dir, &cfg.release_manifest_url, force_repair).await
    } else {
        Ok("1.0.0 (Local)".to_string())
    };

    // 4. Ensure all platform-specific libraries & natives from version.json
    let _ = downloader::ensure_libraries(&app, &game_dir).await;

    sync_result
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

    // Ensure all platform-specific libraries and natives are present before launching
    let _ = downloader::ensure_libraries(&app, &game_dir).await;

    // Launch Forge process
    launcher::launch_minecraft(&app, &java_bin, &game_dir, &profile, cfg.ram_gb, &cfg.jvm_flags, cfg.auto_connect)
}

#[tauri::command]
fn get_game_status() -> bool {
    launcher::is_minecraft_running()
}

#[tauri::command]
async fn get_update_log() -> Result<String, String> {
    let cfg = config::load_config();

    // 1. Check active game_dir FancyMenu updatelog (updated after sync)
    let game_log = PathBuf::from(&cfg.game_dir).join("config/fancymenu/customization/updatelog_layout.txt");
    if game_log.exists() {
        if let Ok(content) = std::fs::read_to_string(&game_log) {
            if let Some(text) = extract_fancymenu_text(&content) {
                return Ok(text);
            }
        }
    }

    // 2. Check local create-modpack FancyMenu updatelog
    let local_path = PathBuf::from("/home/varixxl/create-modpack/config/fancymenu/customization/updatelog_layout.txt");
    if local_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&local_path) {
            if let Some(text) = extract_fancymenu_text(&content) {
                return Ok(text);
            }
        }
    }

    // 3. Fallback: fetch from GitHub raw
    let client = reqwest::Client::new();
    let url = "https://raw.githubusercontent.com/JaviDaBomb/create-modpack/main/config/fancymenu/customization/updatelog_layout.txt";
    if let Ok(res) = client.get(url).send().await {
        if res.status().is_success() {
            if let Ok(content) = res.text().await {
                if let Some(text) = extract_fancymenu_text(&content) {
                    return Ok(text);
                }
            }
        }
    }

    Ok("Create Server Update Log currently unavailable.".to_string())
}

fn extract_fancymenu_text(layout: &str) -> Option<String> {
    let mut best_text: Option<String> = None;
    let mut max_len = 0;

    for line in layout.lines() {
        let trimmed = line.trim();
        if let Some(raw) = trimmed.strip_prefix("source = ") {
            if raw.len() > max_len {
                max_len = raw.len();
                let clean = raw.replace("%n%", "\n");
                best_text = Some(clean);
            }
        }
    }
    best_text
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        let _ = std::fs::create_dir_all(p);
    }
    open::that(p).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    open::that(url).map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_client_update() -> Result<updater::ClientUpdateInfo, String> {
    updater::check_for_update().await
}

#[tauri::command]
async fn perform_client_update(app: AppHandle, download_url: String) -> Result<String, String> {
    updater::perform_update(&app, &download_url).await
}

#[tauri::command]
async fn check_modpack_update() -> Result<downloader::ModpackUpdateInfo, String> {
    let cfg = config::load_config();
    let game_dir = PathBuf::from(&cfg.game_dir);
    downloader::check_modpack_update(&game_dir, &cfg.release_manifest_url).await
}

#[tauri::command]
fn delete_all_game_data() -> Result<(), String> {
    let cfg = config::load_config();
    let game_dir = PathBuf::from(&cfg.game_dir);
    downloader::delete_all_game_data(&game_dir)
}

#[tauri::command]
async fn reset_modpack_settings() -> Result<(), String> {
    let cfg = config::load_config();
    let game_dir = PathBuf::from(&cfg.game_dir);
    downloader::reset_modpack_settings(&game_dir).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_auth_status,
            start_device_login,
            poll_device_login,
            logout,
            get_server_status,
            sync_modpack,
            launch_game,
            get_game_status,
            get_update_log,
            open_folder,
            open_url,
            check_client_update,
            perform_client_update,
            check_modpack_update,
            delete_all_game_data,
            reset_modpack_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
