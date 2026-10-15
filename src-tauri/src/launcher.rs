use crate::auth::MinecraftProfile;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Deserialize)]
pub struct VersionMeta {
    pub id: Option<String>,
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
}

pub fn launch_minecraft(
    java_bin: &Path,
    game_dir: &Path,
    profile: &MinecraftProfile,
    ram_gb: u32,
    custom_jvm_flags: &str,
) -> Result<u32, String> {
    if !java_bin.exists() {
        return Err(format!("Java executable not found at: {:?}", java_bin));
    }
    if !game_dir.exists() {
        return Err(format!("Game directory not found at: {:?}", game_dir));
    }

    // 1. Gather all jars for classpath
    let mut classpath_entries: Vec<PathBuf> = Vec::new();

    // Check libraries directory
    let lib_dir = game_dir.join("libraries");
    if lib_dir.exists() {
        collect_jars(&lib_dir, &mut classpath_entries);
    }

    // Check client jar
    let client_jar = game_dir.join("client.jar");
    if client_jar.exists() {
        classpath_entries.push(client_jar);
    } else {
        let v_jar = game_dir.join("1.20.1.jar");
        if v_jar.exists() {
            classpath_entries.push(v_jar);
        }
    }

    let separator = if cfg!(target_os = "windows") { ";" } else { ":" };
    let classpath = classpath_entries
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(separator);

    // 2. Read version.json for mainClass if present
    let version_json_path = game_dir.join("version.json");
    let main_class = if version_json_path.exists() {
        fs::read_to_string(&version_json_path)
            .ok()
            .and_then(|c| serde_json::from_str::<VersionMeta>(&c).ok())
            .and_then(|v| v.main_class)
            .unwrap_or_else(|| "net.minecraftforge.bootstrap.ForgeBootstrap".to_string())
    } else {
        "net.minecraftforge.bootstrap.ForgeBootstrap".to_string()
    };

    let assets_dir = game_dir.join("assets");
    let natives_dir = game_dir.join("natives");

    // 3. Build command arguments
    let mut cmd = Command::new(java_bin);
    cmd.current_dir(game_dir);

    // Memory settings
    cmd.arg("-Xms2G");
    cmd.arg(format!("-Xmx{}G", ram_gb));

    // Custom JVM flags
    for flag in custom_jvm_flags.split_whitespace() {
        if !flag.is_empty() {
            cmd.arg(flag);
        }
    }

    // Standard Minecraft JVM properties
    cmd.arg(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
    cmd.arg("-Dminecraft.launcher.brand=CreateClient");
    cmd.arg("-Dminecraft.launcher.version=0.1.0");

    // Classpath
    if !classpath.is_empty() {
        cmd.arg("-cp");
        cmd.arg(&classpath);
    }

    // Main Class
    cmd.arg(&main_class);

    // Game arguments
    cmd.arg("--username").arg(&profile.username);
    cmd.arg("--version").arg("1.20.1");
    cmd.arg("--gameDir").arg(game_dir.to_string_lossy().to_string());
    cmd.arg("--assetsDir").arg(assets_dir.to_string_lossy().to_string());
    cmd.arg("--assetIndex").arg("5");
    cmd.arg("--uuid").arg(&profile.uuid);
    cmd.arg("--accessToken").arg(&profile.access_token);
    cmd.arg("--userType").arg("msa");
    cmd.arg("--versionType").arg("Forge");

    // Ensure servers.dat exists with Create Server if not present
    ensure_default_server_in_game_dir(game_dir);

    // Spawn process
    #[cfg(unix)]
    {
        // On Unix, spawn detached so closing the launcher won't kill the game
        cmd.stdin(Stdio::null());
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn Minecraft process: {}", e))?;

    Ok(child.id())
}

fn collect_jars(dir: &Path, jars: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_jars(&path, jars);
            } else if path.extension().and_then(|s| s.to_str()) == Some("jar") {
                jars.push(path);
            }
        }
    }
}

fn ensure_default_server_in_game_dir(game_dir: &Path) {
    let servers_dat = game_dir.join("servers.dat");
    if !servers_dat.exists() {
        // If servers.dat does not exist yet, we can create or copy from template
        // Note: FancyMenu or modpack zip normally bundles this.
    }
}
