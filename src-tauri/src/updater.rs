use serde::{Deserialize, Serialize};
use std::fs;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use tauri::AppHandle;

pub const CURRENT_VERSION: &str = "1.1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientUpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub release_url: String,
    pub changelog: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub async fn check_for_update() -> Result<ClientUpdateInfo, String> {
    let client = reqwest::Client::new();
    let url = "https://api.github.com/repos/Varixxl/Create-Client/releases/latest";

    let res = client
        .get(url)
        .header("User-Agent", "CreateClient-Launcher")
        .send()
        .await
        .map_err(|e| format!("Failed to query GitHub for client update: {}", e))?;

    if !res.status().is_success() {
        return Ok(ClientUpdateInfo {
            has_update: false,
            current_version: CURRENT_VERSION.to_string(),
            latest_version: CURRENT_VERSION.to_string(),
            download_url: None,
            asset_name: None,
            release_url: "https://github.com/Varixxl/Create-Client/releases".to_string(),
            changelog: None,
        });
    }

    let release = res
        .json::<GitHubRelease>()
        .await
        .map_err(|e| format!("Failed to parse GitHub release: {}", e))?;

    let latest_clean = release.tag_name.trim_start_matches('v').to_string();
    let current_clean = CURRENT_VERSION.trim_start_matches('v').to_string();

    let has_update = latest_clean != current_clean && !release.tag_name.is_empty();

    let matching_asset = find_matching_asset(&release.assets);

    Ok(ClientUpdateInfo {
        has_update,
        current_version: CURRENT_VERSION.to_string(),
        latest_version: release.tag_name.clone(),
        download_url: matching_asset.map(|a| a.browser_download_url.clone()),
        asset_name: matching_asset.map(|a| a.name.clone()),
        release_url: release.html_url,
        changelog: release.body,
    })
}

fn find_matching_asset<'a>(assets: &'a [GitHubAsset]) -> Option<&'a GitHubAsset> {
    #[cfg(target_os = "windows")]
    {
        assets.iter().find(|a| {
            let n = a.name.to_lowercase();
            n.ends_with(".exe") && (n.contains("create-client") || n.contains("client"))
        })
    }
    #[cfg(target_os = "linux")]
    {
        // Prioritize raw standalone binary or AppImage first
        if let Some(raw_bin) = assets.iter().find(|a| {
            let n = a.name.to_lowercase();
            n == "create-client" || n.ends_with(".appimage")
        }) {
            return Some(raw_bin);
        }

        // Fall back to .tar.gz archive if standalone binary asset is not present
        assets.iter().find(|a| {
            let n = a.name.to_lowercase();
            n.starts_with("create-client-linux")
                || (n.starts_with("create-client-") && n.ends_with(".tar.gz"))
                || n == "create-client.tar.gz"
        })
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        assets.first()
    }
}

fn validate_update_url(download_url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(download_url)
        .map_err(|e| format!("Invalid update download URL: {}", e))?;
    if parsed.scheme() != "https" {
        return Err("Update download URL must use HTTPS".to_string());
    }
    let host = parsed.host_str().unwrap_or("");
    let path = parsed.path().to_lowercase();
    let is_trusted_github = (host == "github.com"
        && (path.starts_with("/varixxl/create-client/releases/download/")
            || path.starts_with("/javidabomb/create-client/releases/download/")))
        || host == "objects.githubusercontent.com"
        || host == "release-assets.githubusercontent.com";

    if !is_trusted_github {
        return Err(format!("Untrusted update host or repository path: {}", download_url));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn extract_binary_from_tar_gz(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(bytes));
    let mut archive = tar::Archive::new(gz);
    let entries = archive
        .entries()
        .map_err(|e| format!("Failed to read update tar.gz archive: {}", e))?;

    for entry_res in entries {
        let mut entry = entry_res.map_err(|e| format!("Invalid tar entry: {}", e))?;
        if let Ok(path) = entry.path() {
            if path.file_name().and_then(|n| n.to_str()) == Some("create-client") {
                let mut buf = Vec::new();
                entry
                    .read_to_end(&mut buf)
                    .map_err(|e| format!("Failed to read binary from tar.gz: {}", e))?;
                return Ok(buf);
            }
        }
    }
    Err("Could not find 'create-client' binary inside downloaded .tar.gz archive".to_string())
}

pub async fn perform_update(_app: &AppHandle, download_url: &str) -> Result<String, String> {
    validate_update_url(download_url)?;

    let client = reqwest::Client::new();
    let res = client
        .get(download_url)
        .header("User-Agent", "CreateClient-Launcher")
        .send()
        .await
        .map_err(|e| format!("Failed to download client update: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Download failed with status: {}", res.status()));
    }

    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("Failed to read update stream: {}", e))?;

    #[cfg(target_os = "linux")]
    {
        // If the downloaded payload is a gzip tarball (.tar.gz), extract the inner binary first
        let is_gzip = bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b;
        let bin_bytes: Vec<u8> = if is_gzip || download_url.to_lowercase().ends_with(".tar.gz") {
            extract_binary_from_tar_gz(&bytes)?
        } else {
            bytes.to_vec()
        };

        if bin_bytes.len() < 4 || &bin_bytes[0..4] != b"\x7fELF" {
            return Err("Downloaded update is not a valid Linux ELF executable".to_string());
        }

        let target_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let bin_path = PathBuf::from(&home).join(".local/bin/create-client");
        let actual_target = if bin_path.exists() {
            bin_path
        } else {
            target_path
        };

        let tmp_path = actual_target.with_extension("tmp_update");
        fs::write(&tmp_path, &bin_bytes).map_err(|e| format!("Failed to write update: {}", e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755));
        }

        fs::rename(&tmp_path, &actual_target).map_err(|e| format!("Failed to replace binary: {}", e))?;

        let actual_str = actual_target.to_string_lossy().to_string();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1200));
            let _ = std::process::Command::new(&actual_str).spawn();
            std::process::exit(0);
        });

        Ok("Create Client updated in place! Restarting launcher...".to_string())
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        if bytes.len() < 2 || &bytes[0..2] != b"MZ" {
            return Err("Downloaded update is not a valid Windows executable".to_string());
        }

        let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let old_exe = current_exe.with_extension("old");
        let _ = fs::remove_file(&old_exe);

        // 1. Rename running exe to .old so the current_exe path is completely free
        fs::rename(&current_exe, &old_exe).map_err(|e| format!("Failed to rename running executable: {}", e))?;

        // 2. Write new binary directly to current_exe (exact same location, exact same file name!)
        if let Err(e) = fs::write(&current_exe, &bytes) {
            let _ = fs::rename(&old_exe, &current_exe);
            return Err(format!("Failed to write updated executable: {}", e));
        }

        // 3. Spawn a hidden detached cmd background process that waits for this process to exit, deletes old_exe, and restarts the updated client
        let old_str = old_exe.to_string_lossy().to_string();
        let current_str = current_exe.to_string_lossy().to_string();

        let cmd_script = format!(
            "ping 127.0.0.1 -n 3 > NUL & del /F /Q \"{}\" & start \"\" \"{}\"",
            old_str, current_str
        );

        let _ = std::process::Command::new("cmd")
            .args(["/C", &cmd_script])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();

        // 4. Exit this process cleanly after a short delay so the frontend displays the update message and file lock is released
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(1200));
            std::process::exit(0);
        });

        Ok("Create Client updated in place! Restarting launcher...".to_string())
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Ok("Update downloaded successfully.".to_string())
    }
}
