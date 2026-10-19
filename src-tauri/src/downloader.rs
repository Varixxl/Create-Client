use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use crate::launcher::{is_library_for_current_platform, maven_to_path, VersionMeta};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressPayload {
    pub task: String,
    pub current_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f64,
    pub speed_bytes_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasePackage {
    pub version: String,
    pub url: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaPatch {
    pub from_version: String,
    pub to_version: String,
    pub url: String,
    pub sha256: Option<String>,
    #[serde(default)]
    pub delete_files: Vec<String>,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackManifest {
    #[serde(alias = "version")]
    pub latest_version: String,
    #[serde(default)]
    pub base_package: Option<BasePackage>,
    #[serde(default)]
    pub patches: Vec<DeltaPatch>,
    pub modpack_url: Option<String>,
    pub sha256: Option<String>,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersion {
    pub version: String,
    pub installed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackUpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub changelog: Option<String>,
}

pub fn get_runtime_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "create", "CreateClient") {
        proj_dirs.data_dir().join("runtime")
    } else {
        PathBuf::from("./create_client_runtime")
    }
}

pub fn get_java_executable(runtime_dir: &Path) -> PathBuf {
    get_java_executable_for_version(runtime_dir, 21)
}

pub fn get_java_executable_for_version(runtime_dir: &Path, major: u32) -> PathBuf {
    let folder_name = format!("java{}", major);
    #[cfg(target_os = "windows")]
    {
        let javaw = runtime_dir.join(&folder_name).join("bin").join("javaw.exe");
        if javaw.exists() {
            javaw
        } else {
            runtime_dir.join(&folder_name).join("bin").join("java.exe")
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        runtime_dir.join(&folder_name).join("bin").join("java")
    }
}

pub async fn ensure_java(app: &AppHandle, runtime_dir: &Path) -> Result<PathBuf, String> {
    ensure_java_version(app, runtime_dir, 21).await
}

pub async fn ensure_java_for_instance(
    app: &AppHandle,
    instance_dir: &Path,
    mc_version: &str,
) -> Result<PathBuf, String> {
    let required_version = determine_required_java_version(instance_dir, mc_version);
    let runtime_dir = get_runtime_dir();
    ensure_java_version(app, &runtime_dir, required_version).await
}

pub fn determine_required_java_version(instance_dir: &Path, mc_version: &str) -> u32 {
    let mut required = 17;

    // 1. Inspect version.json in instance_dir (e.g. Forge 1.20.1 specifies majorVersion 17)
    let version_file = instance_dir.join("version.json");
    if version_file.exists() {
        if let Ok(content) = fs::read_to_string(&version_file) {
            if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(major) = meta["javaVersion"]["majorVersion"].as_u64() {
                    required = required.max(major as u32);
                }
            }
        }
    }

    let meta_file = instance_dir.join("fabric_launch_meta.json");
    let mut resolved_mc = mc_version.trim().to_string();
    if meta_file.exists() && (resolved_mc == "latest" || resolved_mc.is_empty()) {
        if let Ok(content) = fs::read_to_string(&meta_file) {
            if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(mc) = meta["mc_version"].as_str() {
                    resolved_mc = mc.trim().to_string();
                }
            }
        }
    }

    // 2. Base version from Minecraft version string
    let trimmed_mc = resolved_mc.trim();
    if trimmed_mc.starts_with("26.") || trimmed_mc.starts_with("27.") {
        required = required.max(25);
    } else if trimmed_mc.starts_with("1.21") || trimmed_mc.starts_with("1.20.5") || trimmed_mc.starts_with("1.20.6") {
        required = required.max(21);
    } else if trimmed_mc.starts_with("1.18") || trimmed_mc.starts_with("1.19") || trimmed_mc.starts_with("1.20") {
        required = required.max(17);
    } else if trimmed_mc.starts_with("1.17") {
        required = required.max(17);
    } else if !trimmed_mc.is_empty() && (trimmed_mc.starts_with("1.16") || trimmed_mc.starts_with("1.12") || trimmed_mc.starts_with("1.7")) {
        required = 8;
    } else if trimmed_mc == "latest" || trimmed_mc.is_empty() {
        required = required.max(21);
    }

    // 3. Scan mods directory for higher Java requirements (e.g. Fabric API 26.3 requiring Java >= 25)
    let mods_dir = instance_dir.join("mods");
    if mods_dir.exists() {
        if let Ok(entries) = fs::read_dir(&mods_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("jar") {
                    if let Some(mod_java) = inspect_jar_for_java_requirement(&p) {
                        required = required.max(mod_java);
                    }
                }
            }
        }
    }

    required
}

