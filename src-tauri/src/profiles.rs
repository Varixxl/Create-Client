use crate::config;
use crate::downloader::ProgressPayload;
use crate::launcher::{is_library_for_current_platform, maven_to_path, VersionLibrary};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha512};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

pub const CREATE_SERVER_PROFILE_ID: &str = "create-server";
pub const FABRIC_DEFAULT_PROFILE_ID: &str = "fabric-performance-qol";

/// Default curated Fabric optimization + quality-of-life Modrinth project slugs.
/// Easy to modify or extend — versions are resolved dynamically via Modrinth's API.
pub const DEFAULT_FABRIC_MOD_SLUGS: &[&str] = &[
    // Core & Performance / Optimization Mods
    "fabric-api",
    "sodium",
    "lithium",
    "ferrite-core",
    "immediatelyfast",
    "entityculling",
    // Quality-of-Life Mods
    "modmenu",
    "cloth-config",
    "jade",
    "appleskin",
    "xaeros-minimap",
    "zoomify",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mc_version: String,
    pub resolved_mc_version: Option<String>,
    pub loader: String, // "forge" | "fabric" | "vanilla"
    pub loader_version: Option<String>,
    pub instance_dir: String,
    pub is_builtin: bool,
    pub ram_gb: Option<u32>,
    pub jvm_flags: Option<String>,
    #[serde(default)]
    pub java_version: Option<u32>,
    #[serde(default)]
    pub modrinth_slugs: Vec<String>,
    pub created_at: u64,
    pub last_played: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesState {
    pub selected_profile_id: String,
    pub profiles: Vec<ModpackProfile>,
    #[serde(default)]
    pub deleted_builtin_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLaunchMeta {
    pub mc_version: String,
    pub loader_version: String,
    pub main_class: String,
    pub asset_index: String,
    pub assets_dir: String,
    pub natives_dir: String,
    pub client_jar: String,
    pub classpath: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledModMetadata {
    pub slug: String,
    pub title: String,
    pub version_number: String,
    pub file_name: String,
    pub description: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledModInfo {
    pub file_name: String,
    pub title: String,
    pub slug: Option<String>,
    pub version_number: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub enabled: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthSearchResult {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSearchResult {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub installed: bool,
    pub content_type: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModloaderVersionItem {
    pub version: String,
    pub stable: bool,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInstance {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub server_dir: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: String,
    pub host: String,
    pub created_at: String,
    pub installed_mods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDivergenceInfo {
    pub server_id: String,
    pub server_name: String,
    pub profile_mods_count: usize,
    pub server_mods_count: usize,
    pub added_to_profile: Vec<String>,
    pub removed_from_profile: Vec<String>,
    pub loader_mismatch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerManifest {
    pub server_id: String,
    pub profile_id: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: String,
    pub mods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceLogsPayload {
    pub log_source: String,
    pub lines: Vec<String>,
    pub crash_report: Option<String>,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn get_instances_root_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "create", "CreateClient") {
        let dir = proj_dirs.data_dir().join("instances");
        let _ = fs::create_dir_all(&dir);
        dir
    } else {
        let dir = PathBuf::from("./create_client_instances");
        let _ = fs::create_dir_all(&dir);
        dir
    }
}

pub fn get_shared_meta_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "create", "CreateClient") {
        let dir = proj_dirs.data_dir().join("shared_meta");
        let _ = fs::create_dir_all(&dir);
        dir
    } else {
        let dir = PathBuf::from("./create_client_meta");
        let _ = fs::create_dir_all(&dir);
        dir
    }
}

fn get_profiles_file_path() -> PathBuf {
    config::get_config_dir().join("profiles.json")
}

pub fn get_servers_root_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "create", "CreateClient") {
        let dir = proj_dirs.data_dir().join("servers");
        let _ = fs::create_dir_all(&dir);
        dir
    } else {
        let dir = PathBuf::from("./create_client_servers");
        let _ = fs::create_dir_all(&dir);
        dir
    }
}

pub fn get_servers_file_path() -> PathBuf {
    config::get_config_dir().join("servers.json")
}

pub fn load_servers_state() -> Vec<ServerInstance> {
    let path = get_servers_file_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(servers) = serde_json::from_str::<Vec<ServerInstance>>(&content) {
                return servers;
            }
        }
    }
    Vec::new()
}

pub fn save_servers_state(servers: &[ServerInstance]) -> Result<(), String> {
    let path = get_servers_file_path();
    let content = serde_json::to_string_pretty(servers).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

fn build_builtin_create_profile(cfg: &config::AppConfig) -> ModpackProfile {
    ModpackProfile {
        id: CREATE_SERVER_PROFILE_ID.to_string(),
        name: "Create Server".to_string(),
        description: "Official Create Server Forge 1.20.1 modpack with differential auto-sync, Distant Horizons, and Complementary Shaders.".to_string(),
        mc_version: "1.20.1".to_string(),
        resolved_mc_version: Some("1.20.1".to_string()),
        loader: "forge".to_string(),
        loader_version: Some("47.4.20".to_string()),
        instance_dir: cfg.game_dir.clone(),
        is_builtin: true,
        ram_gb: None,
        jvm_flags: None,
        java_version: Some(17),
        modrinth_slugs: Vec::new(),
        created_at: 1700000000,
        last_played: None,
    }
}

fn build_builtin_fabric_profile() -> ModpackProfile {
    let instance_dir = get_instances_root_dir()
        .join(FABRIC_DEFAULT_PROFILE_ID)
        .to_string_lossy()
        .to_string();
    ModpackProfile {
        id: FABRIC_DEFAULT_PROFILE_ID.to_string(),
        name: "Fabric Performance & QoL".to_string(),
        description: "Latest stable Minecraft with Fabric Loader, Sodium, Lithium, FerriteCore, ImmediatelyFast, EntityCulling, Mod Menu, Xaero's Minimap, Jade, AppleSkin & Zoomify.".to_string(),
        mc_version: "latest".to_string(),
        resolved_mc_version: None,
        loader: "fabric".to_string(),
        loader_version: None,
        instance_dir,
        is_builtin: true,
        ram_gb: None,
        jvm_flags: None,
        java_version: Some(25),
        modrinth_slugs: DEFAULT_FABRIC_MOD_SLUGS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        created_at: 1700000001,
        last_played: None,
    }
}

pub fn load_profiles_state() -> ProfilesState {
    let cfg = config::load_config();
    let path = get_profiles_file_path();

    let mut state = if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<ProfilesState>(&s).ok())
            .unwrap_or_else(|| ProfilesState {
                selected_profile_id: CREATE_SERVER_PROFILE_ID.to_string(),
                profiles: Vec::new(),
                deleted_builtin_ids: Vec::new(),
            })
    } else {
        ProfilesState {
            selected_profile_id: CREATE_SERVER_PROFILE_ID.to_string(),
            profiles: Vec::new(),
            deleted_builtin_ids: Vec::new(),
        }
    };

    // Ensure built-in Create Server profile exists ONLY if user hasn't explicitly deleted it
    if !state.deleted_builtin_ids.contains(&CREATE_SERVER_PROFILE_ID.to_string()) {
        if let Some(create_prof) = state
            .profiles
            .iter_mut()
            .find(|p| p.id == CREATE_SERVER_PROFILE_ID)
        {
            create_prof.instance_dir = cfg.game_dir.clone();
            create_prof.is_builtin = true;
            if create_prof.java_version.is_none() {
                create_prof.java_version = Some(17);
            }
        } else {
            state.profiles.insert(0, build_builtin_create_profile(&cfg));
        }
    }

    // Ensure built-in Fabric Performance & QoL profile exists ONLY if user hasn't explicitly deleted it
    if !state.deleted_builtin_ids.contains(&FABRIC_DEFAULT_PROFILE_ID.to_string()) {
        if let Some(fab_prof) = state
            .profiles
            .iter_mut()
            .find(|p| p.id == FABRIC_DEFAULT_PROFILE_ID)
        {
            if fab_prof.java_version.is_none() {
                fab_prof.java_version = Some(25);
            }
        } else {
            let insert_idx = if state.profiles.is_empty() { 0 } else { 1.min(state.profiles.len()) };
            state
                .profiles
                .insert(insert_idx, build_builtin_fabric_profile());
        }
    }

    // Guarantee that every single profile in state has a concrete, compatible java_version
    for p in state.profiles.iter_mut() {
        if p.java_version.is_none() {
            let instance_path = PathBuf::from(&p.instance_dir);
            let target_mc = p.resolved_mc_version.as_deref().unwrap_or(&p.mc_version);
            let req = crate::downloader::determine_required_java_version(&instance_path, target_mc);
            p.java_version = Some(req);
        }
    }

    if !state
        .profiles
        .iter()
        .any(|p| p.id == state.selected_profile_id)
    {
        if let Some(first) = state.profiles.first() {
            state.selected_profile_id = first.id.clone();
        } else {
            state.selected_profile_id = String::new();
        }
    }

    let _ = save_profiles_state(&state);
    state
}

pub fn save_profiles_state(state: &ProfilesState) -> Result<(), String> {
    let path = get_profiles_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn select_profile(profile_id: &str) -> Result<ProfilesState, String> {
    let mut state = load_profiles_state();
    if !state.profiles.iter().any(|p| p.id == profile_id) {
        return Err(format!("Profile '{}' not found.", profile_id));
    }
    state.selected_profile_id = profile_id.to_string();
    save_profiles_state(&state)?;
    Ok(state)
}

fn sanitize_slug(name: &str) -> String {
    let raw: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let collapsed = raw
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if collapsed.is_empty() {
        "instance".to_string()
    } else {
        collapsed
    }
}

pub fn create_profile(
    name: String,
    description: String,
    loader: String,
    loader_version: Option<String>,
    mc_version: String,
    include_default_mods: bool,
    ram_gb: Option<u32>,
    jvm_flags: Option<String>,
    java_version: Option<u32>,
) -> Result<ProfilesState, String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Profile name cannot be empty.".to_string());
    }

    let normalized_loader = loader.trim().to_lowercase();
    let supported_loaders = ["fabric", "quilt", "forge", "neoforge", "vanilla"];
    if !supported_loaders.contains(&normalized_loader.as_str()) {
        return Err(format!(
            "Unsupported loader '{}'. Supported loaders: {}",
            loader,
            supported_loaders.join(", ")
        ));
    }

    let mut state = load_profiles_state();
    let base_slug = sanitize_slug(trimmed_name);
    let mut unique_id = base_slug.clone();
    let mut counter = 2;
    while state.profiles.iter().any(|p| p.id == unique_id) {
        unique_id = format!("{}-{}", base_slug, counter);
        counter += 1;
    }

    let instance_path = get_instances_root_dir().join(&unique_id);
    fs::create_dir_all(instance_path.join("mods")).map_err(|e| e.to_string())?;
    fs::create_dir_all(instance_path.join("resourcepacks")).map_err(|e| e.to_string())?;
    fs::create_dir_all(instance_path.join("shaderpacks")).map_err(|e| e.to_string())?;
    fs::create_dir_all(instance_path.join("config")).map_err(|e| e.to_string())?;
    fs::create_dir_all(instance_path.join("saves")).map_err(|e| e.to_string())?;

    let slugs = if normalized_loader == "fabric" && include_default_mods {
        DEFAULT_FABRIC_MOD_SLUGS
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let target_mc = if mc_version.trim().is_empty() {
        "latest".to_string()
    } else {
        mc_version.trim().to_string()
    };

    let min_java = crate::downloader::determine_required_java_version(&instance_path, &target_mc);
    let resolved_java = match java_version {
        Some(v) if v > 0 => v.max(min_java),
        _ => min_java,
    };

    let desc = if description.trim().is_empty() {
        if normalized_loader == "vanilla" {
            format!("Custom Vanilla Minecraft ({}) instance.", target_mc)
        } else {
            format!(
                "Custom {} instance ({}) with isolated mods, saves, and configs.",
                normalized_loader.to_uppercase(),
                target_mc
            )
        }
    } else {
        description.trim().to_string()
    };

    let profile = ModpackProfile {
        id: unique_id.clone(),
        name: trimmed_name.to_string(),
        description: desc,
        mc_version: target_mc.clone(),
        resolved_mc_version: if target_mc == "latest" {
            None
        } else {
            Some(target_mc)
        },
        loader: normalized_loader,
        loader_version: loader_version.filter(|s| !s.trim().is_empty()),
        instance_dir: instance_path.to_string_lossy().to_string(),
        is_builtin: false,
        ram_gb,
        jvm_flags: jvm_flags.filter(|s| !s.trim().is_empty()),
        java_version: Some(resolved_java),
        modrinth_slugs: slugs,
        created_at: now_unix(),
        last_played: None,
    };

    state.profiles.push(profile);
    state.selected_profile_id = unique_id;
    save_profiles_state(&state)?;
    Ok(state)
}

pub fn duplicate_profile(source_id: &str, new_name: String) -> Result<ProfilesState, String> {
    let mut state = load_profiles_state();
    let source = state
        .profiles
        .iter()
        .find(|p| p.id == source_id)
        .cloned()
        .ok_or_else(|| format!("Source profile '{}' not found.", source_id))?;

    if source.id == CREATE_SERVER_PROFILE_ID {
        return Err("The official Create Server profile uses differential server sync and cannot be duplicated as a standalone profile.".to_string());
    }

    let trimmed_name = if new_name.trim().is_empty() {
        format!("{} (Copy)", source.name)
    } else {
        new_name.trim().to_string()
    };

    let base_slug = sanitize_slug(&trimmed_name);
    let mut unique_id = base_slug.clone();
    let mut counter = 2;
    while state.profiles.iter().any(|p| p.id == unique_id) {
        unique_id = format!("{}-{}", base_slug, counter);
        counter += 1;
    }

    let src_dir = PathBuf::from(&source.instance_dir);
    let dst_dir = get_instances_root_dir().join(&unique_id);
    fs::create_dir_all(&dst_dir).map_err(|e| e.to_string())?;

    // Copy mods, config, options.txt, and launch metadata so the duplicated instance is ready immediately
    for subdir in ["mods", "config", "resourcepacks", "shaderpacks"] {
        let s = src_dir.join(subdir);
        let d = dst_dir.join(subdir);
        if s.exists() {
            copy_dir_recursive(&s, &d)?;
        } else {
            let _ = fs::create_dir_all(&d);
        }
    }
    let _ = fs::create_dir_all(dst_dir.join("saves"));

    for file_name in ["options.txt", "servers.dat", "fabric_launch_meta.json"] {
        let s = src_dir.join(file_name);
        if s.exists() {
            let _ = fs::copy(&s, dst_dir.join(file_name));
        }
    }

    let duplicated = ModpackProfile {
        id: unique_id.clone(),
        name: trimmed_name,
        description: source.description.clone(),
        mc_version: source.mc_version.clone(),
        resolved_mc_version: source.resolved_mc_version.clone(),
        loader: source.loader.clone(),
        loader_version: source.loader_version.clone(),
        instance_dir: dst_dir.to_string_lossy().to_string(),
        is_builtin: false,
        ram_gb: source.ram_gb,
        jvm_flags: source.jvm_flags.clone(),
        java_version: source.java_version,
        modrinth_slugs: source.modrinth_slugs.clone(),
        created_at: now_unix(),
        last_played: None,
    };

    state.profiles.push(duplicated);
    state.selected_profile_id = unique_id;
    save_profiles_state(&state)?;
    Ok(state)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    let entries = fs::read_dir(src).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn update_profile(
    profile_id: &str,
    name: String,
    description: Option<String>,
    ram_gb: Option<u32>,
    jvm_flags: Option<String>,
    java_version: Option<u32>,
) -> Result<ProfilesState, String> {
    let mut state = load_profiles_state();
    let prof = state
        .profiles
        .iter_mut()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let trimmed = name.trim();
    if !trimmed.is_empty() {
        prof.name = trimmed.to_string();
    }
    if let Some(desc) = description {
        prof.description = desc.trim().to_string();
    }
    prof.ram_gb = ram_gb;
    prof.jvm_flags = jvm_flags.filter(|s| !s.trim().is_empty());
    if let Some(jv) = java_version {
        if jv > 0 {
            let instance_path = PathBuf::from(&prof.instance_dir);
            let mc = prof.resolved_mc_version.as_deref().unwrap_or(&prof.mc_version);
            let min_java = crate::downloader::determine_required_java_version(&instance_path, mc);
            prof.java_version = Some(jv.max(min_java));
        } else {
            let instance_path = PathBuf::from(&prof.instance_dir);
            let mc = prof.resolved_mc_version.as_deref().unwrap_or(&prof.mc_version);
            prof.java_version = Some(crate::downloader::determine_required_java_version(&instance_path, mc));
        }
    }

    save_profiles_state(&state)?;
    Ok(state)
}

pub fn delete_profile(profile_id: &str) -> Result<ProfilesState, String> {
    let mut state = load_profiles_state();
    let target = state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .cloned()
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let instances_root = get_instances_root_dir();
    let dir_path = PathBuf::from(&target.instance_dir);
    if dir_path.exists() && dir_path.starts_with(&instances_root) && dir_path != instances_root {
        let _ = fs::remove_dir_all(&dir_path);
    }

    if target.is_builtin || profile_id == CREATE_SERVER_PROFILE_ID || profile_id == FABRIC_DEFAULT_PROFILE_ID {
        if !state.deleted_builtin_ids.contains(&profile_id.to_string()) {
            state.deleted_builtin_ids.push(profile_id.to_string());
        }
    }

    state.profiles.retain(|p| p.id != profile_id);

    if !state
        .profiles
        .iter()
        .any(|p| p.id == state.selected_profile_id)
    {
        if let Some(first) = state.profiles.first() {
            state.selected_profile_id = first.id.clone();
        } else {
            state.selected_profile_id = String::new();
        }
    }

    save_profiles_state(&state)?;
    Ok(state)
}

// ============================================================================
// Dynamic Version Resolution & Instance Sync (Mojang + Fabric Meta + Modrinth)
// ============================================================================

#[derive(Debug, Deserialize)]
struct ModrinthGameVersionTag {
    version: String,
    version_type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct ModrinthVersion {
    id: String,
    project_id: String,
    version_number: String,
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    files: Vec<ModrinthFile>,
    #[serde(default)]
    dependencies: Vec<ModrinthDependency>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct ModrinthFile {
    url: String,
    filename: String,
    primary: Option<bool>,
    size: u64,
    hashes: ModrinthHashes,
}

#[derive(Debug, Clone, Deserialize)]
struct ModrinthHashes {
    sha1: Option<String>,
    sha512: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModrinthDependency {
    project_id: Option<String>,
    dependency_type: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ModrinthProject {
    id: String,
    slug: String,
    title: String,
    description: String,
    icon_url: Option<String>,
}

fn emit_progress(
    app: &AppHandle,
    task: &str,
    current_bytes: u64,
    total_bytes: u64,
    percentage: f64,
    speed_bytes_sec: u64,
) {
    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: task.to_string(),
            current_bytes,
            total_bytes,
            percentage,
            speed_bytes_sec,
        },
    );
}

fn url_encode(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}

fn build_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("CreateClient/1.2.4 (https://github.com/Varixxl/create-client)")
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| format!("Failed to initialize HTTP client: {}", e))
}

/// Lists Minecraft versions from Modrinth's tag API for the Profile creation dropdown.
pub async fn get_available_mc_versions(include_snapshots: bool) -> Result<Vec<String>, String> {
    let client = build_http_client()?;
    let tags: Vec<ModrinthGameVersionTag> = client
        .get("https://api.modrinth.com/v2/tag/game_version")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Minecraft versions: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Invalid version tag response: {}", e))?;

    let releases: Vec<String> = tags
        .into_iter()
        .filter(|t| include_snapshots || t.version_type == "release")
        .take(if include_snapshots { 80 } else { 35 })
        .map(|t| t.version)
        .collect();

    Ok(releases)
}

/// Dynamically queries official modloader metadata endpoints to list compatible loader versions
pub async fn get_modloader_versions(
    mc_version: String,
    loader: String,
) -> Result<Vec<ModloaderVersionItem>, String> {
    let client = build_http_client()?;
    let loader_lower = loader.to_lowercase().trim().to_string();

    match loader_lower.as_str() {
        "fabric" => {
            let url = if mc_version != "latest" && !mc_version.is_empty() {
                format!("https://meta.fabricmc.net/v2/versions/loader/{}", mc_version)
            } else {
                "https://meta.fabricmc.net/v2/versions/loader".to_string()
            };

            let mut list: Vec<ModloaderVersionItem> = Vec::new();
            if let Ok(res) = client.get(&url).send().await {
                if res.status().is_success() {
                    if let Ok(json_arr) = res.json::<serde_json::Value>().await {
                        if let Some(arr) = json_arr.as_array() {
                            for (idx, item) in arr.iter().take(30).enumerate() {
                                let ver = item.get("loader")
                                    .and_then(|l| l.get("version"))
                                    .and_then(|v| v.as_str())
                                    .or_else(|| item.get("version").and_then(|v| v.as_str()));
                                let stable = item.get("loader")
                                    .and_then(|l| l.get("stable"))
                                    .and_then(|s| s.as_bool())
                                    .or_else(|| item.get("stable").and_then(|s| s.as_bool()))
                                    .unwrap_or(true);

                                if let Some(v_str) = ver {
                                    list.push(ModloaderVersionItem {
                                        version: v_str.to_string(),
                                        stable,
                                        recommended: idx == 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            if list.is_empty() {
                if let Ok(fallback_res) = client.get("https://meta.fabricmc.net/v2/versions/loader").send().await {
                    if let Ok(json_arr) = fallback_res.json::<serde_json::Value>().await {
                        if let Some(arr) = json_arr.as_array() {
                            for (idx, item) in arr.iter().take(30).enumerate() {
                                if let Some(ver) = item.get("version").and_then(|v| v.as_str()) {
                                    let stable = item.get("stable").and_then(|s| s.as_bool()).unwrap_or(true);
                                    list.push(ModloaderVersionItem {
                                        version: ver.to_string(),
                                        stable,
                                        recommended: idx == 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            Ok(list)
        }
        "quilt" => {
            let url = if mc_version != "latest" && !mc_version.is_empty() {
                format!("https://meta.quiltmc.org/v3/versions/loader/{}", mc_version)
            } else {
                "https://meta.quiltmc.org/v3/versions/loader".to_string()
            };

            let mut list: Vec<ModloaderVersionItem> = Vec::new();
            if let Ok(res) = client.get(&url).send().await {
                if res.status().is_success() {
                    if let Ok(json_arr) = res.json::<serde_json::Value>().await {
                        if let Some(arr) = json_arr.as_array() {
                            for (idx, item) in arr.iter().take(30).enumerate() {
                                let ver = item.get("loader")
                                    .and_then(|l| l.get("version"))
                                    .and_then(|v| v.as_str())
                                    .or_else(|| item.get("version").and_then(|v| v.as_str()));
                                if let Some(v_str) = ver {
                                    list.push(ModloaderVersionItem {
                                        version: v_str.to_string(),
                                        stable: true,
                                        recommended: idx == 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            if list.is_empty() {
                if let Ok(fallback_res) = client.get("https://meta.quiltmc.org/v3/versions/loader").send().await {
                    if let Ok(json_arr) = fallback_res.json::<serde_json::Value>().await {
                        if let Some(arr) = json_arr.as_array() {
                            for (idx, item) in arr.iter().take(30).enumerate() {
                                if let Some(ver) = item.get("version").and_then(|v| v.as_str()) {
                                    list.push(ModloaderVersionItem {
                                        version: ver.to_string(),
                                        stable: true,
                                        recommended: idx == 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            Ok(list)
        }
        "forge" => {
            let mut list: Vec<ModloaderVersionItem> = Vec::new();
            if let Ok(res) = client.get("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json").send().await {
                if let Ok(json_val) = res.json::<serde_json::Value>().await {
                    if let Some(promos) = json_val.get("promos").and_then(|p| p.as_object()) {
                        let rec_key = format!("{}-recommended", mc_version);
                        let latest_key = format!("{}-latest", mc_version);

                        if let Some(rec) = promos.get(&rec_key).and_then(|v| v.as_str()) {
                            list.push(ModloaderVersionItem {
                                version: rec.to_string(),
                                stable: true,
                                recommended: true,
                            });
                        }
                        if let Some(latest) = promos.get(&latest_key).and_then(|v| v.as_str()) {
                            if !list.iter().any(|item| item.version == latest) {
                                list.push(ModloaderVersionItem {
                                    version: latest.to_string(),
                                    stable: true,
                                    recommended: list.is_empty(),
                                });
                            }
                        }
                    }
                }
            }
            if list.is_empty() {
                list.push(ModloaderVersionItem {
                    version: "47.4.20".to_string(),
                    stable: true,
                    recommended: true,
                });
            }
            Ok(list)
        }
        "neoforge" => {
            let mut list: Vec<ModloaderVersionItem> = Vec::new();
            if let Ok(res) = client.get("https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge").send().await {
                if let Ok(json_val) = res.json::<serde_json::Value>().await {
                    if let Some(versions) = json_val.get("versions").and_then(|v| v.as_array()) {
                        let mc_prefix = if mc_version.starts_with("1.") {
                            let parts: Vec<&str> = mc_version.split('.').collect();
                            if parts.len() >= 2 {
                                format!("{}.{}", parts[1], parts.get(2).unwrap_or(&"0"))
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };

                        let filtered: Vec<&str> = versions.iter()
                            .filter_map(|v| v.as_str())
                            .filter(|v| mc_prefix.is_empty() || v.starts_with(&mc_prefix))
                            .collect();

                        for (idx, ver) in filtered.into_iter().rev().take(30).enumerate() {
                            list.push(ModloaderVersionItem {
                                version: ver.to_string(),
                                stable: true,
                                recommended: idx == 0,
                            });
                        }
                    }
                }
            }
            Ok(list)
        }
        _ => Ok(Vec::new()),
    }
}

/// Resolves the newest stable Minecraft release version that is supported by Fabric Loader,
/// Mojang's official version manifest, AND the core optimization mods (`fabric-api`, `sodium`, `lithium`, `modmenu`).
async fn resolve_latest_compatible_mc_version(
    client: &reqwest::Client,
    slugs: &[String],
) -> Result<String, String> {
    let tags: Vec<ModrinthGameVersionTag> = client
        .get("https://api.modrinth.com/v2/tag/game_version")
        .send()
        .await
        .map_err(|e| format!("Failed to query Modrinth game versions: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse Modrinth game versions: {}", e))?;

    let release_order: Vec<String> = tags
        .into_iter()
        .filter(|t| t.version_type == "release")
        .map(|t| t.version)
        .collect();

    // Also fetch Mojang's official manifest so we only pick versions published in Piston Meta
    let mojang_manifest: serde_json::Value = client
        .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Mojang version manifest: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Invalid Mojang version manifest: {}", e))?;

    let mojang_versions: HashSet<String> = mojang_manifest["versions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v["id"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Check core anchor mods to guarantee full compatibility
    let anchor_slugs: Vec<&str> = if slugs.is_empty() {
        vec!["fabric-api", "sodium"]
    } else {
        slugs
            .iter()
            .map(|s| s.as_str())
            .filter(|s| {
                matches!(
                    *s,
                    "fabric-api" | "sodium" | "lithium" | "ferrite-core" | "modmenu" | "xaeros-minimap"
                )
            })
            .collect()
    };

    let mut supported_sets: Vec<HashSet<String>> = Vec::new();
    for slug in anchor_slugs {
        let url = format!(
            "https://api.modrinth.com/v2/project/{}/version?loaders=%5B%22fabric%22%5D",
            slug
        );
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(vers) = resp.json::<Vec<ModrinthVersion>>().await {
                    let mut set = HashSet::new();
                    for v in vers {
                        for gv in v.game_versions {
                            set.insert(gv);
                        }
                    }
                    if !set.is_empty() {
                        supported_sets.push(set);
                    }
                }
            }
        }
    }

    for candidate in &release_order {
        if !mojang_versions.contains(candidate) {
            continue;
        }
        if supported_sets.iter().all(|s| s.contains(candidate)) {
            return Ok(candidate.clone());
        }
    }

    // Fallback to latest Mojang release if no anchor intersection found
    release_order
        .into_iter()
        .find(|v| mojang_versions.contains(v))
        .ok_or_else(|| "Could not resolve a stable Minecraft release version.".to_string())
}

pub async fn sync_profile_instance(
    app: &AppHandle,
    profile_id: &str,
    force_repair: bool,
) -> Result<String, String> {
    let mut state = load_profiles_state();
    let profile_idx = state
        .profiles
        .iter()
        .position(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let profile = state.profiles[profile_idx].clone();
    if profile.id == CREATE_SERVER_PROFILE_ID {
        return Err("Use sync_modpack for the Create Server profile.".to_string());
    }

    let client = build_http_client()?;
    let instance_dir = PathBuf::from(&profile.instance_dir);
    let mods_dir = instance_dir.join("mods");
    let config_dir = instance_dir.join("config");
    let saves_dir = instance_dir.join("saves");
    fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&saves_dir).map_err(|e| e.to_string())?;

    // 1. Ensure Java 21 Runtime
    let runtime_dir = crate::downloader::get_runtime_dir();
    let _ = crate::downloader::ensure_java(app, &runtime_dir).await?;

    // 2. Resolve target Minecraft version
    emit_progress(app, "Resolving latest compatible Minecraft & Fabric versions...", 0, 100, 5.0, 0);
    let target_mc = if profile.mc_version == "latest" {
        if let Some(existing) = &profile.resolved_mc_version {
            if !force_repair {
                existing.clone()
            } else {
                resolve_latest_compatible_mc_version(&client, &profile.modrinth_slugs).await?
            }
        } else {
            resolve_latest_compatible_mc_version(&client, &profile.modrinth_slugs).await?
        }
    } else {
        profile.mc_version.clone()
    };

    // 3. Fetch Mojang Version Manifest & Version JSON
    emit_progress(
        app,
        &format!("Fetching Minecraft {} metadata...", target_mc),
        10,
        100,
        10.0,
        0,
    );
    let manifest_json: serde_json::Value = client
        .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Mojang manifest: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Invalid Mojang manifest JSON: {}", e))?;

    let version_url = manifest_json["versions"]
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|v| v["id"].as_str() == Some(&target_mc))
                .and_then(|v| v["url"].as_str())
        })
        .ok_or_else(|| format!("Minecraft version '{}' not found in Mojang manifest.", target_mc))?
        .to_string();

    let mc_version_data: serde_json::Value = client
        .get(&version_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Minecraft {} JSON: {}", target_mc, e))?
        .json()
        .await
        .map_err(|e| format!("Invalid Minecraft {} JSON: {}", target_mc, e))?;

    let shared_meta = get_shared_meta_dir();
    let shared_libs_dir = shared_meta.join("libraries");
    let shared_assets_dir = shared_meta.join("assets");
    let natives_dir = instance_dir.join("natives");
    fs::create_dir_all(&shared_libs_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&shared_assets_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&natives_dir).map_err(|e| e.to_string())?;

    // 4. Download & verify Vanilla client.jar
    let client_jar_path = instance_dir.join(format!("minecraft-{}-client.jar", target_mc));
    let client_dl = &mc_version_data["downloads"]["client"];
    let client_url = client_dl["url"]
        .as_str()
        .ok_or_else(|| "Missing client.jar download URL in Mojang metadata.".to_string())?;
    let client_sha1 = client_dl["sha1"].as_str().unwrap_or("");

    if force_repair || !verify_file_sha1(&client_jar_path, client_sha1) {
        emit_progress(
            app,
            &format!("Downloading Minecraft {} client.jar...", target_mc),
            0,
            100,
            15.0,
            0,
        );
        download_file_verified(&client, client_url, &client_jar_path, Some(client_sha1), None).await?;
    }

    // 5. Download & verify Vanilla libraries + extract native libraries if any
    let vanilla_libs: Vec<VersionLibrary> = serde_json::from_value(
        mc_version_data["libraries"].clone(),
    )
    .map_err(|e| format!("Failed to parse vanilla libraries: {}", e))?;

    let mut classpath_entries: Vec<PathBuf> = Vec::new();
    let mut seen_paths: HashSet<PathBuf> = HashSet::new();

    let applicable_vanilla: Vec<&VersionLibrary> = vanilla_libs
        .iter()
        .filter(|lib| is_library_for_current_platform(lib))
        .collect();

    let total_vlibs = applicable_vanilla.len();
    for (idx, lib) in applicable_vanilla.iter().enumerate() {
        if let Some(artifact) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            if let (Some(rel_path), Some(url)) = (&artifact.path, &artifact.url) {
                let target_lib = shared_libs_dir.join(rel_path);
                let expected_sha1 = artifact.sha1.as_deref().unwrap_or("");
                if force_repair || !verify_file_sha1(&target_lib, expected_sha1) {
                    let pct = 20.0 + (idx as f64 / total_vlibs.max(1) as f64) * 20.0;
                    emit_progress(
                        app,
                        &format!("Downloading library ({}/{})...", idx + 1, total_vlibs),
                        idx as u64,
                        total_vlibs as u64,
                        pct,
                        0,
                    );
                    download_file_verified(
                        &client,
                        url,
                        &target_lib,
                        if expected_sha1.is_empty() {
                            None
                        } else {
                            Some(expected_sha1)
                        },
                        None,
                    )
                    .await?;
                }
                if target_lib.exists() && seen_paths.insert(target_lib.clone()) {
                    classpath_entries.push(target_lib);
                }
            }
        }
    }

    // 6. Download & verify Asset Index and Vanilla Assets
    let asset_index_id = mc_version_data["assetIndex"]["id"]
        .as_str()
        .unwrap_or(&target_mc)
        .to_string();
    let asset_index_url = mc_version_data["assetIndex"]["url"]
        .as_str()
        .ok_or_else(|| "Missing assetIndex URL in Mojang metadata.".to_string())?;
    let asset_index_sha1 = mc_version_data["assetIndex"]["sha1"].as_str().unwrap_or("");

    let indexes_dir = shared_assets_dir.join("indexes");
    let objects_dir = shared_assets_dir.join("objects");
    fs::create_dir_all(&indexes_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&objects_dir).map_err(|e| e.to_string())?;

    let index_file = indexes_dir.join(format!("{}.json", asset_index_id));
    if force_repair || !verify_file_sha1(&index_file, asset_index_sha1) {
        download_file_verified(
            &client,
            asset_index_url,
            &index_file,
            if asset_index_sha1.is_empty() {
                None
            } else {
                Some(asset_index_sha1)
            },
            None,
        )
        .await?;
    }

    sync_shared_assets(app, &client, &index_file, &objects_dir).await?;

    // 7. Resolve & Download Fabric Loader Libraries (if loader == "fabric")
    let mut resolved_loader_version = String::from("vanilla");
    let mut main_class = mc_version_data["mainClass"]
        .as_str()
        .unwrap_or("net.minecraft.client.main.Main")
        .to_string();

    if profile.loader == "fabric" {
        emit_progress(
            app,
            &format!("Resolving Fabric Loader for Minecraft {}...", target_mc),
            65,
            100,
            65.0,
            0,
        );
        let loader_list_url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}",
            url_encode(&target_mc)
        );
        let loader_entries: serde_json::Value = client
            .get(&loader_list_url)
            .send()
            .await
            .map_err(|e| format!("Failed to query Fabric Meta API: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Invalid Fabric Meta response: {}", e))?;

        let chosen_loader = if let Some(explicit) = &profile.loader_version {
            explicit.clone()
        } else {
            let arr = loader_entries
                .as_array()
                .ok_or_else(|| format!("No Fabric Loader versions found for MC {}.", target_mc))?;
            arr.iter()
                .find(|entry| entry["loader"]["stable"].as_bool() == Some(true))
                .or_else(|| arr.first())
                .and_then(|entry| entry["loader"]["version"].as_str())
                .ok_or_else(|| format!("No valid Fabric Loader found for MC {}.", target_mc))?
                .to_string()
        };
        resolved_loader_version = chosen_loader.clone();

        let profile_url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
            url_encode(&target_mc),
            url_encode(&chosen_loader)
        );
        let fabric_profile: serde_json::Value = client
            .get(&profile_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch Fabric launch profile: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Invalid Fabric launch profile JSON: {}", e))?;

        if let Some(mc) = fabric_profile["mainClass"].as_str() {
            main_class = mc.to_string();
        }

        if let Some(flibs) = fabric_profile["libraries"].as_array() {
            let total_flibs = flibs.len();
            for (i, flib) in flibs.iter().enumerate() {
                let name = match flib["name"].as_str() {
                    Some(n) => n,
                    None => continue,
                };
                let base_url = flib["url"]
                    .as_str()
                    .unwrap_or("https://maven.fabricmc.net/");
                let rel_path = match maven_to_path(name) {
                    Some(p) => p,
                    None => continue,
                };
                let full_url = format!("{}{}", base_url.trim_end_matches('/'), format!("/{}", rel_path));
                let expected_sha1 = flib["sha1"].as_str().unwrap_or("");
                let target_lib = shared_libs_dir.join(&rel_path);

                if force_repair || !verify_file_sha1(&target_lib, expected_sha1) {
                    let pct = 65.0 + (i as f64 / total_flibs.max(1) as f64) * 15.0;
                    emit_progress(
                        app,
                        &format!("Downloading Fabric library ({}/{})...", i + 1, total_flibs),
                        i as u64,
                        total_flibs as u64,
                        pct,
                        0,
                    );
                    download_file_verified(
                        &client,
                        &full_url,
                        &target_lib,
                        if expected_sha1.is_empty() {
                            None
                        } else {
                            Some(expected_sha1)
                        },
                        None,
                    )
                    .await?;
                }
                if target_lib.exists() && seen_paths.insert(target_lib.clone()) {
                    classpath_entries.push(target_lib);
                }
            }
        }
    }

    // Add Vanilla client.jar last on classpath
    if seen_paths.insert(client_jar_path.clone()) {
        classpath_entries.push(client_jar_path.clone());
    }

    // Save Fabric/Vanilla Launch Meta JSON
    let launch_meta = FabricLaunchMeta {
        mc_version: target_mc.clone(),
        loader_version: resolved_loader_version.clone(),
        main_class,
        asset_index: asset_index_id,
        assets_dir: shared_assets_dir.to_string_lossy().to_string(),
        natives_dir: natives_dir.to_string_lossy().to_string(),
        client_jar: client_jar_path.to_string_lossy().to_string(),
        classpath: classpath_entries
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    };
    let meta_json = serde_json::to_string_pretty(&launch_meta).map_err(|e| e.to_string())?;
    fs::write(instance_dir.join("fabric_launch_meta.json"), meta_json).map_err(|e| e.to_string())?;

    // 8. Resolve & Download Modrinth Mods for this Profile
    if profile.loader == "fabric" && !profile.modrinth_slugs.is_empty() {
        sync_modrinth_mods_for_instance(
            app,
            &client,
            &mods_dir,
            &target_mc,
            &profile.loader,
            &profile.modrinth_slugs,
            force_repair,
        )
        .await?;
    }

    // Ensure the exact Java runtime version required by Minecraft and all installed mods
    let min_java = crate::downloader::determine_required_java_version(&instance_dir, &target_mc);
    let effective_java = profile.java_version.unwrap_or(min_java).max(min_java);
    let runtime_dir = crate::downloader::get_runtime_dir();
    let _ = crate::downloader::ensure_java_version(app, &runtime_dir, effective_java).await?;

    // 9. Update profile metadata in profiles.json
    state.profiles[profile_idx].java_version = Some(effective_java);
    state.profiles[profile_idx].resolved_mc_version = Some(target_mc.clone());
    if profile.loader == "fabric" {
        state.profiles[profile_idx].loader_version = Some(resolved_loader_version.clone());
    }
    save_profiles_state(&state)?;

    emit_progress(app, "Instance ready to launch!", 100, 100, 100.0, 0);
    Ok(format!("MC {} ({})", target_mc, resolved_loader_version))
}

async fn sync_shared_assets(
    app: &AppHandle,
    client: &reqwest::Client,
    index_file: &Path,
    objects_dir: &Path,
) -> Result<(), String> {
    let content = fs::read_to_string(index_file).map_err(|e| e.to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let mut missing: Vec<(String, u64)> = Vec::new();
    if let Some(objs) = parsed["objects"].as_object() {
        for (_name, obj) in objs {
            if let (Some(hash), Some(size)) = (obj["hash"].as_str(), obj["size"].as_u64()) {
                if hash.len() >= 2 {
                    let prefix = &hash[0..2];
                    let file_path = objects_dir.join(prefix).join(hash);
                    let valid = file_path
                        .metadata()
                        .map(|m| m.len() == size)
                        .unwrap_or(false);
                    if !valid {
                        missing.push((hash.to_string(), size));
                    }
                }
            }
        }
    }

    if missing.is_empty() {
        return Ok(());
    }

    let total_missing = missing.len();
    let total_bytes: u64 = missing.iter().map(|(_, s)| *s).sum();
    let downloaded_bytes = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let completed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let start = Instant::now();

    let results: Vec<Result<(), String>> = futures_util::stream::iter(missing)
        .map(|(hash, _size)| {
            let client = client.clone();
            let objects_dir = objects_dir.to_path_buf();
            let app = app.clone();
            let downloaded_bytes = downloaded_bytes.clone();
            let completed = completed.clone();
            async move {
                let prefix = &hash[0..2];
                let sub = objects_dir.join(prefix);
                let _ = fs::create_dir_all(&sub);
                let target = sub.join(&hash);
                let url = format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    prefix, hash
                );

                download_file_verified(&client, &url, &target, Some(&hash), None).await?;
                let len = target.metadata().map(|m| m.len()).unwrap_or(0);
                let cur_b = downloaded_bytes.fetch_add(len, std::sync::atomic::Ordering::Relaxed) + len;
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                if done % 20 == 0 || done == total_missing {
                    let elapsed = start.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 {
                        (cur_b as f64 / elapsed) as u64
                    } else {
                        0
                    };
                    let pct = 40.0 + (done as f64 / total_missing as f64) * 24.0;
                    emit_progress(
                        &app,
                        &format!("Downloading Minecraft assets ({}/{})...", done, total_missing),
                        cur_b,
                        total_bytes,
                        pct,
                        speed,
                    );
                }
                Ok(())
            }
        })
        .buffer_unordered(24)
        .collect()
        .await;

    let failed: Vec<String> = results.into_iter().filter_map(|r| r.err()).collect();
    if !failed.is_empty() {
        return Err(format!(
            "Failed to download {} asset(s). First error: {}",
            failed.len(),
            failed[0]
        ));
    }
    Ok(())
}

fn load_modrinth_index(mods_dir: &Path) -> HashMap<String, InstalledModMetadata> {
    let path = mods_dir.join(".modrinth_index.json");
    if path.exists() {
        if let Ok(s) = fs::read_to_string(&path) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, InstalledModMetadata>>(&s) {
                return map;
            }
        }
    }
    HashMap::new()
}

fn save_modrinth_index(mods_dir: &Path, index: &HashMap<String, InstalledModMetadata>) {
    let path = mods_dir.join(".modrinth_index.json");
    if let Ok(s) = serde_json::to_string_pretty(index) {
        let _ = fs::write(path, s);
    }
}

async fn sync_modrinth_mods_for_instance(
    app: &AppHandle,
    client: &reqwest::Client,
    mods_dir: &Path,
    mc_version: &str,
    loader: &str,
    slugs: &[String],
    force_repair: bool,
) -> Result<(), String> {
    let mut index = load_modrinth_index(mods_dir);
    let mut visited_projects: HashSet<String> = HashSet::new();
    let mut queue: Vec<String> = slugs.to_vec();

    let mut step = 0usize;
    while let Some(slug_or_id) = queue.pop() {
        if !visited_projects.insert(slug_or_id.clone()) {
            continue;
        }
        step += 1;
        let pct = (80.0 + (step as f64 * 1.5)).min(98.0);
        emit_progress(
            app,
            &format!("Resolving Modrinth mod '{}' for MC {}...", slug_or_id, mc_version),
            step as u64,
            slugs.len().max(step) as u64,
            pct,
            0,
        );

        let proj_url = format!("https://api.modrinth.com/v2/project/{}", slug_or_id);
        let project: ModrinthProject = match client.get(&proj_url).send().await {
            Ok(r) if r.status().is_success() => match r.json().await {
                Ok(p) => p,
                Err(_) => continue,
            },
            _ => continue,
        };
        visited_projects.insert(project.id.clone());
        visited_projects.insert(project.slug.clone());

        let ver_url = format!(
            "https://api.modrinth.com/v2/project/{}/version?loaders=%5B%22{}%22%5D&game_versions=%5B%22{}%22%5D",
            project.id,
            url_encode(loader),
            url_encode(mc_version)
        );

        let versions: Vec<ModrinthVersion> = match client.get(&ver_url).send().await {
            Ok(r) if r.status().is_success() => match r.json().await {
                Ok(v) => v,
                Err(_) => continue,
            },
            _ => continue,
        };

        let chosen = match versions.first() {
            Some(v) => v,
            None => {
                eprintln!(
                    "[Modrinth] No compatible {} version of '{}' found for MC {}",
                    loader, project.title, mc_version
                );
                continue;
            }
        };

        // Queue required Modrinth dependencies automatically
        for dep in &chosen.dependencies {
            if dep.dependency_type == "required" {
                if let Some(dep_id) = &dep.project_id {
                    if !visited_projects.contains(dep_id) {
                        queue.push(dep_id.clone());
                    }
                }
            }
        }

        let file = chosen
            .files
            .iter()
            .find(|f| f.primary == Some(true))
            .or_else(|| chosen.files.first());

        if let Some(mod_file) = file {
            let target_jar = mods_dir.join(&mod_file.filename);
            let disabled_jar = mods_dir.join(format!("{}.disabled", mod_file.filename));

            // Remove older versions of the same project slug if filename changed
            if let Some(old_meta) = index.get(&project.slug) {
                if old_meta.file_name != mod_file.filename {
                    let _ = fs::remove_file(mods_dir.join(&old_meta.file_name));
                    let _ = fs::remove_file(mods_dir.join(format!("{}.disabled", old_meta.file_name)));
                }
            }

            let active_path = if disabled_jar.exists() && !target_jar.exists() {
                &disabled_jar
            } else {
                &target_jar
            };

            let expected_sha1 = mod_file.hashes.sha1.as_deref();
            let expected_sha512 = mod_file.hashes.sha512.as_deref();
            let already_valid = !force_repair
                && active_path.exists()
                && expected_sha1.map_or(true, |h| verify_file_sha1(active_path, h));

            if !already_valid {
                download_file_verified(
                    client,
                    &mod_file.url,
                    active_path,
                    expected_sha1,
                    expected_sha512,
                )
                .await?;
            }

            index.insert(
                project.slug.clone(),
                InstalledModMetadata {
                    slug: project.slug.clone(),
                    title: project.title.clone(),
                    version_number: chosen.version_number.clone(),
                    file_name: mod_file.filename.clone(),
                    description: project.description.clone(),
                    icon_url: project.icon_url.clone(),
                },
            );
        }
    }

    save_modrinth_index(mods_dir, &index);
    Ok(())
}

// ============================================================================
// Per-Profile Mod Manager & Modrinth Search/Install
// ============================================================================

pub fn list_installed_mods(profile_id: &str) -> Result<Vec<InstalledModInfo>, String> {
    let state = load_profiles_state();
    let profile = state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let mods_dir = PathBuf::from(&profile.instance_dir).join("mods");
    if !mods_dir.exists() {
        return Ok(Vec::new());
    }

    let index = load_modrinth_index(&mods_dir);
    let mut by_filename: HashMap<String, &InstalledModMetadata> = HashMap::new();
    for meta in index.values() {
        by_filename.insert(meta.file_name.clone(), meta);
    }

    let mut mods: Vec<InstalledModInfo> = Vec::new();
    let entries = fs::read_dir(&mods_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let raw_name = entry.file_name().to_string_lossy().to_string();
        let (base_jar_name, enabled) = if raw_name.ends_with(".jar") {
            (raw_name.clone(), true)
        } else if raw_name.ends_with(".jar.disabled") {
            (raw_name.trim_end_matches(".disabled").to_string(), false)
        } else {
            continue;
        };

        let size_bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
        if let Some(meta) = by_filename.get(&base_jar_name) {
            mods.push(InstalledModInfo {
                file_name: raw_name,
                title: meta.title.clone(),
                slug: Some(meta.slug.clone()),
                version_number: meta.version_number.clone(),
                description: meta.description.clone(),
                icon_url: meta.icon_url.clone(),
                enabled,
                size_bytes,
            });
        } else {
            let clean_title = base_jar_name
                .trim_end_matches(".jar")
                .replace(['_', '-'], " ");
            mods.push(InstalledModInfo {
                file_name: raw_name,
                title: clean_title,
                slug: None,
                version_number: "Installed".to_string(),
                description: "Local mod archive in instance mods folder.".to_string(),
                icon_url: None,
                enabled,
                size_bytes,
            });
        }
    }

    mods.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(mods)
}

pub fn toggle_mod(profile_id: &str, file_name: &str, enabled: bool) -> Result<(), String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err("Invalid mod filename.".to_string());
    }
    let state = load_profiles_state();
    let profile = state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let mods_dir = PathBuf::from(&profile.instance_dir).join("mods");
    let base_name = file_name.trim_end_matches(".disabled");
    let enabled_path = mods_dir.join(base_name);
    let disabled_path = mods_dir.join(format!("{}.disabled", base_name));

    if enabled {
        if disabled_path.exists() {
            fs::rename(&disabled_path, &enabled_path).map_err(|e| e.to_string())?;
        }
    } else if enabled_path.exists() {
        fs::rename(&enabled_path, &disabled_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn remove_mod(profile_id: &str, file_name: &str) -> Result<(), String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err("Invalid mod filename.".to_string());
    }
    let mut state = load_profiles_state();
    let prof_idx = state
        .profiles
        .iter()
        .position(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let mods_dir = PathBuf::from(&state.profiles[prof_idx].instance_dir).join("mods");
    let base_name = file_name.trim_end_matches(".disabled");
    let _ = fs::remove_file(mods_dir.join(base_name));
    let _ = fs::remove_file(mods_dir.join(format!("{}.disabled", base_name)));

    let mut index = load_modrinth_index(&mods_dir);
    let mut removed_slug: Option<String> = None;
    index.retain(|slug, meta| {
        if meta.file_name == base_name {
            removed_slug = Some(slug.clone());
            false
        } else {
            true
        }
    });
    save_modrinth_index(&mods_dir, &index);

    if let Some(slug) = removed_slug {
        state.profiles[prof_idx]
            .modrinth_slugs
            .retain(|s| s != &slug);
        save_profiles_state(&state)?;
    }

    Ok(())
}

fn get_content_dir(profile_instance_dir: &str, content_type: &str) -> PathBuf {
    let base = PathBuf::from(profile_instance_dir);
    match content_type.to_lowercase().as_str() {
        "resourcepack" | "resourcepacks" => base.join("resourcepacks"),
        "shader" | "shaders" | "shaderpack" | "shaderpacks" => base.join("shaderpacks"),
        _ => base.join("mods"),
    }
}

pub fn list_installed_content(profile_id: &str, content_type: &str) -> Result<Vec<InstalledModInfo>, String> {
    let state = load_profiles_state();
    let profile = state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let target_dir = get_content_dir(&profile.instance_dir, content_type);
    if !target_dir.exists() {
        let _ = fs::create_dir_all(&target_dir);
        return Ok(Vec::new());
    }

    let is_mod = matches!(content_type.to_lowercase().as_str(), "mod" | "mods");
    let index = if is_mod { load_modrinth_index(&target_dir) } else { HashMap::new() };
    let mut by_filename: HashMap<String, &InstalledModMetadata> = HashMap::new();
    for meta in index.values() {
        by_filename.insert(meta.file_name.clone(), meta);
    }

    let mut list: Vec<InstalledModInfo> = Vec::new();
    let entries = fs::read_dir(&target_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let raw_name = entry.file_name().to_string_lossy().to_string();
        let (base_name, enabled) = if raw_name.ends_with(".jar") || raw_name.ends_with(".zip") {
            (raw_name.clone(), true)
        } else if raw_name.ends_with(".disabled") {
            let stripped = raw_name.trim_end_matches(".disabled").to_string();
            (stripped, false)
        } else {
            continue;
        };

        let size_bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
        if let Some(meta) = by_filename.get(&base_name) {
            list.push(InstalledModInfo {
                file_name: raw_name,
                title: meta.title.clone(),
                slug: Some(meta.slug.clone()),
                version_number: meta.version_number.clone(),
                description: meta.description.clone(),
                icon_url: meta.icon_url.clone(),
                enabled,
                size_bytes,
            });
        } else {
            let clean_title = base_name
                .trim_end_matches(".jar")
                .trim_end_matches(".zip")
                .replace(['_', '-'], " ");
            list.push(InstalledModInfo {
                file_name: raw_name,
                title: clean_title,
                slug: None,
                version_number: "Installed".to_string(),
                description: format!("Local {} archive.", content_type),
                icon_url: None,
                enabled,
                size_bytes,
            });
        }
    }

    list.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(list)
}

pub fn toggle_content(profile_id: &str, content_type: &str, file_name: &str, enabled: bool) -> Result<(), String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err("Invalid filename.".to_string());
    }
    let state = load_profiles_state();
    let profile = state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let target_dir = get_content_dir(&profile.instance_dir, content_type);
    let base_name = file_name.trim_end_matches(".disabled");
    let enabled_path = target_dir.join(base_name);
    let disabled_path = target_dir.join(format!("{}.disabled", base_name));

    if enabled {
        if disabled_path.exists() {
            fs::rename(&disabled_path, &enabled_path).map_err(|e| e.to_string())?;
        }
    } else if enabled_path.exists() {
        fs::rename(&enabled_path, &disabled_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn remove_content(profile_id: &str, content_type: &str, file_name: &str) -> Result<(), String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err("Invalid filename.".to_string());
    }
    let mut state = load_profiles_state();
    let prof_idx = state
        .profiles
        .iter()
        .position(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let target_dir = get_content_dir(&state.profiles[prof_idx].instance_dir, content_type);
    let base_name = file_name.trim_end_matches(".disabled");
    let _ = fs::remove_file(target_dir.join(base_name));
    let _ = fs::remove_file(target_dir.join(format!("{}.disabled", base_name)));

    if matches!(content_type.to_lowercase().as_str(), "mod" | "mods") {
        let mut index = load_modrinth_index(&target_dir);
        let mut removed_slug: Option<String> = None;
        index.retain(|slug, meta| {
            if meta.file_name == base_name {
                removed_slug = Some(slug.clone());
                false
            } else {
                true
            }
        });
        save_modrinth_index(&target_dir, &index);

        if let Some(slug) = removed_slug {
            state.profiles[prof_idx].modrinth_slugs.retain(|s| s != &slug);
            save_profiles_state(&state)?;
        }
    }

    Ok(())
}

pub fn resolve_profile_target_mc(profile: Option<&ModpackProfile>, explicit_mc: Option<String>) -> String {
    if let Some(m) = explicit_mc.filter(|v| v != "latest" && !v.is_empty()) {
        return m;
    }
    if let Some(p) = profile {
        if let Some(ref r) = p.resolved_mc_version {
            if r != "latest" && !r.is_empty() {
                return r.clone();
            }
        }
        if p.mc_version != "latest" && !p.mc_version.is_empty() {
            return p.mc_version.clone();
        }
    }
    "1.21.4".to_string()
}

pub async fn search_content(
    query: String,
    content_type: String,
    source: String,
    mc_version: Option<String>,
    loader: Option<String>,
    curseforge_key: Option<String>,
    profile_id: String,
) -> Result<Vec<ContentSearchResult>, String> {
    let state = load_profiles_state();
    let profile = state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .cloned();

    let target_mc = resolve_profile_target_mc(profile.as_ref(), mc_version);

    let resolved_loader = loader
        .or_else(|| profile.as_ref().map(|p| p.loader.clone()))
        .unwrap_or_else(|| "fabric".to_string());

    let target_dir = profile
        .as_ref()
        .map(|p| get_content_dir(&p.instance_dir, &content_type));

    let installed_files: HashSet<String> = target_dir
        .as_ref()
        .and_then(|d| fs::read_dir(d).ok())
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_lowercase())
                .collect()
        })
        .unwrap_or_default();

    let client = build_http_client()?;
    let c_type = content_type.to_lowercase();
    let src = source.to_lowercase();

    // Vanilla profiles cannot run mods
    if (c_type == "mod" || c_type == "mods") && resolved_loader.eq_ignore_ascii_case("vanilla") {
        return Ok(Vec::new());
    }

    if src == "curseforge" {
        let key = curseforge_key
            .filter(|k| !k.trim().is_empty())
            .ok_or_else(|| {
                "CurseForge API Key not configured. Please add your free CurseForge API key in Settings -> CurseForge to enable CurseForge search and downloads.".to_string()
            })?;

        let class_id = match c_type.as_str() {
            "resourcepack" | "resourcepacks" => 12,
            "shader" | "shaders" | "shaderpack" => 6552,
            _ => 6, // Mods
        };

        let mut url = format!(
            "https://api.curseforge.com/v1/mods/search?gameId=432&classId={}&pageSize=20&sortField=2&sortOrder=desc",
            class_id
        );
        if !query.trim().is_empty() {
            url.push_str(&format!("&searchFilter={}", url_encode(query.trim())));
        }
        url.push_str(&format!("&gameVersion={}", url_encode(&target_mc)));
        if class_id == 6 {
            let loader_type = match resolved_loader.as_str() {
                "forge" => Some(1),
                "fabric" => Some(4),
                "quilt" => Some(5),
                "neoforge" => Some(6),
                _ => None,
            };
            if let Some(lt) = loader_type {
                url.push_str(&format!("&modLoaderType={}", lt));
            }
        }

        let resp: serde_json::Value = client
            .get(&url)
            .header("x-api-key", key.trim())
            .send()
            .await
            .map_err(|e| format!("CurseForge search failed: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Invalid CurseForge response: {}", e))?;

        let mut results = Vec::new();
        if let Some(data) = resp["data"].as_array() {
            for item in data {
                let project_id = item["id"].to_string();
                let slug = item["slug"].as_str().unwrap_or("").to_string();
                let title = item["name"].as_str().unwrap_or("Unknown").to_string();
                let description = item["summary"].as_str().unwrap_or("").to_string();
                let author = item["authors"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|a| a["name"].as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                let downloads = item["downloadCount"].as_u64().unwrap_or(0);
                let icon_url = item["logo"]["thumbnailUrl"]
                    .as_str()
                    .or_else(|| item["logo"]["url"].as_str())
                    .map(|s| s.to_string());

                let slug_lower = slug.to_lowercase();
                let installed = installed_files.iter().any(|f| f.contains(&slug_lower));

                results.push(ContentSearchResult {
                    project_id,
                    slug,
                    title,
                    description,
                    author,
                    downloads,
                    icon_url,
                    installed,
                    content_type: c_type.clone(),
                    source: "curseforge".to_string(),
                });
            }
        }
        Ok(results)
    } else {
        // Modrinth search
        let project_type = match c_type.as_str() {
            "resourcepack" | "resourcepacks" => "resourcepack",
            "shader" | "shaders" | "shaderpack" => "shader",
            _ => "mod",
        };

        let mut facets_arr = vec![format!("[\"project_type:{}\"]", project_type)];
        facets_arr.push(format!("[\"versions:{}\"]", target_mc));

        if project_type == "mod" {
            match resolved_loader.as_str() {
                "quilt" => facets_arr.push("[\"categories:quilt\",\"categories:fabric\"]".to_string()),
                "fabric" => facets_arr.push("[\"categories:fabric\"]".to_string()),
                "forge" => facets_arr.push("[\"categories:forge\"]".to_string()),
                "neoforge" => {
                    if target_mc == "1.20.1" {
                        facets_arr.push("[\"categories:neoforge\",\"categories:forge\"]".to_string());
                    } else {
                        facets_arr.push("[\"categories:neoforge\"]".to_string());
                    }
                }
                l => facets_arr.push(format!("[\"categories:{}\"]", l)),
            }
        }
        let facets_str = format!("[{}]", facets_arr.join(","));

        let url = format!(
            "https://api.modrinth.com/v2/search?query={}&facets={}&limit=20",
            url_encode(query.trim()),
            url_encode(&facets_str)
        );

        let resp: serde_json::Value = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Modrinth search failed: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Invalid Modrinth response: {}", e))?;

        let mut results = Vec::new();
        if let Some(hits) = resp["hits"].as_array() {
            for hit in hits {
                let project_id = hit["project_id"].as_str().unwrap_or("").to_string();
                let slug = hit["slug"].as_str().unwrap_or("").to_string();
                if slug.is_empty() {
                    continue;
                }
                let title = hit["title"].as_str().unwrap_or("Unknown").to_string();
                let description = hit["description"].as_str().unwrap_or("").to_string();
                let author = hit["author"].as_str().unwrap_or("Unknown").to_string();
                let downloads = hit["downloads"].as_u64().unwrap_or(0);
                let icon_url = hit["icon_url"].as_str().map(|s| s.to_string());

                let slug_lower = slug.to_lowercase();
                let installed = installed_files.iter().any(|f| f.contains(&slug_lower));

                results.push(ContentSearchResult {
                    project_id,
                    slug,
                    title,
                    description,
                    author,
                    downloads,
                    icon_url,
                    installed,
                    content_type: c_type.clone(),
                    source: "modrinth".to_string(),
                });
            }
        }
        Ok(results)
    }
}

pub async fn install_content(
    app: &AppHandle,
    profile_id: &str,
    project_id: &str,
    _version_id: Option<&str>,
    content_type: &str,
    source: &str,
    curseforge_key: Option<&str>,
) -> Result<String, String> {
    let mut state = load_profiles_state();
    let prof_idx = state
        .profiles
        .iter()
        .position(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let profile = state.profiles[prof_idx].clone();
    let is_mod = matches!(content_type.to_lowercase().as_str(), "mod" | "mods");

    // Guard: Vanilla profiles cannot run mods
    if is_mod && profile.loader.eq_ignore_ascii_case("vanilla") {
        return Err(format!(
            "Cannot install mod into '{}': Vanilla Minecraft does not support mods. Please create or switch to a Forge, Fabric, Quilt, or NeoForge profile.",
            profile.name
        ));
    }

    let client = build_http_client()?;
    let mc_ver = resolve_profile_target_mc(Some(&profile), None);

    let target_dir = get_content_dir(&profile.instance_dir, content_type);
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    if source.eq_ignore_ascii_case("curseforge") {
        let key = curseforge_key
            .filter(|k| !k.trim().is_empty())
            .ok_or_else(|| "CurseForge API Key required for CurseForge downloads.".to_string())?;

        let files_url = format!("https://api.curseforge.com/v1/mods/{}/files", project_id);
        let resp: serde_json::Value = client
            .get(&files_url)
            .header("x-api-key", key.trim())
            .send()
            .await
            .map_err(|e| format!("Failed to fetch CurseForge files: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Invalid CurseForge files JSON: {}", e))?;

        let files = resp["data"]
            .as_array()
            .ok_or_else(|| "No files found for CurseForge project".to_string())?;

        // Strict compatibility matching for CurseForge files
        let compatible_file = files.iter().find(|f| {
            let gv = match f["gameVersions"].as_array() {
                Some(arr) => arr,
                None => return false,
            };
            let has_mc = gv.iter().any(|v| v.as_str() == Some(&mc_ver));
            if !has_mc {
                return false;
            }
            if is_mod {
                let has_loader = gv.iter().any(|v| {
                    let s = v.as_str().unwrap_or("");
                    match profile.loader.as_str() {
                        "fabric" => s.eq_ignore_ascii_case("fabric"),
                        "quilt" => s.eq_ignore_ascii_case("quilt") || s.eq_ignore_ascii_case("fabric"),
                        "forge" => s.eq_ignore_ascii_case("forge"),
                        "neoforge" => s.eq_ignore_ascii_case("neoforge") || (mc_ver == "1.20.1" && s.eq_ignore_ascii_case("forge")),
                        _ => false,
                    }
                });
                if !has_loader {
                    return false;
                }
            }
            true
        });

        let chosen_file = compatible_file.ok_or_else(|| {
            format!(
                "Cannot install '{}': Incompatible with profile '{}'. No CurseForge file matches Minecraft {} ({}).",
                project_id, profile.name, mc_ver, profile.loader.to_uppercase()
            )
        })?;

        let file_id = chosen_file["id"].as_u64().unwrap_or(0);
        let file_name = chosen_file["fileName"]
            .as_str()
            .ok_or_else(|| "CurseForge file missing fileName".to_string())?;

        let download_url = chosen_file["downloadUrl"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                format!(
                    "https://edge.forgecdn.net/files/{}/{}/{}",
                    file_id / 1000,
                    file_id % 1000,
                    file_name
                )
            });

        let mut sha1: Option<&str> = None;
        if let Some(hashes) = chosen_file["hashes"].as_array() {
            for h in hashes {
                if h["algo"].as_u64() == Some(1) {
                    sha1 = h["value"].as_str();
                    break;
                }
            }
        }

        emit_progress(app, &format!("Downloading {}...", file_name), 0, 100, 20.0, 0);
        let dest = target_dir.join(file_name);
        download_file_verified(&client, &download_url, &dest, sha1, None).await?;
        emit_progress(app, "Installation complete!", 100, 100, 100.0, 0);

        Ok(file_name.to_string())
    } else {
        // Modrinth install with strict compatibility
        let mut ver_url = format!("https://api.modrinth.com/v2/project/{}/version?", project_id);
        if is_mod && profile.loader != "vanilla" {
            let loaders_param = match profile.loader.as_str() {
                "quilt" => "%5B%22quilt%22%2C%22fabric%22%5D",
                "fabric" => "%5B%22fabric%22%5D",
                "forge" => "%5B%22forge%22%5D",
                "neoforge" => {
                    if mc_ver == "1.20.1" {
                        "%5B%22neoforge%22%2C%22forge%22%5D"
                    } else {
                        "%5B%22neoforge%22%5D"
                    }
                }
                l => &format!("%5B%22{}%22%5D", url_encode(l)),
            };
            ver_url.push_str(&format!("loaders={}&", loaders_param));
        }
        ver_url.push_str(&format!("game_versions=%5B%22{}%22%5D", url_encode(&mc_ver)));

        let versions_resp = client.get(&ver_url).send().await;
        let vers: Vec<ModrinthVersion> = match versions_resp {
            Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
            _ => Vec::new(),
        };

        let compatible_ver = vers.into_iter().find(|v| {
            let mc_match = v.game_versions.contains(&mc_ver);
            let loader_match = !is_mod || match profile.loader.as_str() {
                "fabric" => v.loaders.iter().any(|l| l.eq_ignore_ascii_case("fabric")),
                "quilt" => v.loaders.iter().any(|l| l.eq_ignore_ascii_case("quilt") || l.eq_ignore_ascii_case("fabric")),
                "forge" => v.loaders.iter().any(|l| l.eq_ignore_ascii_case("forge")),
                "neoforge" => v.loaders.iter().any(|l| l.eq_ignore_ascii_case("neoforge") || (mc_ver == "1.20.1" && l.eq_ignore_ascii_case("forge"))),
                _ => false,
            };
            mc_match && loader_match
        });

        let chosen_ver = compatible_ver.ok_or_else(|| {
            format!(
                "Cannot install '{}': Incompatible with profile '{}'. No Modrinth version matches Minecraft {} ({}).",
                project_id, profile.name, mc_ver, profile.loader.to_uppercase()
            )
        })?;

        let primary_file = chosen_ver
            .files
            .iter()
            .find(|f| f.primary == Some(true))
            .or_else(|| chosen_ver.files.first())
            .ok_or_else(|| "No downloadable file found in version".to_string())?;

        let dest = target_dir.join(&primary_file.filename);
        emit_progress(app, &format!("Downloading {}...", primary_file.filename), 0, primary_file.size, 20.0, 0);
        download_file_verified(
            &client,
            &primary_file.url,
            &dest,
            primary_file.hashes.sha1.as_deref(),
            primary_file.hashes.sha512.as_deref(),
        )
        .await?;

        if is_mod {
            if !state.profiles[prof_idx].modrinth_slugs.contains(&project_id.to_string()) {
                state.profiles[prof_idx].modrinth_slugs.push(project_id.to_string());
            }
            let _ = save_profiles_state(&state);
        }

        emit_progress(app, "Installation complete!", 100, 100, 100.0, 0);
        Ok(primary_file.filename.clone())
    }
}

pub async fn search_modrinth_mods(
    profile_id: &str,
    query: String,
) -> Result<Vec<ModrinthSearchResult>, String> {
    let res = search_content(
        query,
        "mod".to_string(),
        "modrinth".to_string(),
        None,
        None,
        None,
        profile_id.to_string(),
    )
    .await?;

    Ok(res
        .into_iter()
        .map(|r| ModrinthSearchResult {
            project_id: r.project_id,
            slug: r.slug,
            title: r.title,
            description: r.description,
            author: r.author,
            downloads: r.downloads,
            icon_url: r.icon_url,
            installed: r.installed,
        })
        .collect())
}

pub async fn install_modrinth_mod(
    app: &AppHandle,
    profile_id: &str,
    slug: String,
) -> Result<(), String> {
    install_content(app, profile_id, &slug, None, "mod", "modrinth", None).await?;
    Ok(())
}

// ============================================================================
// Server Creator (Beta), Divergence Check & Synchronization
// ============================================================================

pub fn is_client_only_mod(jar_path: &Path) -> bool {
    let file_name_lower = jar_path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    const CLIENT_KEYWORDS: &[&str] = &[
        "sodium", "iris", "fancymenu", "drippyloadingscreen", "drippy", "zoomify",
        "xaerominimap", "xaeroworldmap", "xaero", "rubidium", "oculus", "embeddium",
        "entityculling", "immediatelyfast", "modmenu", "cloth-config", "customskinloader",
        "presencefootsteps", "soundphysics", "resourcify", "controlling", "searchables",
        "configured", "catalogue", "appleskin", "wthit", "jade", "notenoughanimations",
        "dynamiccrosshair", "fallingleaves", "ambientenvironment", "visuality", "continuity",
        "lambdynamiclights", "craftingtweaks", "invmove", "chatheads", "replanter",
        "betterf3", "borderless", "waveycapes", "raised", "reeses-sodium-options"
    ];

    for kw in CLIENT_KEYWORDS {
        if file_name_lower.contains(kw) {
            return true;
        }
    }

    if let Ok(file) = fs::File::open(jar_path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            if let Ok(mut mod_json) = archive.by_name("fabric.mod.json") {
                let mut content = String::new();
                if mod_json.read_to_string(&mut content).is_ok() {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(env) = val.get("environment").and_then(|v| v.as_str()) {
                            if env.eq_ignore_ascii_case("client") {
                                return true;
                            }
                        }
                    }
                }
            }
            if let Ok(mut toml) = archive.by_name("META-INF/mods.toml") {
                let mut content = String::new();
                if toml.read_to_string(&mut content).is_ok() {
                    let upper = content.to_uppercase();
                    if upper.contains("CLIENT")
                        && (upper.contains("SIDE = \"CLIENT\"")
                            || upper.contains("SIDE=\"CLIENT\"")
                            || upper.contains("DISPLAYTEST=\"IGNORE_SERVER_VERSION\"")
                            || upper.contains("DISPLAYTEST = \"IGNORE_SERVER_VERSION\""))
                    {
                        return true;
                    }
                }
            }
        }
    }

    false
}

pub async fn create_server_for_profile(
    app: &AppHandle,
    profile_id: &str,
    server_name: &str,
) -> Result<ServerInstance, String> {
    let state = load_profiles_state();
    let profile = state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let mc_ver = match &profile.resolved_mc_version {
        Some(v) => v.clone(),
        None if profile.mc_version != "latest" => profile.mc_version.clone(),
        None => "1.20.1".to_string(),
    };

    let loader = if profile.loader.is_empty() {
        "fabric".to_string()
    } else {
        profile.loader.clone()
    };

    let loader_ver = if let Some(ref lv) = profile.loader_version {
        lv.clone()
    } else {
        let versions = get_modloader_versions(mc_ver.clone(), loader.clone())
            .await
            .unwrap_or_default();
        versions
            .first()
            .map(|v| v.version.clone())
            .unwrap_or_else(|| {
                if loader == "fabric" {
                    "0.16.9".to_string()
                } else if loader == "forge" {
                    "47.4.20".to_string()
                } else {
                    "latest".to_string()
                }
            })
    };

    let server_id = format!("server-{}", rand::random::<u32>());
    let server_dir = get_servers_root_dir().join(&server_id);
    fs::create_dir_all(server_dir.join("mods")).map_err(|e| e.to_string())?;

    let client = build_http_client()?;
    let server_jar_path = server_dir.join("server.jar");
    emit_progress(
        app,
        &format!("Downloading {} Dedicated Server...", loader.to_uppercase()),
        0,
        100,
        10.0,
        0,
    );

    if loader == "fabric" {
        let installer_ver = if let Ok(resp) = client
            .get("https://meta.fabricmc.net/v2/versions/installer")
            .send()
            .await
        {
            if let Ok(arr) = resp.json::<serde_json::Value>().await {
                arr.as_array()
                    .and_then(|a| a.first())
                    .and_then(|i| i["version"].as_str())
                    .unwrap_or("1.1.2")
                    .to_string()
            } else {
                "1.1.2".to_string()
            }
        } else {
            "1.1.2".to_string()
        };

        let fabric_server_url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/{}/server/jar",
            mc_ver, loader_ver, installer_ver
        );
        download_file_verified(&client, &fabric_server_url, &server_jar_path, None, None).await?;
    } else {
        let mojang_manifest: serde_json::Value = client
            .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch Mojang version manifest: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Invalid Mojang manifest: {}", e))?;

        let mut pkg_url: Option<String> = None;
        if let Some(arr) = mojang_manifest["versions"].as_array() {
            for item in arr {
                if item["id"].as_str() == Some(&mc_ver) {
                    pkg_url = item["url"].as_str().map(|s| s.to_string());
                    break;
                }
            }
        }
        let pkg_url = pkg_url.ok_or_else(|| {
            format!("Minecraft version '{}' not found in Mojang manifest", mc_ver)
        })?;
        let pkg_json: serde_json::Value = client
            .get(&pkg_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch version package: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Invalid version package json: {}", e))?;

        let srv_download = &pkg_json["downloads"]["server"];
        let srv_url = srv_download["url"]
            .as_str()
            .ok_or_else(|| "Server download URL missing".to_string())?;
        let srv_sha1 = srv_download["sha1"].as_str();
        download_file_verified(&client, srv_url, &server_jar_path, srv_sha1, None).await?;
    }

    // Filter and copy server-compatible mods from profile instance
    let profile_mods_dir = PathBuf::from(&profile.instance_dir).join("mods");
    let mut copied_mods: Vec<String> = Vec::new();
    if profile_mods_dir.exists() {
        if let Ok(entries) = fs::read_dir(&profile_mods_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.ends_with(".jar") && !is_client_only_mod(&path) {
                        let dest = server_dir.join("mods").join(&name);
                        if fs::copy(&path, &dest).is_ok() {
                            copied_mods.push(name);
                        }
                    }
                }
            }
        }
    }

    fs::write(server_dir.join("eula.txt"), "eula=true\n").map_err(|e| e.to_string())?;

    let server_props = format!(
        "# Minecraft Server Properties (Pre-configured by Create Client)\n\
        server-port=25565\n\
        motd={}\n\
        difficulty=easy\n\
        gamemode=survival\n\
        max-players=20\n\
        online-mode=true\n\
        enable-command-block=true\n\
        spawn-protection=0\n\
        view-distance=10\n\
        simulation-distance=10\n",
        server_name
    );
    fs::write(server_dir.join("server.properties"), server_props).map_err(|e| e.to_string())?;

    let run_sh = "#!/usr/bin/env bash\n\
        cd \"$(dirname \"$0\")\"\n\
        JAVA_CMD=\"java\"\n\
        if [ -n \"$JAVA_HOME\" ] && [ -x \"$JAVA_HOME/bin/java\" ]; then\n\
            JAVA_CMD=\"$JAVA_HOME/bin/java\"\n\
        fi\n\
        echo \"Starting Dedicated Server...\"\n\
        \"$JAVA_CMD\" -Xmx4G -Xms2G -jar server.jar nogui\n";
    let run_sh_path = server_dir.join("run.sh");
    fs::write(&run_sh_path, run_sh).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&run_sh_path, fs::Permissions::from_mode(0o755));
    }

    let run_bat = "@echo off\r\n\
        cd /d \"%~dp0\"\r\n\
        echo Starting Dedicated Server...\r\n\
        java -Xmx4G -Xms2G -jar server.jar nogui\r\n\
        pause\r\n";
    fs::write(server_dir.join("run.bat"), run_bat).map_err(|e| e.to_string())?;

    let manifest = ServerManifest {
        server_id: server_id.clone(),
        profile_id: profile.id.clone(),
        mc_version: mc_ver.clone(),
        loader: loader.clone(),
        loader_version: loader_ver.clone(),
        mods: copied_mods.clone(),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(server_dir.join("server_manifest.json"), manifest_json).map_err(|e| e.to_string())?;

    let mut servers = load_servers_state();
    let new_instance = ServerInstance {
        id: server_id,
        profile_id: profile.id.clone(),
        name: server_name.to_string(),
        server_dir: server_dir.to_string_lossy().to_string(),
        mc_version: mc_ver,
        loader,
        loader_version: loader_ver,
        host: "127.0.0.1:25565".to_string(),
        created_at: format!("{}", now_unix()),
        installed_mods: copied_mods,
    };
    servers.push(new_instance.clone());
    save_servers_state(&servers)?;
    emit_progress(app, "Server creation completed!", 100, 100, 100.0, 0);

    Ok(new_instance)
}

pub fn check_server_profile_divergence(server_id: &str) -> Result<ServerDivergenceInfo, String> {
    let servers = load_servers_state();
    let server = servers
        .iter()
        .find(|s| s.id == server_id)
        .ok_or_else(|| format!("Server '{}' not found", server_id))?;

    let profiles = load_profiles_state();
    let profile = profiles
        .profiles
        .iter()
        .find(|p| p.id == server.profile_id)
        .ok_or_else(|| format!("Associated profile '{}' not found", server.profile_id))?;

    let profile_mods_dir = PathBuf::from(&profile.instance_dir).join("mods");
    let mut profile_mods = HashSet::new();
    if profile_mods_dir.exists() {
        if let Ok(entries) = fs::read_dir(&profile_mods_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.ends_with(".jar") && !is_client_only_mod(&p) {
                        profile_mods.insert(name);
                    }
                }
            }
        }
    }

    let server_mods_dir = PathBuf::from(&server.server_dir).join("mods");
    let mut server_mods = HashSet::new();
    if server_mods_dir.exists() {
        if let Ok(entries) = fs::read_dir(&server_mods_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.ends_with(".jar") {
                        server_mods.insert(name);
                    }
                }
            }
        }
    }

    let mut added_to_profile: Vec<String> = profile_mods
        .difference(&server_mods)
        .cloned()
        .collect();
    let mut removed_from_profile: Vec<String> = server_mods
        .difference(&profile_mods)
        .cloned()
        .collect();

    added_to_profile.sort();
    removed_from_profile.sort();

    let loader_mismatch = profile.loader != server.loader;

    Ok(ServerDivergenceInfo {
        server_id: server.id.clone(),
        server_name: server.name.clone(),
        profile_mods_count: profile_mods.len(),
        server_mods_count: server_mods.len(),
        added_to_profile,
        removed_from_profile,
        loader_mismatch,
    })
}

pub async fn sync_server_with_profile(
    app: &AppHandle,
    server_id: &str,
) -> Result<String, String> {
    let div = check_server_profile_divergence(server_id)?;
    let mut servers = load_servers_state();
    let s_idx = servers
        .iter()
        .position(|s| s.id == server_id)
        .ok_or_else(|| format!("Server '{}' not found", server_id))?;

    let profiles = load_profiles_state();
    let profile = profiles
        .profiles
        .iter()
        .find(|p| p.id == servers[s_idx].profile_id)
        .ok_or_else(|| "Associated profile not found".to_string())?;

    let server_mods_dir = PathBuf::from(&servers[s_idx].server_dir).join("mods");
    let profile_mods_dir = PathBuf::from(&profile.instance_dir).join("mods");
    fs::create_dir_all(&server_mods_dir).map_err(|e| e.to_string())?;

    emit_progress(app, "Syncing server mods...", 0, 100, 30.0, 0);

    for rem in &div.removed_from_profile {
        let _ = fs::remove_file(server_mods_dir.join(rem));
    }

    for add in &div.added_to_profile {
        let src = profile_mods_dir.join(add);
        let dst = server_mods_dir.join(add);
        if src.exists() {
            let _ = fs::copy(&src, &dst);
        }
    }

    let mut current_mods = Vec::new();
    if let Ok(entries) = fs::read_dir(&server_mods_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".jar") {
                current_mods.push(name);
            }
        }
    }
    current_mods.sort();

    servers[s_idx].installed_mods = current_mods.clone();
    save_servers_state(&servers)?;

    let manifest_path = PathBuf::from(&servers[s_idx].server_dir).join("server_manifest.json");
    let manifest = ServerManifest {
        server_id: servers[s_idx].id.clone(),
        profile_id: servers[s_idx].profile_id.clone(),
        mc_version: servers[s_idx].mc_version.clone(),
        loader: servers[s_idx].loader.clone(),
        loader_version: servers[s_idx].loader_version.clone(),
        mods: current_mods,
    };
    if let Ok(m_json) = serde_json::to_string_pretty(&manifest) {
        let _ = fs::write(manifest_path, m_json);
    }

    emit_progress(app, "Server mods sync completed!", 100, 100, 100.0, 0);

    Ok(format!(
        "Server mods synchronized: {} added, {} removed.",
        div.added_to_profile.len(),
        div.removed_from_profile.len()
    ))
}

pub fn get_servers_for_profile(profile_id: &str) -> Result<Vec<ServerInstance>, String> {
    let servers = load_servers_state();
    Ok(servers
        .into_iter()
        .filter(|s| s.profile_id == profile_id)
        .collect())
}

pub fn get_all_servers() -> Result<Vec<ServerInstance>, String> {
    Ok(load_servers_state())
}

pub fn delete_server(server_id: &str) -> Result<(), String> {
    let mut servers = load_servers_state();
    if let Some(srv) = servers.iter().find(|s| s.id == server_id).cloned() {
        let _ = fs::remove_dir_all(&srv.server_dir);
    }
    servers.retain(|s| s.id != server_id);
    save_servers_state(&servers)?;
    Ok(())
}

pub async fn ping_server_instance(server_id: &str) -> Result<crate::pinger::ServerStatus, String> {
    let servers = load_servers_state();
    let server = servers
        .iter()
        .find(|s| s.id == server_id)
        .ok_or_else(|| format!("Server '{}' not found", server_id))?;

    let parts: Vec<&str> = server.host.split(':').collect();
    let host = parts.first().unwrap_or(&"127.0.0.1");
    let port = parts.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(25565);

    Ok(crate::pinger::ping_server_address(host, port, &server.name).await)
}

// ============================================================================
// Live Game Console / Log Reader
// ============================================================================

pub fn get_instance_logs(profile_id: &str, max_lines: usize) -> Result<InstanceLogsPayload, String> {
    let state = load_profiles_state();
    let profile = state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let dir = PathBuf::from(&profile.instance_dir);
    let launcher_log = dir.join("launcher_log.txt");
    let latest_log = dir.join("logs").join("latest.log");

    // Pick whichever log file was modified more recently
    let chosen_log = match (
        launcher_log.metadata().and_then(|m| m.modified()),
        latest_log.metadata().and_then(|m| m.modified()),
    ) {
        (Ok(t1), Ok(t2)) => {
            if t1 >= t2 {
                launcher_log
            } else {
                latest_log
            }
        }
        (Ok(_), Err(_)) => launcher_log,
        (Err(_), Ok(_)) => latest_log,
        _ => launcher_log,
    };

    let mut lines: Vec<String> = Vec::new();
    let log_source = chosen_log
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "launcher_log.txt".to_string());

    if chosen_log.exists() {
        if let Ok(bytes) = fs::read(&chosen_log) {
            let content = String::from_utf8_lossy(&bytes);
            let all_lines: Vec<&str> = content.lines().collect();
            let start = all_lines.len().saturating_sub(max_lines.max(50));
            lines = all_lines[start..].iter().map(|s| s.to_string()).collect();
        }
    }

    // Check for recent crash reports
    let mut crash_report: Option<String> = None;
    let crash_dir = dir.join("crash-reports");
    if crash_dir.exists() {
        if let Ok(entries) = fs::read_dir(&crash_dir) {
            let mut newest: Option<(SystemTime, PathBuf)> = None;
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Ok(modified) = p.metadata().and_then(|m| m.modified()) {
                        if newest.as_ref().map_or(true, |(t, _)| modified > *t) {
                            newest = Some((modified, p));
                        }
                    }
                }
            }
            if let Some((_, crash_path)) = newest {
                if let Ok(text) = fs::read_to_string(&crash_path) {
                    let snippet: Vec<&str> = text.lines().take(35).collect();
                    crash_report = Some(snippet.join("\n"));
                }
            }
        }
    }

    Ok(InstanceLogsPayload {
        log_source,
        lines,
        crash_report,
    })
}

pub fn clear_instance_logs(profile_id: &str) -> Result<(), String> {
    let state = load_profiles_state();
    let profile = state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let dir = PathBuf::from(&profile.instance_dir);
    let launcher_log = dir.join("launcher_log.txt");
    if launcher_log.exists() {
        let _ = fs::write(&launcher_log, "");
    }
    Ok(())
}

// ============================================================================
// Download & Cryptographic Hash Helpers
// ============================================================================

fn verify_file_sha1(path: &Path, expected_sha1: &str) -> bool {
    if !path.exists() {
        return false;
    }
    if expected_sha1.trim().is_empty() {
        return path.metadata().map(|m| m.len() > 0).unwrap_or(false);
    }
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    let actual = format!("{:x}", hasher.finalize());
    actual.eq_ignore_ascii_case(expected_sha1.trim())
}

async fn download_file_verified(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
    expected_sha512: Option<&str>,
) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut last_err = String::new();
    for attempt in 1..=3 {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                Ok(bytes) => {
                    if let Some(sha1_hex) = expected_sha1 {
                        if !sha1_hex.trim().is_empty() {
                            let mut hasher = Sha1::new();
                            hasher.update(&bytes);
                            let actual = format!("{:x}", hasher.finalize());
                            if !actual.eq_ignore_ascii_case(sha1_hex.trim()) {
                                last_err = format!(
                                    "SHA-1 mismatch for {:?}: expected {}, got {}",
                                    dest.file_name().unwrap_or_default(),
                                    sha1_hex,
                                    actual
                                );
                                continue;
                            }
                        }
                    }
                    if let Some(sha512_hex) = expected_sha512 {
                        if !sha512_hex.trim().is_empty() {
                            let mut hasher = Sha512::new();
                            hasher.update(&bytes);
                            let actual = format!("{:x}", hasher.finalize());
                            if !actual.eq_ignore_ascii_case(sha512_hex.trim()) {
                                last_err = format!(
                                    "SHA-512 mismatch for {:?}: expected {}, got {}",
                                    dest.file_name().unwrap_or_default(),
                                    sha512_hex,
                                    actual
                                );
                                continue;
                            }
                        }
                    }

                    let mut file = fs::File::create(dest).map_err(|e| e.to_string())?;
                    file.write_all(&bytes).map_err(|e| e.to_string())?;
                    return Ok(());
                }
                Err(e) => last_err = e.to_string(),
            },
            Ok(resp) => last_err = format!("HTTP {} from {}", resp.status(), url),
            Err(e) => last_err = e.to_string(),
        }
        if attempt < 3 {
            tokio::time::sleep(std::time::Duration::from_millis(400 * attempt)).await;
        }
    }

    Err(format!("Failed to download {}: {}", url, last_err))
}
