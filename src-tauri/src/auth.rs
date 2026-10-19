use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

pub const ALLOWLISTED_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftProfile {
    pub uuid: String,
    pub username: String,
    pub skin_url: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

impl MinecraftProfile {
    pub fn redacted_for_frontend(&self) -> Self {
        Self {
            uuid: self.uuid.clone(),
            username: self.username.clone(),
            skin_url: self.skin_url.clone(),
            access_token: String::new(),
            refresh_token: String::new(),
            expires_at: self.expires_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub profile: Option<MinecraftProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeInfo {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

fn get_auth_file_path() -> std::path::PathBuf {
    crate::config::get_config_dir().join("auth.json")
}

fn tighten_auth_file_permissions(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.exists() {
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
        }
    }
}

pub fn load_saved_profile() -> Option<MinecraftProfile> {
    let path = get_auth_file_path();
    if path.exists() {
        tighten_auth_file_permissions(&path);
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(profile) = serde_json::from_str::<MinecraftProfile>(&content) {
                return Some(profile);
            }
        }
    }
    None
}

pub fn save_profile(profile: &MinecraftProfile) -> Result<(), String> {
    let path = get_auth_file_path();
    let data = serde_json::to_string_pretty(profile).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())?;
    tighten_auth_file_permissions(&path);
    Ok(())
}

pub fn clear_profile() -> Result<(), String> {
    let path = get_auth_file_path();
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

// ---------------- Device Code Flow ----------------

#[derive(Deserialize)]
struct DeviceCodeResponse {
    user_code: String,
    device_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: Option<u64>,
}

pub async fn start_device_flow() -> Result<DeviceCodeInfo, String> {
    let http = Client::new();

    let params = [
        ("client_id", ALLOWLISTED_CLIENT_ID),
        ("scope", "XboxLive.signin offline_access"),
    ];

    let res = http
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to initiate device code request: {}", e))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(format!("Device code request failed: {}", err_text));
    }

    let data: DeviceCodeResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse device code JSON: {}", e))?;

    // Open browser to verification link
    let _ = open::that(&data.verification_uri);

    Ok(DeviceCodeInfo {
        user_code: data.user_code,
        device_code: data.device_code,
        verification_uri: data.verification_uri,
        expires_in: data.expires_in,
        interval: data.interval.unwrap_or(5),
    })
}

#[derive(Deserialize)]
struct DeviceTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    error: Option<String>,
}

pub async fn poll_device_flow(device_code: &str) -> Result<Option<MinecraftProfile>, String> {
    let http = Client::new();

    let params = [
        ("client_id", ALLOWLISTED_CLIENT_ID),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", device_code),
    ];

    let res = http
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token poll failed: {}", e))?;

    let token_data: DeviceTokenResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    if let Some(err) = token_data.error {
        if err == "authorization_pending" || err == "slow_down" {
            return Ok(None); // Still waiting for player to click approve
        }
        return Err(format!("Authorization error: {}", err));
    }

    if let (Some(ms_access), Some(ms_refresh)) = (token_data.access_token, token_data.refresh_token) {
        // Player approved! Complete Xbox Live & Minecraft Services chain
        let profile = authenticate_with_ms_token(ALLOWLISTED_CLIENT_ID, &ms_access, &ms_refresh).await?;
        return Ok(Some(profile));
    }

    Ok(None)
}

// ---------------- Background Token Refresh ----------------

pub async fn refresh_saved_session(client_id: &str, refresh_token: &str) -> Result<MinecraftProfile, String> {
    let mut last_err = String::new();

    // Always prioritize ALLOWLISTED_CLIENT_ID since device flow issues tokens under it
    let mut candidate_ids = vec![ALLOWLISTED_CLIENT_ID];
    let trimmed = client_id.trim();
    if !trimmed.is_empty() && trimmed != ALLOWLISTED_CLIENT_ID {
        candidate_ids.push(trimmed);
    }

    for cid in candidate_ids {
        for attempt in 1..=3 {
            match try_refresh_with_client_id(cid, refresh_token).await {
                Ok(profile) => return Ok(profile),
                Err(e) => {
                    last_err = e;
                    // If Microsoft explicitly says invalid_grant, don't retry same client_id
                    if last_err.contains("invalid_grant") {
                        break;
                    }
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_millis(600 * attempt)).await;
                    }
                }
            }
        }
    }

    Err(last_err)
}

async fn try_refresh_with_client_id(cid: &str, refresh_token: &str) -> Result<MinecraftProfile, String> {
    let http = Client::new();

    let token_params = [
        ("client_id", cid),
        ("scope", "XboxLive.signin offline_access"),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let ms_res = http
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&token_params)
        .send()
        .await
        .map_err(|e| format!("Microsoft refresh request failed: {}", e))?;

    if !ms_res.status().is_success() {
        let err_text = ms_res.text().await.unwrap_or_default();
        return Err(format!("Microsoft refresh error: {}", err_text));
    }

    #[derive(Deserialize)]
    struct MsRefreshResponse {
        access_token: String,
        refresh_token: Option<String>,
    }

    let ms_tokens: MsRefreshResponse = ms_res
        .json()
        .await
        .map_err(|e| format!("Failed to parse refreshed token JSON: {}", e))?;

    let new_refresh = ms_tokens.refresh_token.as_deref().unwrap_or(refresh_token);
    authenticate_with_ms_token(cid, &ms_tokens.access_token, new_refresh).await
}