fn inspect_jar_for_java_requirement(jar_path: &Path) -> Option<u32> {
    let file = fs::File::open(jar_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    // Check fabric.mod.json
    if let Ok(mut mod_json_file) = archive.by_name("fabric.mod.json") {
        let mut content = String::new();
        if std::io::Read::read_to_string(&mut mod_json_file, &mut content).is_ok() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(dep_java) = val["depends"]["java"].as_str() {
                    if let Some(v) = parse_java_version_spec(dep_java) {
                        return Some(v);
                    }
                } else if let Some(arr) = val["depends"]["java"].as_array() {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            if let Some(v) = parse_java_version_spec(s) {
                                return Some(v);
                            }
                        }
                    }
                }
            }
        }
    }

    // Check META-INF/mods.toml or neoforge.mods.toml
    for toml_path in ["META-INF/mods.toml", "META-INF/neoforge.mods.toml"] {
        if let Ok(mut toml_file) = archive.by_name(toml_path) {
            let mut content = String::new();
            if std::io::Read::read_to_string(&mut toml_file, &mut content).is_ok() {
                if let Some(v) = parse_forge_mods_toml_java(&content) {
                    return Some(v);
                }
            }
        }
    }

    None
}

fn parse_java_version_spec(spec: &str) -> Option<u32> {
    spec.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<u32>().ok())
        .find(|&num| (8..=35).contains(&num))
}

fn parse_forge_mods_toml_java(content: &str) -> Option<u32> {
    let mut in_java_block = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[[dependencies") {
            in_java_block = false;
        }
        if trimmed.contains("modId") && (trimmed.contains("\"java\"") || trimmed.contains("'java'")) {
            in_java_block = true;
        }
        if in_java_block && trimmed.contains("versionRange") {
            if let Some(v) = parse_java_version_spec(trimmed) {
                return Some(v);
            }
        }
    }
    None
}

pub async fn ensure_java_version(
    app: &AppHandle,
    runtime_dir: &Path,
    major: u32,
) -> Result<PathBuf, String> {
    let java_bin = get_java_executable_for_version(runtime_dir, major);
    if java_bin.exists() {
        return Ok(java_bin);
    }

    fs::create_dir_all(runtime_dir).map_err(|e| format!("Failed to create runtime directory: {}", e))?;

    let os_str = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };

    let arch_str = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x64"
    };

    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: format!("Resolving OpenJDK {} Runtime...", major),
            current_bytes: 0,
            total_bytes: 100,
            percentage: 0.0,
            speed_bytes_sec: 0,
        },
    );

    // Try resolving download link via Adoptium API first
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let adoptium_url = format!(
        "https://api.adoptium.net/v3/assets/latest/{}/hotspot?image_type=jre&os={}&architecture={}",
        major, os_str, arch_str
    );

    let mut download_url = None;
    if let Ok(resp) = client.get(&adoptium_url).header("User-Agent", "Create-Client").send().await {
        if let Ok(data) = resp.json::<serde_json::Value>().await {
            if let Some(arr) = data.as_array() {
                if let Some(first) = arr.first() {
                    if let Some(link) = first["binary"]["package"]["link"].as_str() {
                        download_url = Some(link.to_string());
                    }
                }
            }
        }
    }

    // High-availability fallback URLs
    let final_url = download_url.unwrap_or_else(|| {
        match major {
            25 => {
                if cfg!(target_os = "windows") {
                    "https://github.com/adoptium/temurin25-binaries/releases/download/jdk-25.0.4.1%2B1/OpenJDK25U-jre_x64_windows_hotspot_25.0.4.1_1.zip".to_string()
                } else if cfg!(target_os = "macos") {
                    "https://github.com/adoptium/temurin25-binaries/releases/download/jdk-25.0.4.1%2B1/OpenJDK25U-jre_x64_mac_hotspot_25.0.4.1_1.tar.gz".to_string()
                } else {
                    "https://github.com/adoptium/temurin25-binaries/releases/download/jdk-25.0.4.1%2B1/OpenJDK25U-jre_x64_linux_hotspot_25.0.4.1_1.tar.gz".to_string()
                }
            }
            17 => {
                if cfg!(target_os = "windows") {
                    "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.20.1%2B1/OpenJDK17U-jre_x64_windows_hotspot_17.0.20.1_1.zip".to_string()
                } else if cfg!(target_os = "macos") {
                    "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.20.1%2B1/OpenJDK17U-jre_x64_mac_hotspot_17.0.20.1_1.tar.gz".to_string()
                } else {
                    "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.20.1%2B1/OpenJDK17U-jre_x64_linux_hotspot_17.0.20.1_1.tar.gz".to_string()
                }
            }
            8 => {
                if cfg!(target_os = "windows") {
                    "https://github.com/adoptium/temurin8-binaries/releases/download/jdk8u504-b01/OpenJDK8U-jre_x64_windows_hotspot_8u504b01.zip".to_string()
                } else {
                    "https://github.com/adoptium/temurin8-binaries/releases/download/jdk8u504-b01/OpenJDK8U-jre_x64_linux_hotspot_8u504b01.tar.gz".to_string()
                }
            }
            _ => {
                // Default Java 21 Azul Zulu fallback
                if cfg!(target_os = "windows") {
                    "https://cdn.azul.com/zulu/bin/zulu21.52.203-ca-jre21.0.12.1-win_x64.zip".to_string()
                } else if cfg!(target_os = "macos") {
                    "https://cdn.azul.com/zulu/bin/zulu21.52.203-ca-jre21.0.12.1-macosx_x64.tar.gz".to_string()
                } else {
                    "https://cdn.azul.com/zulu/bin/zulu21.52.203-ca-jre21.0.12.1-linux_x64.tar.gz".to_string()
                }
            }
        }
    });

    let is_zip = final_url.ends_with(".zip");
    let archive_ext = if is_zip { "zip" } else { "tar.gz" };
    let temp_archive = runtime_dir.join(format!("java{}_download.{}", major, archive_ext));

    let download_label = format!("Downloading OpenJDK {} JRE", major);
    download_file_with_progress(app, &final_url, &temp_archive, &download_label).await?;

    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: format!("Extracting OpenJDK {} JRE...", major),
            current_bytes: 100,
            total_bytes: 100,
            percentage: 99.0,
            speed_bytes_sec: 0,
        },
    );

    let temp_extract_dir = runtime_dir.join(format!("java{}_extract_temp", major));
    let _ = fs::remove_dir_all(&temp_extract_dir);
    fs::create_dir_all(&temp_extract_dir).map_err(|e| e.to_string())?;

    if is_zip {
        extract_zip(&temp_archive, &temp_extract_dir)?;
    } else {
        extract_tar_gz(&temp_archive, &temp_extract_dir)?;
    }

    let _ = fs::remove_file(&temp_archive);

    let mut inner_dir = None;
    for entry in fs::read_dir(&temp_extract_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            inner_dir = Some(entry.path());
            break;
        }
    }

    let target_dir = runtime_dir.join(format!("java{}", major));
    let _ = fs::remove_dir_all(&target_dir);

    if let Some(src) = inner_dir {
        fs::rename(src, &target_dir).map_err(|e| format!("Failed to move extracted JRE: {}", e))?;
    } else {
        fs::rename(&temp_extract_dir, &target_dir).map_err(|e| format!("Failed to move extracted JRE: {}", e))?;
    }
    let _ = fs::remove_dir_all(&temp_extract_dir);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let bin_path = get_java_executable_for_version(runtime_dir, major);
        if bin_path.exists() {
            let _ = fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o755));
        }
    }

    let final_bin = get_java_executable_for_version(runtime_dir, major);
    if !final_bin.exists() {
        return Err(format!("Java binary was not found at expected path: {:?}", final_bin));
    }

    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: format!("OpenJDK {} Ready", major),
            current_bytes: 100,
            total_bytes: 100,
            percentage: 100.0,
            speed_bytes_sec: 0,
        },
    );

    Ok(final_bin)
}

