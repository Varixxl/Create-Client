use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const REDIRECT_PORT: u16 = 8483;
const REDIRECT_URI: &str = "http://localhost:8483/callback";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftProfile {
    pub uuid: String,
    pub username: String,
    pub skin_url: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub profile: Option<MinecraftProfile>,
}

pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

pub fn generate_pkce() -> PkceChallenge {
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    let verifier = URL_SAFE_NO_PAD.encode(bytes);

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(hash);

    PkceChallenge { verifier, challenge }
}

fn get_auth_file_path() -> std::path::PathBuf {
    crate::config::get_config_dir().join("auth.json")
}

pub fn load_saved_profile() -> Option<MinecraftProfile> {
    let path = get_auth_file_path();
    if path.exists() {
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
    fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_profile() -> Result<(), String> {
    let path = get_auth_file_path();
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

// ---------------- Microsoft OAuth & Minecraft Services ----------------

#[derive(Deserialize)]
#[allow(dead_code)]
struct MsTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

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

pub async fn start_login_flow(client_id: String) -> Result<MinecraftProfile, String> {
    if client_id.trim().is_empty() {
        return Err("Azure Client ID is required. Please set it in Settings.".to_string());
    }

    let pkce = generate_pkce();
    let auth_url = format!(
        "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize?\
        client_id={}&\
        response_type=code&\
        redirect_uri={}&\
        scope=XboxLive.signin%20offline_access&\
        code_challenge={}&\
        code_challenge_method=S256",
        url::form_urlencoded::byte_serialize(client_id.as_bytes()).collect::<String>(),
        url::form_urlencoded::byte_serialize(REDIRECT_URI.as_bytes()).collect::<String>(),
        pkce.challenge
    );

    // Bind local TCP listener
    let listener = TcpListener::bind(format!("127.0.0.1:{}", REDIRECT_PORT))
        .await
        .map_err(|e| format!("Failed to bind local auth listener on port {}: {}", REDIRECT_PORT, e))?;

    // Open browser for the player
    let _ = open::that(&auth_url);

    // Wait for the redirect callback
    let code = match wait_for_code(listener).await {
        Ok(c) => c,
        Err(e) => return Err(format!("OAuth callback error: {}", e)),
    };

    // Complete token exchange chain
    complete_auth_chain(&client_id, &code, &pkce.verifier).await
}

async fn wait_for_code(listener: TcpListener) -> Result<String, String> {
    // Wait for incoming HTTP request
    let (mut socket, _) = listener
        .accept()
        .await
        .map_err(|e| format!("Failed to accept incoming connection: {}", e))?;

    let mut buf = [0u8; 2048];
    let bytes_read = socket
        .read(&mut buf)
        .await
        .map_err(|e| format!("Failed to read request from browser: {}", e))?;

    let req = String::from_utf8_lossy(&buf[..bytes_read]);

    let mut code = None;
    if let Some(first_line) = req.lines().next() {
        // e.g. "GET /callback?code=M.R3_BLA... HTTP/1.1"
        if let Some(path) = first_line.split_whitespace().nth(1) {
            if let Ok(url) = url::Url::parse(&format!("http://localhost{}", path)) {
                for (k, v) in url.query_pairs() {
                    if k == "code" {
                        code = Some(v.to_string());
                        break;
                    }
                }
            }
        }
    }

    // Send HTTP response to browser
    let response_body = r#"<!DOCTYPE html>
<html>
<head>
    <title>Create Client - Login Successful</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #121417; color: #e1e7ec; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }
        .card { background: #1a1e24; border: 1px solid #2e3846; border-radius: 12px; padding: 32px 40px; text-align: center; box-shadow: 0 10px 30px rgba(0,0,0,0.5); max-width: 400px; }
        h1 { color: #e6a23c; margin: 0 0 12px 0; font-size: 24px; }
        p { color: #9aa7b4; line-height: 1.5; margin: 0; }
    </style>
</head>
<body>
    <div class="card">
        <h1>Login Successful!</h1>
        <p>You have signed in to Create Client. You may now safely close this browser window and return to the game launcher.</p>
    </div>
</body>
</html>"#;

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.flush().await;

    code.ok_or_else(|| "No authorization code found in callback query".to_string())
}

async fn complete_auth_chain(
    client_id: &str,
    code: &str,
    code_verifier: &str,
) -> Result<MinecraftProfile, String> {
    let http = Client::new();

    // 1. Exchange Microsoft Auth Code for Microsoft Tokens
    let token_params = [
        ("client_id", client_id),
        ("scope", "XboxLive.signin offline_access"),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier),
    ];

    let ms_res = http
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&token_params)
        .send()
        .await
        .map_err(|e| format!("Microsoft token request failed: {}", e))?;

    if !ms_res.status().is_success() {
        let err_text = ms_res.text().await.unwrap_or_default();
        return Err(format!("Microsoft token exchange error: {}", err_text));
    }

    let ms_tokens: MsTokenResponse = ms_res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Microsoft token JSON: {}", e))?;

    authenticate_with_ms_token(client_id, &ms_tokens.access_token, &ms_tokens.refresh_token).await
}

pub async fn refresh_saved_session(client_id: &str, refresh_token: &str) -> Result<MinecraftProfile, String> {
    let http = Client::new();

    let token_params = [
        ("client_id", client_id),
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

    let ms_tokens: MsTokenResponse = ms_res
        .json()
        .await
        .map_err(|e| format!("Failed to parse refreshed token JSON: {}", e))?;

    authenticate_with_ms_token(client_id, &ms_tokens.access_token, &ms_tokens.refresh_token).await
}

async fn authenticate_with_ms_token(
    _client_id: &str,
    ms_access_token: &str,
    ms_refresh_token: &str,
) -> Result<MinecraftProfile, String> {
    let http = Client::new();

    // 2. Authenticate with Xbox Live (user.auth.xboxlive.com)
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

    // 3. Obtain XSTS Token for Minecraft
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

    // 4. Authenticate with Minecraft Services
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

    // 5. Get Minecraft Profile
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

    let _ = save_profile(&profile);
    Ok(profile)
}
