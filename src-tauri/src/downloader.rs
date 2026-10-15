use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressPayload {
    pub task: String,
    pub current_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f64,
    pub speed_bytes_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackManifest {
    pub version: String,
    pub modpack_url: String,
    pub sha256: Option<String>,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersion {
    pub version: String,
    pub installed_at: u64,
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
        runtime_dir.join("java17").join("bin").join("java.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        runtime_dir.join("java17").join("bin").join("java")
    }
}

pub async fn ensure_java(app: &AppHandle, runtime_dir: &Path) -> Result<PathBuf, String> {
    let java_bin = get_java_executable(runtime_dir);
    if java_bin.exists() {
        return Ok(java_bin);
    }

    fs::create_dir_all(runtime_dir).map_err(|e| format!("Failed to create runtime directory: {}", e))?;

    let os_name = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };

    let api_url = format!(
        "https://api.adoptium.net/v3/assets/latest/17/hotspot?architecture=x64&image_type=jre&os={}&vendor=eclipse",
        os_name
    );

    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: "Resolving Java 17 Runtime...".to_string(),
            current_bytes: 0,
            total_bytes: 100,
            percentage: 0.0,
            speed_bytes_sec: 0,
        },
    );

    let client = Client::new();
    let res = client
        .get(&api_url)
        .header("User-Agent", "CreateClient-Launcher")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Java 17 release info: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Adoptium API returned status: {}", res.status()));
    }

    let assets_json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Adoptium response: {}", e))?;

    let binary_pkg = assets_json
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("binary"))
        .and_then(|bin| bin.get("package"))
        .ok_or_else(|| "Could not find Java 17 package download in Adoptium response".to_string())?;

    let download_url = binary_pkg
        .get("link")
        .and_then(|l| l.as_str())
        .ok_or_else(|| "No download link in Adoptium binary package".to_string())?;

    let is_zip = download_url.ends_with(".zip");
    let archive_ext = if is_zip { "zip" } else { "tar.gz" };
    let temp_archive = runtime_dir.join(format!("java17_download.{}", archive_ext));

    download_file_with_progress(app, download_url, &temp_archive, "Downloading Java 17 JRE").await?;

    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: "Extracting Java 17 JRE...".to_string(),
            current_bytes: 100,
            total_bytes: 100,
            percentage: 99.0,
            speed_bytes_sec: 0,
        },
    );

    let temp_extract_dir = runtime_dir.join("java17_extract_temp");
    let _ = fs::remove_dir_all(&temp_extract_dir);
    fs::create_dir_all(&temp_extract_dir).map_err(|e| e.to_string())?;

    if is_zip {
        extract_zip(&temp_archive, &temp_extract_dir)?;
    } else {
        extract_tar_gz(&temp_archive, &temp_extract_dir)?;
    }

    let _ = fs::remove_file(&temp_archive);

    // Find the inner directory Adoptium created (e.g. `jdk-17.0.x+y-jre`)
    let mut inner_dir = None;
    for entry in fs::read_dir(&temp_extract_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            inner_dir = Some(entry.path());
            break;
        }
    }

    let target_java17 = runtime_dir.join("java17");
    let _ = fs::remove_dir_all(&target_java17);

    if let Some(src) = inner_dir {
        fs::rename(src, &target_java17).map_err(|e| format!("Failed to move extracted JRE: {}", e))?;
    } else {
        fs::rename(&temp_extract_dir, &target_java17).map_err(|e| format!("Failed to move extracted JRE: {}", e))?;
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
            task: "Java 17 ready".to_string(),
            current_bytes: 100,
            total_bytes: 100,
            percentage: 100.0,
            speed_bytes_sec: 0,
        },
    );

    Ok(final_bin)
}

pub async fn check_and_sync_modpack(
    app: &AppHandle,
    game_dir: &Path,
    manifest_url: &str,
    force_repair: bool,
) -> Result<String, String> {
    fs::create_dir_all(game_dir).map_err(|e| format!("Failed to create game directory: {}", e))?;

    let version_file = game_dir.join("client_version.json");
    let local_version: Option<String> = if version_file.exists() && !force_repair {
        fs::read_to_string(&version_file)
            .ok()
            .and_then(|c| serde_json::from_str::<InstalledVersion>(&c).ok())
            .map(|v| v.version)
    } else {
        None
    };

    let client = Client::new();
    let res = client
        .get(manifest_url)
        .header("User-Agent", "CreateClient-Launcher")
        .send()
        .await
        .map_err(|e| format!("Failed to check modpack manifest: {}", e))?;

    if !res.status().is_success() {
        if local_version.is_some() {
            // If offline or manifest fails, proceed with existing local version
            return Ok(local_version.unwrap());
        }
        return Err(format!("Could not fetch modpack manifest (HTTP {})", res.status()));
    }

    let manifest: ModpackManifest = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse modpack manifest: {}", e))?;

    if Some(&manifest.version) == local_version.as_ref() && !force_repair {
        return Ok(manifest.version);
    }

    // Need update or repair
    let temp_zip = game_dir.join("modpack_update.zip");
    download_file_with_progress(app, &manifest.modpack_url, &temp_zip, "Downloading Modpack Update").await?;

    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task: "Extracting Modpack Files...".to_string(),
            current_bytes: 100,
            total_bytes: 100,
            percentage: 95.0,
            speed_bytes_sec: 0,
        },
    );

    extract_modpack_safely(&temp_zip, game_dir)?;
    let _ = fs::remove_file(&temp_zip);

    // Save installed version
    let installed = InstalledVersion {
        version: manifest.version.clone(),
        installed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    let _ = fs::write(
        &version_file,
        serde_json::to_string_pretty(&installed).unwrap_or_default(),
    );

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

    Ok(manifest.version)
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
    ];
    let protected_prefixes = [
        "screenshots/",
        "saves/",
        "resourcepacks/",
        "shaderpacks/",
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