pub fn get_installed_version(game_dir: &Path) -> Option<String> {
    let version_file = game_dir.join("client_version.json");
    if version_file.exists() {
        fs::read_to_string(&version_file)
            .ok()
            .and_then(|c| serde_json::from_str::<InstalledVersion>(&c).ok())
            .map(|v| v.version)
    } else {
        None
    }
}

pub async fn fetch_manifest(manifest_url: &str) -> Result<ModpackManifest, String> {
    let client = Client::new();
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let cache_bust_url = if manifest_url.contains('?') {
        format!("{}&_nocache={}", manifest_url, now_ts)
    } else {
        format!("{}?_nocache={}", manifest_url, now_ts)
    };

    let fetch_result = client
        .get(&cache_bust_url)
        .header("User-Agent", "CreateClient-Launcher")
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .send()
        .await;

    let res = match fetch_result {
        Ok(r) if r.status().is_success() => r,
        _ => {
            let fallback_url = format!(
                "https://raw.githubusercontent.com/JaviDaBomb/create-modpack/main/manifest.json?_nocache={}",
                now_ts
            );
            client
                .get(&fallback_url)
                .header("User-Agent", "CreateClient-Launcher")
                .header("Cache-Control", "no-cache, no-store, must-revalidate")
                .header("Pragma", "no-cache")
                .send()
                .await
                .map_err(|e| format!("Failed to check modpack manifest: {}", e))?
        }
    };

    if !res.status().is_success() {
        return Err(format!("Could not fetch modpack manifest (HTTP {})", res.status()));
    }

    res.json::<ModpackManifest>()
        .await
        .map_err(|e| format!("Failed to parse modpack manifest: {}", e))
}