pub async fn ensure_valid_profile(force_refresh: bool) -> Result<Option<MinecraftProfile>, String> {
    let profile = match load_saved_profile() {
        Some(p) => p,
        None => return Ok(None),
    };

    let now_sec = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Refresh if forced (e.g., on game launch) or if less than 12 hours remain before expiry
    let needs_refresh = force_refresh || profile.expires_at <= now_sec + 43200;

    if needs_refresh {
        let cfg = crate::config::load_config();
        match refresh_saved_session(&cfg.azure_client_id, &profile.refresh_token).await {
            Ok(refreshed) => return Ok(Some(refreshed)),
            Err(err) => {
                // If existing access_token is still valid for >5 minutes, fall back to it gracefully
                if profile.expires_at > now_sec + 300 {
                    return Ok(Some(profile));
                }
                return Err(err);
            }
        }
    }

    Ok(Some(profile))
}

// ---------------- Xbox Live, XSTS & Minecraft Services ----------------

#[derive(Deserialize)]
struct XblResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XblDisplayClaims,
}

#[derive(Deserialize)]
struct XblDisplayClaims {
    xui: Vec<XblUhs>,
}

#[derive(Deserialize)]
struct XblUhs {
    uhs: String,
}

#[derive(Deserialize)]
struct MinecraftLoginResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize)]
struct MinecraftProfileResponse {
    id: String,
    name: String,
    skins: Option<Vec<MinecraftSkinItem>>,
}

#[derive(Deserialize)]
struct MinecraftSkinItem {
    url: String,
    state: Option<String>,
}

async fn authenticate_with_ms_token(
    _client_id: &str,
    ms_access_token: &str,
    ms_refresh_token: &str,
) -> Result<MinecraftProfile, String> {
    let http = Client::new();

    // 1. Authenticate with Xbox Live (user.auth.xboxlive.com)
    let xbl_payload = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={}", ms_access_token)
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let xbl_res = http
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&xbl_payload)
        .send()
        .await
        .map_err(|e| format!("Xbox Live request failed: {}", e))?;

    if !xbl_res.status().is_success() {
        let err_text = xbl_res.text().await.unwrap_or_default();
        return Err(format!("Xbox Live auth failed: {}", err_text));
    }

    let xbl_data: XblResponse = xbl_res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Xbox Live response: {}", e))?;

    let uhs = xbl_data
        .display_claims
        .xui
        .first()
        .ok_or_else(|| "No user hash (uhs) in Xbox Live response".to_string())?
        .uhs
        .clone();

    // 2. Obtain XSTS Token for Minecraft
    let xsts_payload = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_data.token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let xsts_res = http
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&xsts_payload)
        .send()
        .await
        .map_err(|e| format!("XSTS request failed: {}", e))?;

    if !xsts_res.status().is_success() {
        let err_text = xsts_res.text().await.unwrap_or_default();
        return Err(format!("XSTS authorization failed: {}", err_text));
    }

    let xsts_data: XblResponse = xsts_res
        .json()
        .await
        .map_err(|e| format!("Failed to parse XSTS response: {}", e))?;

    // 3. Authenticate with Minecraft Services
    let mc_login_payload = serde_json::json!({
        "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_data.token)
    });

    let mc_res = http
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&mc_login_payload)
        .send()
        .await
        .map_err(|e| format!("Minecraft login request failed: {}", e))?;

    if !mc_res.status().is_success() {
        let err_text = mc_res.text().await.unwrap_or_default();
        return Err(format!("Minecraft Services login failed: {}", err_text));
    }

    let mc_login: MinecraftLoginResponse = mc_res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Minecraft login response: {}", e))?;

    // 4. Get Minecraft Profile
    let profile_res = http
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&mc_login.access_token)
        .send()
        .await
        .map_err(|e| format!("Minecraft profile fetch failed: {}", e))?;

    if !profile_res.status().is_success() {
        let err_text = profile_res.text().await.unwrap_or_default();
        return Err(format!("Minecraft profile error (verify game ownership): {}", err_text));
    }

    let profile_data: MinecraftProfileResponse = profile_res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Minecraft profile JSON: {}", e))?;

    let skin_url = profile_data
        .skins
        .and_then(|skins| skins.into_iter().find(|s| s.state.as_deref() == Some("ACTIVE")).or_else(|| None))
        .map(|s| s.url);

    let now_sec = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let profile = MinecraftProfile {
        uuid: profile_data.id,
        username: profile_data.name,
        skin_url,
        access_token: mc_login.access_token,
        refresh_token: ms_refresh_token.to_string(),
        expires_at: now_sec + mc_login.expires_in,
    };

    save_profile(&profile)?;
    Ok(profile)
}
