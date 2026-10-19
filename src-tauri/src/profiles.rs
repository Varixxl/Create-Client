use crate::config;
use crate::downloader::ProgressPayload;
use crate::launcher::{is_library_for_current_platform, maven_to_path, VersionLibrary};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha512};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
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
    pub modrinth_slugs: Vec<String>,
    pub created_at: u64,
    pub last_played: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesState {
    pub selected_profile_id: String,
    pub profiles: Vec<ModpackProfile>,
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
            })
    } else {
        ProfilesState {
            selected_profile_id: CREATE_SERVER_PROFILE_ID.to_string(),
            profiles: Vec::new(),
        }
    };

    // Ensure built-in Create Server profile exists and reflects current cfg.game_dir
    if let Some(create_prof) = state
        .profiles
        .iter_mut()
        .find(|p| p.id == CREATE_SERVER_PROFILE_ID)
    {
        create_prof.instance_dir = cfg.game_dir.clone();
        create_prof.is_builtin = true;
    } else {
        state.profiles.insert(0, build_builtin_create_profile(&cfg));
    }

    // Ensure built-in Fabric Performance & QoL profile exists
    if !state
        .profiles
        .iter()
        .any(|p| p.id == FABRIC_DEFAULT_PROFILE_ID)
    {
        let insert_idx = if state.profiles.is_empty() { 0 } else { 1 };
        state
            .profiles
            .insert(insert_idx, build_builtin_fabric_profile());
    }

    if !state
        .profiles
        .iter()
        .any(|p| p.id == state.selected_profile_id)
    {
        state.selected_profile_id = CREATE_SERVER_PROFILE_ID.to_string();
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
    mc_version: String,
    loader: String,
    include_default_mods: bool,
    ram_gb: Option<u32>,
    jvm_flags: Option<String>,
) -> Result<ProfilesState, String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Profile name cannot be empty.".to_string());
    }

    let normalized_loader = loader.trim().to_lowercase();
    if normalized_loader != "fabric" && normalized_loader != "vanilla" {
        return Err("Custom profiles support 'fabric' or 'vanilla' loaders.".to_string());
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

    let desc = if description.trim().is_empty() {
        if normalized_loader == "fabric" {
            format!(
                "Custom Fabric instance ({}) with isolated mods, saves, and configs.",
                target_mc
            )
        } else {
            format!("Custom Vanilla Minecraft ({}) instance.", target_mc)
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
        loader_version: None,
        instance_dir: instance_path.to_string_lossy().to_string(),
        is_builtin: false,
        ram_gb,
        jvm_flags: jvm_flags.filter(|s| !s.trim().is_empty()),
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
) -> Result<ProfilesState, String> {
    let mut state = load_profiles_state();
    let prof = state
        .profiles
        .iter_mut()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let trimmed = name.trim();
    if !trimmed.is_empty() && !prof.is_builtin {
        prof.name = trimmed.to_string();
    }
    if let Some(desc) = description {
        if !desc.trim().is_empty() && !prof.is_builtin {
            prof.description = desc.trim().to_string();
        }
    }
    prof.ram_gb = ram_gb;
    prof.jvm_flags = jvm_flags.filter(|s| !s.trim().is_empty());

    save_profiles_state(&state)?;
    Ok(state)
}

pub fn delete_profile(profile_id: &str) -> Result<ProfilesState, String> {
    if profile_id == CREATE_SERVER_PROFILE_ID {
        return Err("Cannot delete the official Create Server profile.".to_string());
    }

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

    if target.is_builtin {
        // For the built-in Fabric profile, reset its instance state cleanly
        if let Some(p) = state.profiles.iter_mut().find(|p| p.id == profile_id) {
            *p = build_builtin_fabric_profile();
        }
    } else {
        state.profiles.retain(|p| p.id != profile_id);
    }

    if !state
        .profiles
        .iter()
        .any(|p| p.id == state.selected_profile_id)
    {
        state.selected_profile_id = CREATE_SERVER_PROFILE_ID.to_string();
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
        .user_agent("CreateClient/1.1.0 (https://github.com/Varixxl/create-client)")
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| format!("Failed to initialize HTTP client: {}", e))
}

/// Lists stable Minecraft release versions from Modrinth's tag API for the Profile creation dropdown.
pub async fn get_available_mc_versions() -> Result<Vec<String>, String> {
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
        .filter(|t| t.version_type == "release")
        .take(25)
        .map(|t| t.version)
        .collect();

    Ok(releases)
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

    // 9. Update profile metadata in profiles.json
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

pub async fn search_modrinth_mods(
    profile_id: &str,
    query: String,
) -> Result<Vec<ModrinthSearchResult>, String> {
    let state = load_profiles_state();
    let profile = state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let client = build_http_client()?;
    let mc_ver = match &profile.resolved_mc_version {
        Some(v) => v.clone(),
        None if profile.mc_version != "latest" => profile.mc_version.clone(),
        None => resolve_latest_compatible_mc_version(&client, &profile.modrinth_slugs).await?,
    };

    let loader_facet = if profile.loader == "forge" {
        "forge"
    } else {
        "fabric"
    };

    let facets = format!(
        "[[\"categories:{}\"],[\"versions:{}\"],[\"project_type:mod\"]]",
        loader_facet, mc_ver
    );
    let url = format!(
        "https://api.modrinth.com/v2/search?query={}&facets={}&limit=20",
        url_encode(query.trim()),
        url_encode(&facets)
    );

    let resp: serde_json::Value = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Modrinth search failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Invalid Modrinth search JSON: {}", e))?;

    let mods_dir = PathBuf::from(&profile.instance_dir).join("mods");
    let index = load_modrinth_index(&mods_dir);

    let mut results = Vec::new();
    if let Some(hits) = resp["hits"].as_array() {
        for hit in hits {
            let slug = hit["slug"].as_str().unwrap_or("").to_string();
            let project_id = hit["project_id"].as_str().unwrap_or("").to_string();
            if slug.is_empty() {
                continue;
            }
            let installed = index.contains_key(&slug)
                || profile.modrinth_slugs.iter().any(|s| s == &slug);
            results.push(ModrinthSearchResult {
                project_id,
                slug,
                title: hit["title"].as_str().unwrap_or("Unknown Mod").to_string(),
                description: hit["description"].as_str().unwrap_or("").to_string(),
                author: hit["author"].as_str().unwrap_or("Unknown").to_string(),
                downloads: hit["downloads"].as_u64().unwrap_or(0),
                icon_url: hit["icon_url"].as_str().map(|s| s.to_string()),
                installed,
            });
        }
    }
    Ok(results)
}

pub async fn install_modrinth_mod(
    app: &AppHandle,
    profile_id: &str,
    slug: String,
) -> Result<(), String> {
    let mut state = load_profiles_state();
    let prof_idx = state
        .profiles
        .iter()
        .position(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile '{}' not found.", profile_id))?;

    let profile = state.profiles[prof_idx].clone();
    let client = build_http_client()?;
    let mc_ver = match &profile.resolved_mc_version {
        Some(v) => v.clone(),
        None if profile.mc_version != "latest" => profile.mc_version.clone(),
        None => resolve_latest_compatible_mc_version(&client, &profile.modrinth_slugs).await?,
    };

    let mods_dir = PathBuf::from(&profile.instance_dir).join("mods");
    fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;

    let loader_name = if profile.loader == "forge" {
        "forge"
    } else {
        "fabric"
    };

    sync_modrinth_mods_for_instance(
        app,
        &client,
        &mods_dir,
        &mc_ver,
        loader_name,
        &[slug.clone()],
        false,
    )
    .await?;

    if !state.profiles[prof_idx].modrinth_slugs.contains(&slug) {
        state.profiles[prof_idx].modrinth_slugs.push(slug);
    }
    if state.profiles[prof_idx].resolved_mc_version.is_none() {
        state.profiles[prof_idx].resolved_mc_version = Some(mc_ver);
    }
    save_profiles_state(&state)?;
    Ok(())
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