pub async fn check_modpack_update(game_dir: &Path, manifest_url: &str) -> Result<ModpackUpdateInfo, String> {
    let current_ver = get_installed_version(game_dir);
    let manifest = match fetch_manifest(manifest_url).await {
        Ok(m) => m,
        Err(_) => {
            return Ok(ModpackUpdateInfo {
                has_update: false,
                current_version: current_ver.unwrap_or_else(|| "Not Installed".to_string()),
                latest_version: "Unknown".to_string(),
                changelog: None,
            });
        }
    };

    let has_update = current_ver.as_ref() != Some(&manifest.latest_version);

    Ok(ModpackUpdateInfo {
        has_update,
        current_version: current_ver.unwrap_or_else(|| "Not Installed".to_string()),
        latest_version: manifest.latest_version.clone(),
        changelog: manifest.changelog,
    })
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

fn verify_sha256_file(path: &Path, expected_hex: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let expected = expected_hex.trim().to_lowercase();
    if expected.is_empty() {
        return Ok(());
    }

    let mut file = File::open(path).map_err(|e| format!("Failed to open file for SHA-256 check: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("Error reading file for SHA-256 check: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual = bytes_to_hex(&hasher.finalize());
    if actual != expected {
        let _ = fs::remove_file(path);
        return Err(format!(
            "SHA-256 checksum mismatch for {:?}: expected {}, got {}",
            path.file_name().unwrap_or_default(),
            expected,
            actual
        ));
    }
    Ok(())
}

fn verify_sha1_bytes(bytes: &[u8], expected_hex: &str) -> bool {
    use sha1::{Digest, Sha1};
    let expected = expected_hex.trim().to_lowercase();
    if expected.is_empty() {
        return true;
    }
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    bytes_to_hex(&hasher.finalize()) == expected
}

pub fn delete_all_game_data(game_dir: &Path) -> Result<(), String> {
    if !game_dir.exists() {
        return Ok(());
    }

    let canonical = fs::canonicalize(game_dir).unwrap_or_else(|_| game_dir.to_path_buf());
    if canonical.components().count() <= 2 {
        return Err("Refusing to delete a root or top-level system directory.".to_string());
    }

    if let Some(base_dirs) = directories::BaseDirs::new() {
        let home = fs::canonicalize(base_dirs.home_dir()).unwrap_or_else(|_| base_dirs.home_dir().to_path_buf());
        if canonical == home {
            return Err("Refusing to delete user home directory.".to_string());
        }
    }

    let dir_name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    let has_safe_name = dir_name.contains("game")
        || dir_name.contains("create")
        || dir_name.contains("minecraft");
    let has_mc_markers = game_dir.join("client_version.json").exists()
        || game_dir.join("mods").exists()
        || game_dir.join("libraries").exists()
        || game_dir.join("assets").exists()
        || game_dir.join("version.json").exists();

    if !has_safe_name && !has_mc_markers {
        return Err(
            "Refusing to wipe directory: target folder does not appear to be a Create Client game directory."
                .to_string(),
        );
    }

    let entries = fs::read_dir(game_dir).map_err(|e| format!("Failed to read game directory: {}", e))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let _ = fs::remove_dir_all(&p);
        } else {
            let _ = fs::remove_file(&p);
        }
    }
    Ok(())
}

pub async fn reset_modpack_settings(game_dir: &Path) -> Result<(), String> {
    let _ = fs::create_dir_all(game_dir);
    let options_file = game_dir.join("options.txt");
    let client = Client::new();
    let raw_url = "https://raw.githubusercontent.com/JaviDaBomb/create-modpack/main/options.txt";
    if let Ok(res) = client.get(raw_url).send().await {
        if res.status().is_success() {
            if let Ok(bytes) = res.bytes().await {
                let _ = fs::write(&options_file, bytes);
                return Ok(());
            }
        }
    }
    let local = Path::new("/home/varixxl/create-modpack/options.txt");
    if local.exists() {
        let _ = fs::copy(local, &options_file);
        return Ok(());
    }
    Err("Could not fetch default options.txt".to_string())
}

pub async fn check_and_sync_modpack(
    app: &AppHandle,
    game_dir: &Path,
    manifest_url: &str,
    force_repair: bool,
) -> Result<String, String> {
    fs::create_dir_all(game_dir).map_err(|e| format!("Failed to create game directory: {}", e))?;

    let version_file = game_dir.join("client_version.json");
    let mut current_version: Option<String> = if force_repair {
        None
    } else {
        get_installed_version(game_dir)
    };

    let manifest = match fetch_manifest(manifest_url).await {
        Ok(m) => m,
        Err(e) => {
            if let Some(ver) = current_version {
                return Ok(ver);
            }
            return Err(e);
        }
    };

    if Some(&manifest.latest_version) == current_version.as_ref() && !force_repair {
        return Ok(manifest.latest_version);
    }

    // Determine base package info (fallback to modpack_url if base_package not specified)
    let base_pkg = manifest.base_package.clone().unwrap_or_else(|| BasePackage {
        version: manifest.latest_version.clone(),
        url: manifest.modpack_url.clone().unwrap_or_default(),
        sha256: manifest.sha256.clone(),
    });

    // Step 1: Base installation if no installation exists or force_repair requested
    if current_version.is_none() || force_repair {
        if base_pkg.url.is_empty() {
            return Err("No base package URL found in manifest".to_string());
        }

        let temp_zip = game_dir.join("base_modpack.zip");
        download_file_with_progress(
            app,
            &base_pkg.url,
            &temp_zip,
            &format!("Downloading Base Modpack v{}...", base_pkg.version),
        )
        .await?;

        if let Some(expected_sha) = &base_pkg.sha256 {
            verify_sha256_file(&temp_zip, expected_sha)?;
        }

        let _ = app.emit(
            "download-progress",
            ProgressPayload {
                task: "Extracting Base Modpack...".to_string(),
                current_bytes: 100,
                total_bytes: 100,
                percentage: 95.0,
                speed_bytes_sec: 0,
            },
        );

        extract_modpack_safely(&temp_zip, game_dir)?;
        let _ = fs::remove_file(&temp_zip);

        current_version = Some(base_pkg.version.clone());
        save_installed_version(&version_file, &base_pkg.version)?;
    }

    // Step 2: Apply sequential delta patches if current version is behind latest
    let mut installed_ver = current_version.unwrap();
    if installed_ver != manifest.latest_version {
        let maybe_chain = find_patch_chain(&manifest.patches, &installed_ver, &manifest.latest_version);

        match maybe_chain {
            Some(chain) => {
                for patch in chain {
                    let task_label = format!("Downloading Update (v{} -> v{})...", patch.from_version, patch.to_version);
                    let temp_zip = game_dir.join(format!("patch_v{}.zip", patch.to_version));

                    download_file_with_progress(app, &patch.url, &temp_zip, &task_label).await?;

                    if let Some(expected_sha) = &patch.sha256 {
                        verify_sha256_file(&temp_zip, expected_sha)?;
                    }

                    // Delete files specified in patch removal list (only after verifying patch integrity)
                    for del_rel in &patch.delete_files {
                        delete_file_safely(game_dir, del_rel);
                    }

                    let _ = app.emit(
                        "download-progress",
                        ProgressPayload {
                            task: format!("Applying Patch v{}...", patch.to_version),
                            current_bytes: 100,
                            total_bytes: 100,
                            percentage: 95.0,
                            speed_bytes_sec: 0,
                        },
                    );

                    extract_modpack_safely(&temp_zip, game_dir)?;
                    let _ = fs::remove_file(&temp_zip);

                    installed_ver = patch.to_version.clone();
                    save_installed_version(&version_file, &installed_ver)?;
                }
            }
            None => {
                // If direct patch chain cannot be constructed, fall back to base package
                if !base_pkg.url.is_empty() {
                    let temp_zip = game_dir.join("base_modpack.zip");
                    download_file_with_progress(
                        app,
                        &base_pkg.url,
                        &temp_zip,
                        &format!("Re-syncing Base Modpack v{}...", base_pkg.version),
                    )
                    .await?;

                    if let Some(expected_sha) = &base_pkg.sha256 {
                        verify_sha256_file(&temp_zip, expected_sha)?;
                    }

                    extract_modpack_safely(&temp_zip, game_dir)?;
                    let _ = fs::remove_file(&temp_zip);

                    installed_ver = base_pkg.version.clone();
                    save_installed_version(&version_file, &installed_ver)?;

                    // Check if additional patches exist from base to latest
                    if let Some(sub_chain) = find_patch_chain(&manifest.patches, &installed_ver, &manifest.latest_version) {
                        for patch in sub_chain {
                            let task_label = format!("Downloading Update (v{} -> v{})...", patch.from_version, patch.to_version);
                            let temp_zip = game_dir.join(format!("patch_v{}.zip", patch.to_version));

                            download_file_with_progress(app, &patch.url, &temp_zip, &task_label).await?;

                            if let Some(expected_sha) = &patch.sha256 {
                                verify_sha256_file(&temp_zip, expected_sha)?;
                            }

                            for del_rel in &patch.delete_files {
                                delete_file_safely(game_dir, del_rel);
                            }

                            extract_modpack_safely(&temp_zip, game_dir)?;
                            let _ = fs::remove_file(&temp_zip);

                            installed_ver = patch.to_version.clone();
                            save_installed_version(&version_file, &installed_ver)?;
                        }
                    }
                }
            }
        }
    }

    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: "Modpack up to date".to_string(),
            current_bytes: 100,
            total_bytes: 100,
            percentage: 100.0,
            speed_bytes_sec: 0,
        },
    );

    Ok(installed_ver)
}

