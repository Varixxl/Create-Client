pub mod auth;
pub mod config;
pub mod downloader;
pub mod launcher;
pub mod pinger;
pub mod profiles;
pub mod updater;

use auth::AuthStatus;
use config::AppConfig;
use pinger::ServerStatus;
use profiles::{
    InstalledModInfo, InstanceLogsPayload, ModrinthSearchResult, ProfilesState,
    CREATE_SERVER_PROFILE_ID,
};
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
fn get_profiles() -> ProfilesState {
    profiles::load_profiles_state()
}

#[tauri::command]
fn select_profile(profile_id: String) -> Result<ProfilesState, String> {
    profiles::select_profile(&profile_id)
}

#[tauri::command]
fn create_profile(
    name: String,
    description: String,
    loader: String,
    mc_version: String,
    include_default_fabric_mods: bool,
    ram_gb: Option<u32>,
    jvm_flags: Option<String>,
) -> Result<ProfilesState, String> {
    profiles::create_profile(
        name,
        description,
        loader,
        mc_version,
        include_default_fabric_mods,
        ram_gb,
        jvm_flags,
    )
}

#[tauri::command]
fn duplicate_profile(source_id: String, new_name: String) -> Result<ProfilesState, String> {
    profiles::duplicate_profile(&source_id, new_name)
}

#[tauri::command]
fn update_profile(
    profile_id: String,
    name: String,
    description: Option<String>,
    ram_gb: Option<u32>,
    jvm_flags: Option<String>,
) -> Result<ProfilesState, String> {
    profiles::update_profile(&profile_id, name, description, ram_gb, jvm_flags)
}

#[tauri::command]
fn delete_profile(profile_id: String) -> Result<ProfilesState, String> {
    profiles::delete_profile(&profile_id)
}

#[tauri::command]
async fn get_available_mc_versions() -> Result<Vec<String>, String> {
    profiles::get_available_mc_versions().await
}

#[tauri::command]
async fn sync_profile(app: AppHandle, profile_id: String, force_repair: bool) -> Result<String, String> {
    if profile_id == CREATE_SERVER_PROFILE_ID {
        return sync_modpack(app, force_repair).await;
    }
    profiles::sync_profile_instance(&app, &profile_id, force_repair).await
}

#[tauri::command]
async fn launch_profile(app: AppHandle, profile_id: String) -> Result<u32, String> {
    let cfg = config::load_config();
    let state = profiles::load_profiles_state();
    let target_profile = state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .cloned()
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let effective_ram = target_profile.ram_gb.unwrap_or(cfg.ram_gb);
    let effective_flags = target_profile
        .jvm_flags
        .clone()
        .unwrap_or_else(|| cfg.jvm_flags.clone());

    if target_profile.id == CREATE_SERVER_PROFILE_ID {
        let game_dir = PathBuf::from(&cfg.game_dir);
        let auth_prof = auth::ensure_valid_profile(true)
            .await
            .map_err(|e| {
                format!(
                    "Session expired and automatic refresh failed ({}). Please sign in again.",
                    e
                )
            })?
            .ok_or_else(|| "Not signed in. Please sign in to launch.".to_string())?;

        let runtime_dir = downloader::get_runtime_dir();
        let java_bin = downloader::ensure_java(&app, &runtime_dir).await?;
        downloader::ensure_libraries(&app, &game_dir).await?;
        downloader::ensure_assets(&app, &game_dir).await?;

        return launcher::launch_minecraft(
            &app,
            &java_bin,
            &game_dir,
            &auth_prof,
            effective_ram,
            &effective_flags,
            cfg.auto_connect,
            &cfg.graphics_preset,
        );
    }

    let instance_dir = PathBuf::from(&target_profile.instance_dir);
    if !instance_dir.join("fabric_launch_meta.json").exists() {
        profiles::sync_profile_instance(&app, &profile_id, false).await?;
    }

    let auth_prof = auth::ensure_valid_profile(true)
        .await
        .map_err(|e| {
            format!(
                "Session expired and automatic refresh failed ({}). Please sign in again.",
                e
            )
        })?
        .ok_or_else(|| "Not signed in. Please sign in to launch.".to_string())?;

    let runtime_dir = downloader::get_runtime_dir();
    let java_bin = downloader::ensure_java(&app, &runtime_dir).await?;

    launcher::launch_fabric_or_vanilla(
        &app,
        &java_bin,
        &instance_dir,
        &auth_prof,
        effective_ram,
        &effective_flags,
    )
}

#[tauri::command]
fn list_installed_mods(profile_id: String) -> Result<Vec<InstalledModInfo>, String> {
    profiles::list_installed_mods(&profile_id)
}

#[tauri::command]
fn toggle_mod(profile_id: String, file_name: String, enabled: bool) -> Result<(), String> {
    profiles::toggle_mod(&profile_id, &file_name, enabled)
}

#[tauri::command]
fn remove_mod(profile_id: String, file_name: String) -> Result<(), String> {
    profiles::remove_mod(&profile_id, &file_name)
}

