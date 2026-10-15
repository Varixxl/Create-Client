use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use crate::launcher::{is_rule_allowed, maven_to_path, VersionMeta};

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
    #[cfg(target_os = "windows")]
    {
        let javaw = runtime_dir.join("java21").join("bin").join("javaw.exe");
        if javaw.exists() {
            javaw
        } else {
            runtime_dir.join("java21").join("bin").join("java.exe")
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        runtime_dir.join("java21").join("bin").join("java")
    }
}

pub async fn ensure_java(app: &AppHandle, runtime_dir: &Path) -> Result<PathBuf, String> {
    let java_bin = get_java_executable(runtime_dir);
    if java_bin.exists() {
        return Ok(java_bin);
    }

    fs::create_dir_all(runtime_dir).map_err(|e| format!("Failed to create runtime directory: {}", e))?;

    let download_url = if cfg!(target_os = "windows") {
        "https://cdn.azul.com/zulu/bin/zulu21.52.203-ca-jre21.0.12.1-win_x64.zip"
    } else if cfg!(target_os = "macos") {
        "https://cdn.azul.com/zulu/bin/zulu21.52.203-ca-jre21.0.12.1-macosx_x64.tar.gz"
    } else {
        "https://cdn.azul.com/zulu/bin/zulu21.52.203-ca-jre21.0.12.1-linux_x64.tar.gz"
    };

    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: "Resolving Azul Zulu 21 Runtime...".to_string(),
            current_bytes: 0,
            total_bytes: 100,
            percentage: 0.0,
            speed_bytes_sec: 0,
        },
    );

    let is_zip = download_url.ends_with(".zip");
    let archive_ext = if is_zip { "zip" } else { "tar.gz" };
    let temp_archive = runtime_dir.join(format!("java21_download.{}", archive_ext));

    download_file_with_progress(app, download_url, &temp_archive, "Downloading Azul Zulu 21 JRE").await?;

    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: "Extracting Azul Zulu 21 JRE...".to_string(),
            current_bytes: 100,
            total_bytes: 100,
            percentage: 99.0,
            speed_bytes_sec: 0,
        },
    );

    let temp_extract_dir = runtime_dir.join("java21_extract_temp");
    let _ = fs::remove_dir_all(&temp_extract_dir);
    fs::create_dir_all(&temp_extract_dir).map_err(|e| e.to_string())?;

    if is_zip {
        extract_zip(&temp_archive, &temp_extract_dir)?;
    } else {
        extract_tar_gz(&temp_archive, &temp_extract_dir)?;
    }

    let _ = fs::remove_file(&temp_archive);

    // Find the inner directory Azul created (e.g. `zulu21.52.203-ca-jre21.0.12.1-...`)
    let mut inner_dir = None;
    for entry in fs::read_dir(&temp_extract_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            inner_dir = Some(entry.path());
            break;
        }
    }

    let target_java21 = runtime_dir.join("java21");
    let _ = fs::remove_dir_all(&target_java21);

    if let Some(src) = inner_dir {
        fs::rename(src, &target_java21).map_err(|e| format!("Failed to move extracted JRE: {}", e))?;
    } else {
        fs::rename(&temp_extract_dir, &target_java21).map_err(|e| format!("Failed to move extracted JRE: {}", e))?;
    }
    let _ = fs::remove_dir_all(&temp_extract_dir);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let bin_path = get_java_executable(runtime_dir);
        if bin_path.exists() {
            let _ = fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o755));
        }
    }

    let final_bin = get_java_executable(runtime_dir);
    if !final_bin.exists() {
        return Err(format!("Java binary was not found at expected path: {:?}", final_bin));
    }

    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: "Azul Zulu 21 Ready".to_string(),
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

pub fn delete_all_game_data(game_dir: &Path) -> Result<(), String> {
    if game_dir.exists() {
        let entries = fs::read_dir(game_dir).map_err(|e| format!("Failed to read game directory: {}", e))?;
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let _ = fs::remove_dir_all(&p);
            } else {
                let _ = fs::remove_file(&p);
            }
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

                    // Delete files specified in patch removal list
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
    let normalized = rel_path.trim_start_matches('/').trim_start_matches('\\');
    if normalized.is_empty() || normalized.contains("..") {
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
        let raw_name = entry.name().to_string();
        let mangled = entry.mangled_name();

        // Check if file is protected
        let is_protected = protected_files.iter().any(|&pf| raw_name == pf)
            || protected_prefixes.iter().any(|&pp| raw_name.starts_with(pp));

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

pub async fn ensure_assets(game_dir: &Path) -> Result<PathBuf, String> {
    let assets_dir = game_dir.join("assets");
    let indexes_dir = assets_dir.join("indexes");
    let index_file = indexes_dir.join("5.json");

    let _ = fs::create_dir_all(&indexes_dir);

    // If local Modrinth meta assets exist, symlink them so the client has full assets immediately
    let modrinth_assets = PathBuf::from("/home/varixxl/.local/share/ModrinthApp/meta/assets");
    let objects_dir = assets_dir.join("objects");
    if !objects_dir.exists() && modrinth_assets.join("objects").exists() {
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(modrinth_assets.join("objects"), &objects_dir);
    }

    if !index_file.exists() {
        if modrinth_assets.join("indexes/5.json").exists() {
            let _ = fs::copy(modrinth_assets.join("indexes/5.json"), &index_file);
        } else {
            let client = Client::new();
            if let Ok(res) = client
                .get("https://piston-meta.mojang.com/v1/packages/3d1e8e9cad62f188270564f4bdfd3a79df9be65a/5.json")
                .send()
                .await
            {
                if let Ok(bytes) = res.bytes().await {
                    let _ = fs::write(&index_file, bytes);
                }
            }
        }
    }

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
        if !is_rule_allowed(&lib.rules) {
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

        if let (Some(rel), Some(url)) = (rel_path, download_url) {
            let full_dest = lib_dir.join(&rel);
            if !full_dest.exists() {
                missing_libs.push((full_dest, url, rel.contains("natives")));
            } else if rel.contains("natives") {
                extract_natives_from_jar(&full_dest, &natives_dir);
            }
        }
    }

    if !missing_libs.is_empty() {
        let total = missing_libs.len();
        for (idx, (full_dest, url, is_native)) in missing_libs.into_iter().enumerate() {
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

            if let Ok(res) = client.get(&url).header("User-Agent", "CreateClient-Launcher").send().await {
                if res.status().is_success() {
                    if let Ok(bytes) = res.bytes().await {
                        let _ = fs::write(&full_dest, bytes);
                        if is_native {
                            extract_natives_from_jar(&full_dest, &natives_dir);
                        }
                    }
                }
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
            let client_url = "https://piston-data.mojang.com/v1/objects/43db9b498cb67058d2e12d394e6507722e71bb45/client.jar";
            if let Ok(res) = client.get(client_url).header("User-Agent", "CreateClient-Launcher").send().await {
                if res.status().is_success() {
                    if let Ok(bytes) = res.bytes().await {
                        let _ = fs::write(&client_jar, bytes);
                    }
                }
            }
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
                            if !dest.exists() {
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
}
