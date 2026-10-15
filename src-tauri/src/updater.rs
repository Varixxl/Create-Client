use serde::{Deserialize, Serialize};
use std::fs;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use tauri::AppHandle;

pub const CURRENT_VERSION: &str = "1.0.0";

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
    let url = "https://api.github.com/repos/JaviDaBomb/Create-Client/releases/latest";

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
            release_url: "https://github.com/JaviDaBomb/Create-Client/releases".to_string(),
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
        assets.iter().find(|a| {
            let n = a.name.to_lowercase();
            n == "create-client"
                || n.starts_with("create-client-linux")
                || n.ends_with(".appimage")
                || n == "create-client.tar.gz"
        })
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        assets.first()
    }
}

pub async fn perform_update(_app: &AppHandle, download_url: &str) -> Result<String, String> {
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
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let bin_path = PathBuf::from(&home).join(".local/bin/create-client");
        let target_path = if bin_path.exists() {
            bin_path
        } else {
            std::env::current_exe().map_err(|e| e.to_string())?
        };

        let tmp_path = target_path.with_extension("tmp_update");
        fs::write(&tmp_path, &bytes).map_err(|e| format!("Failed to write update: {}", e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755));
        }

        fs::rename(&tmp_path, &target_path).map_err(|e| format!("Failed to replace binary: {}", e))?;
        Ok("Create Client updated successfully! Restart the launcher to apply changes.".to_string())
    }

    #[cfg(target_os = "windows")]
    {
        let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let update_exe = current_exe.with_extension("new.exe");
        fs::write(&update_exe, &bytes).map_err(|e| format!("Failed to write update: {}", e))?;
        Ok("Update downloaded to create-client.new.exe. Please restart to apply.".to_string())
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Ok("Update downloaded successfully.".to_string())
    }
}