#[tauri::command]
async fn search_modrinth_mods(
    profile_id: String,
    query: String,
) -> Result<Vec<ModrinthSearchResult>, String> {
    profiles::search_modrinth_mods(&profile_id, query).await
}

#[tauri::command]
async fn install_modrinth_mod(
    app: AppHandle,
    profile_id: String,
    slug: String,
) -> Result<(), String> {
    profiles::install_modrinth_mod(&app, &profile_id, slug).await
}

#[tauri::command]
fn get_instance_logs(profile_id: String, max_lines: usize) -> Result<InstanceLogsPayload, String> {
    profiles::get_instance_logs(&profile_id, max_lines)
}

#[tauri::command]
fn clear_instance_logs(profile_id: String) -> Result<(), String> {
    profiles::clear_instance_logs(&profile_id)
}

#[tauri::command]
async fn get_auth_status() -> AuthStatus {
    match auth::ensure_valid_profile(false).await {
        Ok(Some(profile)) => AuthStatus {
            is_authenticated: true,
            profile: Some(profile.redacted_for_frontend()),
        },
        _ => AuthStatus {
            is_authenticated: false,
            profile: None,
        },
    }
}

#[tauri::command]
async fn start_device_login() -> Result<auth::DeviceCodeInfo, String> {
    auth::start_device_flow().await
}

#[tauri::command]
async fn poll_device_login(device_code: String) -> Result<Option<auth::MinecraftProfile>, String> {
    let profile = auth::poll_device_flow(&device_code).await?;
    Ok(profile.map(|p| p.redacted_for_frontend()))
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

    // 1. Ensure Azul Zulu 21 JRE
    let runtime_dir = downloader::get_runtime_dir();
    let _ = downloader::ensure_java(&app, &runtime_dir).await?;

    // 2. Ensure Minecraft Assets
    downloader::ensure_assets(&app, &game_dir).await?;

    // 3. Synchronize modpack files from GitHub release manifest
    let sync_result = if !cfg.release_manifest_url.trim().is_empty() {
        downloader::check_and_sync_modpack(&app, &game_dir, &cfg.release_manifest_url, force_repair).await?
    } else {
        "1.0.0 (Local)".to_string()
    };

    // 4. Ensure all platform-specific libraries & natives from version.json
    downloader::ensure_libraries(&app, &game_dir).await?;

    Ok(sync_result)
}

#[tauri::command]
async fn launch_game(app: AppHandle) -> Result<u32, String> {
    let cfg = config::load_config();
    let game_dir = PathBuf::from(&cfg.game_dir);

    // Verify and proactively refresh auth session so every launch starts with a fresh 24h token
    let profile = auth::ensure_valid_profile(true)
        .await
        .map_err(|e| format!("Session expired and automatic refresh failed ({}). Please sign in again.", e))?
        .ok_or_else(|| "Not signed in. Please sign in to launch.".to_string())?;

    // Ensure Java 21
    let runtime_dir = downloader::get_runtime_dir();
    let java_bin = downloader::ensure_java(&app, &runtime_dir).await?;

    // Ensure all platform-specific libraries, natives, and vanilla assets are present before launching
    downloader::ensure_libraries(&app, &game_dir).await?;
    downloader::ensure_assets(&app, &game_dir).await?;

    // Launch Forge process
    launcher::launch_minecraft(&app, &java_bin, &game_dir, &profile, cfg.ram_gb, &cfg.jvm_flags, cfg.auto_connect, &cfg.graphics_preset)
}

#[derive(serde::Serialize)]
pub struct GraphicsInfo {
    pub detected_gpu: String,
    pub detected_tier: String,
    pub detected_refresh_rate: u32,
}

#[tauri::command]
fn get_graphics_info() -> GraphicsInfo {
    GraphicsInfo {
        detected_gpu: launcher::detect_gpu_name(),
        detected_tier: launcher::detect_gpu_tier().to_string(),
        detected_refresh_rate: launcher::detect_refresh_rate(),
    }
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
    let parsed = url::Url::parse(&url).map_err(|e| format!("Invalid URL: {}", e))?;
    match parsed.scheme() {
        "https" | "http" => open::that(parsed.as_str()).map_err(|e| e.to_string()),
        scheme => Err(format!("Disallowed URL scheme: {}", scheme)),
    }
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

#[tauri::command]
fn force_close_game(app: AppHandle) -> Result<(), String> {
    launcher::kill_minecraft(&app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_profiles,
            select_profile,
            create_profile,
            duplicate_profile,
            update_profile,
            delete_profile,
            get_available_mc_versions,
            sync_profile,
            launch_profile,
            list_installed_mods,
            toggle_mod,
            remove_mod,
            search_modrinth_mods,
            install_modrinth_mod,
            get_instance_logs,
            clear_instance_logs,
            get_auth_status,
            start_device_login,
            poll_device_login,
            logout,
            get_server_status,
            sync_modpack,
            launch_game,
            get_game_status,
            force_close_game,
            get_update_log,
            open_folder,
            open_url,
            check_client_update,
            perform_client_update,
            check_modpack_update,
            delete_all_game_data,
            reset_modpack_settings,
            get_graphics_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