fn save_installed_version(version_file: &Path, version: &str) -> Result<(), String> {
    let installed = InstalledVersion {
        version: version.to_string(),
        installed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    let data = serde_json::to_string_pretty(&installed).map_err(|e| e.to_string())?;
    fs::write(version_file, data).map_err(|e| e.to_string())?;
    Ok(())
}

fn find_patch_chain<'a>(
    patches: &'a [DeltaPatch],
    start_version: &str,
    target_version: &str,
) -> Option<Vec<&'a DeltaPatch>> {
    let mut chain = Vec::new();
    let mut current = start_version.to_string();
    let mut visited = std::collections::HashSet::new();
    visited.insert(current.clone());

    while current != target_version {
        if let Some(next_patch) = patches.iter().find(|p| p.from_version == current) {
            if !visited.insert(next_patch.to_version.clone()) {
                // Cycle detected, abort chain
                return None;
            }
            current = next_patch.to_version.clone();
            chain.push(next_patch);
        } else {
            return None; // Broken chain
        }
    }

    Some(chain)
}

fn delete_file_safely(game_dir: &Path, rel_path: &str) {
    let slash_norm = rel_path.replace('\\', "/");
    let normalized = slash_norm.trim_start_matches('/');
    if normalized.is_empty()
        || normalized.split('/').any(|seg| seg == ".." || seg == ".")
    {
        return;
    }

    let protected_files = [
        "options.txt",
        "optionsof.txt",
        "servers.dat",
        "client_version.json",
    ];
    let protected_prefixes = [
        "screenshots/",
        "saves/",
        "logs/",
    ];

    if protected_files.iter().any(|&pf| normalized == pf)
        || protected_prefixes.iter().any(|&pp| normalized.starts_with(pp))
    {
        return;
    }

    let target = game_dir.join(normalized);
    if target.is_file() || target.is_symlink() {
        let _ = fs::remove_file(&target);
    } else if target.is_dir() {
        let _ = fs::remove_dir_all(&target);
    }
}

async fn download_file_with_progress(
    app: &AppHandle,
    url: &str,
    target_path: &Path,
    task_label: &str,
) -> Result<(), String> {
    let client = Client::new();
    let res = client
        .get(url)
        .header("User-Agent", "CreateClient-Launcher")
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Download failed with status: {}", res.status()));
    }

    let total_bytes = res.content_length().unwrap_or(0);
    let mut file = File::create(target_path).map_err(|e| format!("Failed to create local file: {}", e))?;

    let mut stream = res.bytes_stream();
    let mut current_bytes = 0u64;
    let mut last_emit = Instant::now();
    let mut bytes_since_last = 0u64;

    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res.map_err(|e| format!("Error downloading chunk: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Error writing to file: {}", e))?;

        let len = chunk.len() as u64;
        current_bytes += len;
        bytes_since_last += len;

        let elapsed = last_emit.elapsed();
        if elapsed.as_millis() >= 150 || current_bytes == total_bytes {
            let speed = if elapsed.as_secs_f64() > 0.0 {
                (bytes_since_last as f64 / elapsed.as_secs_f64()) as u64
            } else {
                0
            };

            let percentage = if total_bytes > 0 {
                (current_bytes as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };

            let _ = app.emit(
                "download-progress",
                ProgressPayload {
                    task: task_label.to_string(),
                    current_bytes,
                    total_bytes,
                    percentage,
                    speed_bytes_sec: speed,
                },
            );

            last_emit = Instant::now();
            bytes_since_last = 0;
        }
    }

    Ok(())
}

fn extract_zip(archive_path: &Path, extract_to: &Path) -> Result<(), String> {
    let file = File::open(archive_path).map_err(|e| format!("Cannot open zip file: {}", e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Cannot read zip archive: {}", e))?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("Invalid zip entry {}: {}", i, e))?;
        let out_path = extract_to.join(entry.mangled_name());

        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = out_path.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut outfile = File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, extract_to: &Path) -> Result<(), String> {
    let tar_gz = File::open(archive_path).map_err(|e| format!("Cannot open tar.gz: {}", e))?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);
    archive.unpack(extract_to).map_err(|e| format!("Failed to unpack tar.gz: {}", e))?;
    Ok(())
}

fn extract_modpack_safely(zip_path: &Path, game_dir: &Path) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| format!("Cannot open modpack zip: {}", e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Cannot read modpack zip: {}", e))?;

    // Protected relative paths that will NEVER be overwritten if they exist
    let protected_files = [
        "options.txt",
        "optionsof.txt",
        "servers.dat",
        "client_version.json",
    ];
    let protected_prefixes = [
        "screenshots/",
        "saves/",
        "logs/",
    ];

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("Invalid zip entry {}: {}", i, e))?;
        let raw_name = entry.name().replace('\\', "/");
        let norm_name = raw_name.trim_start_matches('/');
        let mangled = entry.mangled_name();

        // Check if file is protected
        let is_protected = protected_files.iter().any(|&pf| norm_name == pf)
            || protected_prefixes.iter().any(|&pp| norm_name.starts_with(pp));

        let out_path = game_dir.join(mangled);

        if is_protected && out_path.exists() {
            continue; // Keep player's existing settings/saves
        }

        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = out_path.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut outfile = File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct AssetIndex {
    objects: std::collections::HashMap<String, AssetEntry>,
}

#[derive(Debug, Deserialize)]
struct AssetEntry {
    hash: String,
    size: u64,
}

pub async fn ensure_assets(app: &AppHandle, game_dir: &Path) -> Result<PathBuf, String> {
    let assets_dir = game_dir.join("assets");
    let indexes_dir = assets_dir.join("indexes");
    let objects_dir = assets_dir.join("objects");
    let index_file = indexes_dir.join("5.json");

    let _ = fs::create_dir_all(&indexes_dir);
    let _ = fs::create_dir_all(&objects_dir);

    // If index file doesn't exist, download it from Mojang and verify SHA-1
    if !index_file.exists() {
        let client = Client::new();
        let expected_idx_sha1 = "3d1e8e9cad62f188270564f4bdfd3a79df9be65a";
        if let Ok(res) = client
            .get("https://piston-meta.mojang.com/v1/packages/3d1e8e9cad62f188270564f4bdfd3a79df9be65a/5.json")
            .send()
            .await
        {
            if res.status().is_success() {
                if let Ok(bytes) = res.bytes().await {
                    if verify_sha1_bytes(&bytes, expected_idx_sha1) {
                        let _ = fs::write(&index_file, bytes);
                    }
                }
            }
        }
    }

    if !index_file.exists() {
        return Ok(assets_dir);
    }

    let content = match fs::read_to_string(&index_file) {
        Ok(c) => c,
        Err(_) => return Ok(assets_dir),
    };

    let index: AssetIndex = match serde_json::from_str(&content) {
        Ok(idx) => idx,
        Err(_) => return Ok(assets_dir),
    };

    // Fast-import from existing launcher installations if available (Modrinth, Minecraft, Prism)
    let mut candidate_dirs: Vec<PathBuf> = Vec::new();
    if let Some(base_dirs) = directories::BaseDirs::new() {
        let data = base_dirs.data_dir();
        let home = base_dirs.home_dir();
        candidate_dirs.push(data.join("com.modrinth.theseus").join("meta").join("assets").join("objects"));
        candidate_dirs.push(data.join("ModrinthApp").join("meta").join("assets").join("objects"));
        candidate_dirs.push(data.join(".minecraft").join("assets").join("objects"));
        candidate_dirs.push(home.join(".minecraft").join("assets").join("objects"));
        candidate_dirs.push(data.join("PrismLauncher").join("assets").join("objects"));
        candidate_dirs.push(home.join(".local").join("share").join("ModrinthApp").join("meta").join("assets").join("objects"));
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = PathBuf::from(appdata);
            candidate_dirs.push(p.join("com.modrinth.theseus").join("meta").join("assets").join("objects"));
            candidate_dirs.push(p.join(".minecraft").join("assets").join("objects"));
            candidate_dirs.push(p.join("PrismLauncher").join("assets").join("objects"));
        }
    }

    for candidate in &candidate_dirs {
        if candidate.is_dir() {
            for entry in index.objects.values() {
                let hash = &entry.hash;
                if hash.len() < 2 {
                    continue;
                }
                let prefix = &hash[0..2];
                let dest = objects_dir.join(prefix).join(hash);
                if !dest.exists() {
                    let src = candidate.join(prefix).join(hash);
                    if src.is_file() {
                        if let Some(p) = dest.parent() {
                            let _ = fs::create_dir_all(p);
                        }
                        if fs::hard_link(&src, &dest).is_err() {
                            let _ = fs::copy(&src, &dest);
                        }
                    }
                }
            }
        }
    }

    // Identify any assets that are still missing from disk
    let mut missing_objects: Vec<(String, PathBuf, u64, String)> = Vec::new();
    let mut total_missing_bytes: u64 = 0;

    for entry in index.objects.values() {
        let hash = &entry.hash;
        if hash.len() < 2 {
            continue;
        }
        let prefix = &hash[0..2];
        let dest = objects_dir.join(prefix).join(hash);
        if !dest.exists() {
            let url = format!("https://resources.download.minecraft.net/{}/{}", prefix, hash);
            missing_objects.push((url, dest, entry.size, hash.clone()));
            total_missing_bytes += entry.size;
        }
    }

    if missing_objects.is_empty() {
        return Ok(assets_dir);
    }

    // Concurrently download missing objects from Mojang CDN
    let client = Client::builder()
        .pool_max_idle_per_host(32)
        .build()
        .unwrap_or_else(|_| Client::new());

    let downloaded_bytes = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let total_count = missing_objects.len();
    let completed_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let failed_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let start_time = Instant::now();
    let last_emit = std::sync::Arc::new(tokio::sync::Mutex::new(Instant::now()));

    let stream = futures_util::stream::iter(missing_objects).map(|(url, dest, size, expected_hash)| {
        let client = client.clone();
        let app = app.clone();
        let downloaded = downloaded_bytes.clone();
        let completed = completed_count.clone();
        let failed = failed_count.clone();
        let last_emit = last_emit.clone();

        async move {
            if let Some(parent) = dest.parent() {
                let _ = fs::create_dir_all(parent);
            }

            let mut ok = false;
            if let Ok(res) = client.get(&url).send().await {
                if res.status().is_success() {
                    if let Ok(bytes) = res.bytes().await {
                        if verify_sha1_bytes(&bytes, &expected_hash) {
                            if fs::write(&dest, bytes).is_ok() {
                                ok = true;
                            }
                        }
                    }
                }
            }
            if !ok {
                failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }

            let cur_bytes = downloaded.fetch_add(size, std::sync::atomic::Ordering::Relaxed) + size;
            let count = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;

            let mut emit_guard = last_emit.lock().await;
            if emit_guard.elapsed().as_millis() >= 150 || count == total_count {
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    (cur_bytes as f64 / elapsed) as u64
                } else {
                    0
                };
                let percentage = if total_missing_bytes > 0 {
                    (cur_bytes as f64 / total_missing_bytes as f64) * 100.0
                } else {
                    100.0
                };

                let _ = app.emit(
                    "download-progress",
                    ProgressPayload {
                        task: format!("Downloading Minecraft assets ({}/{})", count, total_count),
                        current_bytes: cur_bytes,
                        total_bytes: total_missing_bytes,
                        percentage,
                        speed_bytes_sec: speed,
                    },
                );
                *emit_guard = Instant::now();
            }
        }
    });

    stream.buffer_unordered(24).collect::<Vec<()>>().await;

    let failures = failed_count.load(std::sync::atomic::Ordering::Relaxed);
    if failures > 0 {
        return Err(format!(
            "Failed to download or verify {} Minecraft asset file(s). Please check your connection and try again.",
            failures
        ));
    }

    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: "Assets synchronized".to_string(),
            current_bytes: total_missing_bytes,
            total_bytes: total_missing_bytes,
            percentage: 100.0,
            speed_bytes_sec: 0,
        },
    );

    Ok(assets_dir)
}

pub async fn ensure_libraries(app: &AppHandle, game_dir: &Path) -> Result<(), String> {
    let version_file = game_dir.join("version.json");
    if !version_file.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&version_file).map_err(|e| e.to_string())?;
    let version_meta: VersionMeta = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    let lib_dir = game_dir.join("libraries");
    let natives_dir = game_dir.join("natives");
    let _ = fs::create_dir_all(&lib_dir);
    let _ = fs::create_dir_all(&natives_dir);

    let client = Client::new();

    // Check which libraries are missing for the current platform
    let mut missing_libs = Vec::new();
    for lib in &version_meta.libraries {
        if !is_library_for_current_platform(lib) {
            continue;
        }

        let rel_path = lib
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .and_then(|a| a.path.as_ref())
            .cloned()
            .or_else(|| lib.name.as_deref().and_then(maven_to_path));

        let download_url = lib
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .and_then(|a| a.url.as_ref())
            .cloned();

        let expected_sha1 = lib
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .and_then(|a| a.sha1.as_ref())
            .cloned();

        if let (Some(rel), Some(url)) = (rel_path, download_url) {
            let full_dest = lib_dir.join(&rel);
            if !full_dest.exists() {
                missing_libs.push((full_dest, url, rel.contains("natives"), expected_sha1));
            } else if rel.contains("natives") {
                extract_natives_from_jar(&full_dest, &natives_dir);
            }
        }
    }

    if !missing_libs.is_empty() {
        let total = missing_libs.len();
        for (idx, (full_dest, url, is_native, expected_sha1)) in missing_libs.into_iter().enumerate() {
            let _ = app.emit(
                "download-progress",
                ProgressPayload {
                    task: format!("Downloading libraries ({}/{})...", idx + 1, total),
                    current_bytes: idx as u64,
                    total_bytes: total as u64,
                    percentage: ((idx as f64) / (total as f64)) * 100.0,
                    speed_bytes_sec: 0,
                },
            );

            if let Some(parent) = full_dest.parent() {
                let _ = fs::create_dir_all(parent);
            }

            let res = client
                .get(&url)
                .header("User-Agent", "CreateClient-Launcher")
                .send()
                .await
                .map_err(|e| format!("Failed to download library {}: {}", url, e))?;

            if !res.status().is_success() {
                return Err(format!("Library download failed (HTTP {}): {}", res.status(), url));
            }

            let bytes = res
                .bytes()
                .await
                .map_err(|e| format!("Failed to read library bytes from {}: {}", url, e))?;

            if let Some(sha1) = &expected_sha1 {
                if !verify_sha1_bytes(&bytes, sha1) {
                    return Err(format!(
                        "SHA-1 checksum mismatch for downloaded library {:?}",
                        full_dest.file_name().unwrap_or_default()
                    ));
                }
            }

            fs::write(&full_dest, bytes)
                .map_err(|e| format!("Failed to write library {:?}: {}", full_dest, e))?;
            if is_native {
                extract_natives_from_jar(&full_dest, &natives_dir);
            }
        }
    }

    // Ensure vanilla client jar
    let client_jar = game_dir.join("1.20.1.jar");
    if !client_jar.exists() {
        let alt = game_dir.join("client.jar");
        if !alt.exists() {
            let _ = app.emit(
                "download-progress",
                ProgressPayload {
                    task: "Downloading Minecraft 1.20.1 client jar...".to_string(),
                    current_bytes: 0,
                    total_bytes: 100,
                    percentage: 50.0,
                    speed_bytes_sec: 0,
                },
            );
            let expected_client_sha1 = "43db9b498cb67058d2e12d394e6507722e71bb45";
            let client_url = "https://piston-data.mojang.com/v1/objects/43db9b498cb67058d2e12d394e6507722e71bb45/client.jar";
            let res = client
                .get(client_url)
                .header("User-Agent", "CreateClient-Launcher")
                .send()
                .await
                .map_err(|e| format!("Failed to download Minecraft 1.20.1 client jar: {}", e))?;
            if !res.status().is_success() {
                return Err(format!(
                    "Failed to download Minecraft 1.20.1 client jar (HTTP {})",
                    res.status()
                ));
            }
            let bytes = res
                .bytes()
                .await
                .map_err(|e| format!("Failed to read Minecraft 1.20.1 client jar stream: {}", e))?;
            if !verify_sha1_bytes(&bytes, expected_client_sha1) {
                return Err("SHA-1 checksum mismatch for Minecraft 1.20.1 client.jar".to_string());
            }
            fs::write(&client_jar, bytes)
                .map_err(|e| format!("Failed to write 1.20.1.jar: {}", e))?;
        }
    }

    Ok(())
}

fn extract_natives_from_jar(jar_path: &Path, natives_dir: &Path) {
    if let Ok(file) = File::open(jar_path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            for i in 0..archive.len() {
                if let Ok(mut entry) = archive.by_index(i) {
                    let name = entry.name().to_string();
                    if name.ends_with(".dll") || name.ends_with(".so") || name.ends_with(".dylib") {
                        if let Some(file_name) = Path::new(&name).file_name() {
                            let dest = natives_dir.join(file_name);
                            if let Ok(mut out) = File::create(&dest) {
                                let _ = std::io::copy(&mut entry, &mut out);
                            }
                        }
                    }
                }
            }
        }
    }
}
