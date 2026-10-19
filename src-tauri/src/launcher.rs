use crate::auth::MinecraftProfile;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use tauri::{AppHandle, Emitter};

static IS_MINECRAFT_RUNNING: AtomicBool = AtomicBool::new(false);
static MINECRAFT_PID: AtomicU32 = AtomicU32::new(0);
static WAS_FORCE_CLOSED: AtomicBool = AtomicBool::new(false);

pub fn is_minecraft_running() -> bool {
    IS_MINECRAFT_RUNNING.load(Ordering::SeqCst)
}

pub fn kill_minecraft(app: &AppHandle) -> Result<(), String> {
    let pid = MINECRAFT_PID.load(Ordering::SeqCst);
    if pid == 0 || !IS_MINECRAFT_RUNNING.load(Ordering::SeqCst) {
        return Ok(());
    }

    WAS_FORCE_CLOSED.store(true, Ordering::SeqCst);

    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-9", &format!("-{}", pid)])
            .output();
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    MINECRAFT_PID.store(0, Ordering::SeqCst);
    IS_MINECRAFT_RUNNING.store(false, Ordering::SeqCst);
    let _ = app.emit("game-status", false);
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionMeta {
    pub id: Option<String>,
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    #[serde(default)]
    pub libraries: Vec<VersionLibrary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionLibrary {
    pub name: Option<String>,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    pub include_in_classpath: Option<bool>,
    #[serde(default)]
    pub rules: Vec<LibraryRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<LibraryArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryArtifact {
    pub path: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryRule {
    pub action: Option<String>,
    pub os: Option<RuleOs>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuleOs {
    pub name: Option<String>,
}

pub fn is_rule_allowed(rules: &[LibraryRule]) -> bool {
    if rules.is_empty() {
        return true;
    }
    let current_os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };

    let mut allowed = false;
    for r in rules {
        let action = r.action.as_deref().unwrap_or("");
        let os_name = r.os.as_ref().and_then(|o| o.name.as_deref());
        if action == "allow" {
            if os_name.is_none() || os_name == Some(current_os) {
                allowed = true;
            }
        } else if action == "disallow" {
            if os_name == Some(current_os) {
                allowed = false;
            }
        }
    }
    allowed
}

pub fn is_library_for_current_platform(lib: &VersionLibrary) -> bool {
    if !is_rule_allowed(&lib.rules) {
        return false;
    }

    if let Some(name) = &lib.name {
        // Filter out non-x86_64 architectures on x86_64 machines (both Windows and Linux)
        #[cfg(target_arch = "x86_64")]
        {
            if name.ends_with("-x86")
                || name.contains("-x86-")
                || name.contains("-arm64")
                || name.contains("-arm32")
                || name.contains("aarch_64")
                || name.contains("linux-arm")
                || name.contains("osx-arm64")
            {
                return false;
            }
        }

        // Filter out non-aarch64 architectures on aarch64 machines
        #[cfg(target_arch = "aarch64")]
        {
            if name.ends_with("-x86")
                || name.contains("-x86-")
                || name.contains("-arm32")
                || name.contains("x86_64")
                || name.ends_with(":natives-windows")
                || name.ends_with(":natives-linux")
            {
                return false;
            }
        }
    }

    true
}

pub fn maven_to_path(name: &str) -> Option<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let mut classifier = String::new();
    let mut ext = "jar".to_string();

    if parts.len() > 3 {
        let rem = parts[3];
        if let Some((c, e)) = rem.split_once('@') {
            classifier = format!("-{}", c);
            ext = e.to_string();
        } else {
            classifier = format!("-{}", rem);
        }
    }
    Some(format!(
        "{}/{}/{}/{}-{}{}.{}",
        group, artifact, version, artifact, version, classifier, ext
    ))
}

pub fn detect_java_major_version(java_bin: &Path) -> u32 {
    let path_str = java_bin.to_string_lossy();
    if path_str.contains("java25") {
        return 25;
    } else if path_str.contains("java21") {
        return 21;
    } else if path_str.contains("java17") {
        return 17;
    } else if path_str.contains("java8") {
        return 8;
    }

    if let Ok(output) = Command::new(java_bin).arg("-version").output() {
        let err_text = String::from_utf8_lossy(&output.stderr);
        let out_text = String::from_utf8_lossy(&output.stdout);
        let text = format!("{}\n{}", err_text, out_text);
        for line in text.lines() {
            if line.contains("version") {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        let ver_str = &line[start + 1..start + 1 + end];
                        let parts: Vec<&str> = ver_str.split('.').collect();
                        if let Some(first) = parts.first() {
                            if *first == "1" {
                                if let Some(second) = parts.get(1) {
                                    if let Ok(v) = second.parse::<u32>() {
                                        return v;
                                    }
                                }
                            } else if let Ok(v) = first.parse::<u32>() {
                                return v;
                            }
                        }
                    }
                }
            }
        }
    }
    21
}

pub fn launch_minecraft(
    app: &AppHandle,
    profile_id: &str,
    java_bin: &Path,
    game_dir: &Path,
    profile: &MinecraftProfile,
    ram_gb: u32,
    custom_jvm_flags: &str,
    auto_connect: bool,
    graphics_preset: &str,
) -> Result<u32, String> {
    if IS_MINECRAFT_RUNNING.load(Ordering::SeqCst) {
        return Err("Minecraft is already running.".to_string());
    }
    if !java_bin.exists() {
        return Err(format!("Java executable not found at: {:?}", java_bin));
    }
    if !game_dir.exists() {
        return Err(format!("Game directory not found at: {:?}", game_dir));
    }

    // Detect display refresh rate and apply requested graphics preset (or auto-detected tier)
    let refresh_rate = detect_refresh_rate();
    apply_graphics_preset(game_dir, graphics_preset);
    sanitize_options_txt(game_dir, refresh_rate);

    let lib_dir = game_dir.join("libraries");
    let separator = if cfg!(target_os = "windows") { ";" } else { ":" };

    // 1. Gather classpath libraries filtered by version.json
    let mut classpath_entries: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    let version_file = game_dir.join("version.json");
    if version_file.exists() {
        if let Ok(content) = fs::read_to_string(&version_file) {
            if let Ok(version_meta) = serde_json::from_str::<VersionMeta>(&content) {
                for lib in version_meta.libraries {
                    if lib.include_in_classpath == Some(false) {
                        continue;
                    }
                    if !is_library_for_current_platform(&lib) {
                        continue;
                    }

                    let rel_path = lib
                        .downloads
                        .as_ref()
                        .and_then(|d| d.artifact.as_ref())
                        .and_then(|a| a.path.as_ref())
                        .cloned()
                        .or_else(|| lib.name.as_deref().and_then(maven_to_path));

                    if let Some(rel) = rel_path {
                        let full = lib_dir.join(rel);
                        if full.exists() && !seen.contains(&full) {
                            seen.insert(full.clone());
                            classpath_entries.push(full);
                        }
                    }
                }
            }
        }
    }

    // Fallback if version.json wasn't parsed or had no libraries
    if classpath_entries.is_empty() && lib_dir.exists() {
        collect_jars(&lib_dir, &mut classpath_entries);
    }

    // Add vanilla client jar if present
    let client_jar = game_dir.join("1.20.1.jar");
    if client_jar.exists() && !seen.contains(&client_jar) {
        seen.insert(client_jar.clone());
        classpath_entries.push(client_jar);
    } else {
        let alt_jar = game_dir.join("client.jar");
        if alt_jar.exists() && !seen.contains(&alt_jar) {
            seen.insert(alt_jar.clone());
            classpath_entries.push(alt_jar);
        }
    }

    let classpath = classpath_entries
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(separator);

    // 2. Module Path (Required for Forge 1.20.1)
    let module_relative_jars = [
        "cpw/mods/bootstraplauncher/1.1.2/bootstraplauncher-1.1.2.jar",
        "cpw/mods/securejarhandler/2.1.10/securejarhandler-2.1.10.jar",
        "org/ow2/asm/asm-commons/9.9.1/asm-commons-9.9.1.jar",
        "org/ow2/asm/asm-util/9.9.1/asm-util-9.9.1.jar",
        "org/ow2/asm/asm-analysis/9.9.1/asm-analysis-9.9.1.jar",
        "org/ow2/asm/asm-tree/9.9.1/asm-tree-9.9.1.jar",
        "org/ow2/asm/asm/9.9.1/asm-9.9.1.jar",
        "net/minecraftforge/JarJarFileSystems/0.3.19/JarJarFileSystems-0.3.19.jar",
    ];

    let module_path = module_relative_jars
        .iter()
        .map(|rel| lib_dir.join(rel).to_string_lossy().to_string())
        .filter(|p| Path::new(p).exists())
        .collect::<Vec<_>>()
        .join(separator);

    // 3. Main class
    let main_class = "cpw.mods.bootstraplauncher.BootstrapLauncher".to_string();

    // 4. Assets Directory
    let assets_dir = if game_dir.join("assets").join("indexes").join("5.json").exists() {
        game_dir.join("assets")
    } else {
        let modrinth_assets = PathBuf::from("/home/varixxl/.local/share/ModrinthApp/meta/assets");
        if modrinth_assets.join("indexes").join("5.json").exists() {
            modrinth_assets
        } else {
            game_dir.join("assets")
        }
    };

    let natives_dir = game_dir.join("natives");

    // 5. Build Command
    let mut cmd = Command::new(java_bin);
    cmd.current_dir(game_dir);

    // Memory settings
    cmd.arg("-Xms512M");
    cmd.arg(format!("-Xmx{}G", ram_gb));

    // Custom JVM flags (deduplicated, with ZGC fallback only if no GC flag is specified)
    let mut final_jvm_flags: Vec<String> = Vec::new();
    for flag in custom_jvm_flags.split_whitespace() {
        if !flag.is_empty() && !final_jvm_flags.iter().any(|f| f == flag) {
            final_jvm_flags.push(flag.to_string());
        }
    }

    let java_major = detect_java_major_version(java_bin);
    let has_gc_flag = final_jvm_flags
        .iter()
        .any(|f| f.starts_with("-XX:+Use") && f.ends_with("GC"));

    if !has_gc_flag {
        if java_major < 21 {
            for fallback in ["-XX:+UseG1GC", "-XX:+AlwaysPreTouch"] {
                if !final_jvm_flags.iter().any(|f| f == fallback) {
                    final_jvm_flags.push(fallback.to_string());
                }
            }
        } else {
            for fallback in ["-XX:+UseZGC", "-XX:+ZGenerational", "-XX:+AlwaysPreTouch"] {
                if !final_jvm_flags.iter().any(|f| f == fallback) {
                    final_jvm_flags.push(fallback.to_string());
                }
            }
        }
    }

    for flag in &final_jvm_flags {
        cmd.arg(flag);
    }

    // Standard JVM Properties
    cmd.arg(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
    cmd.arg(format!("-Djna.tmpdir={}", natives_dir.to_string_lossy()));
    cmd.arg(format!("-Dorg.lwjgl.system.SharedLibraryExtractPath={}", natives_dir.to_string_lossy()));
    cmd.arg(format!("-Dio.netty.native.workdir={}", natives_dir.to_string_lossy()));
    cmd.arg("-Dminecraft.launcher.brand=CreateClient");
    cmd.arg("-Dminecraft.launcher.version=1.0.9");
    cmd.arg("-Djava.net.preferIPv6Addresses=system");
    cmd.arg("-DignoreList=bootstraplauncher,securejarhandler,asm-commons,asm-util,asm-analysis,asm-tree,asm,JarJarFileSystems,client-extra,fmlcore,javafmllanguage,lowcodelanguage,mclanguage,forge-,1.20.1-47.4.20.jar,1.20.1.jar");
    cmd.arg("-DmergeModules=jna-5.10.0.jar,jna-platform-5.10.0.jar");
    cmd.arg(format!("-DlibraryDirectory={}", lib_dir.to_string_lossy()));

    // Module-path configuration
    if !module_path.is_empty() {
        cmd.arg("-p").arg(&module_path);
        cmd.arg("--add-modules").arg("ALL-MODULE-PATH");
        cmd.arg("--add-opens").arg("java.base/java.util.jar=cpw.mods.securejarhandler");
        cmd.arg("--add-opens").arg("java.base/java.lang.invoke=cpw.mods.securejarhandler");
        cmd.arg("--add-exports").arg("java.base/sun.security.util=cpw.mods.securejarhandler");
        cmd.arg("--add-exports").arg("jdk.naming.dns/com.sun.jndi.dns=java.naming");
        cmd.arg("--add-opens").arg("java.base/java.lang.reflect=ALL-UNNAMED");
    }

    // Classpath
    if !classpath.is_empty() {
        cmd.arg("-cp").arg(&classpath);
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
    cmd.arg("--clientId").arg(crate::auth::ALLOWLISTED_CLIENT_ID);
    cmd.arg("--xuid").arg("0");
    cmd.arg("--userType").arg("msa");
    cmd.arg("--versionType").arg("Forge");

    // Forge specific arguments
    cmd.arg("--launchTarget").arg("forgeclient");
    cmd.arg("--fml.forgeVersion").arg("47.4.20");
    cmd.arg("--fml.mcVersion").arg("1.20.1");
    cmd.arg("--fml.forgeGroup").arg("net.minecraftforge");
    cmd.arg("--fml.mcpVersion").arg("20230612.114412");

    // Auto-connect to Create Server if configured
    if auto_connect {
        cmd.arg("--quickPlayMultiplayer").arg("argon.dathost.net:17155");
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.stdin(Stdio::null());
        cmd.process_group(0);

        let current_ld = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        let new_ld = if current_ld.is_empty() {
            natives_dir.to_string_lossy().to_string()
        } else {
            format!("{}:{}", natives_dir.to_string_lossy(), current_ld)
        };
        cmd.env("LD_LIBRARY_PATH", new_ld);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);

        let current_path = std::env::var("PATH").unwrap_or_default();
        let java_bin_dir = java_bin.parent().unwrap_or(Path::new(""));
        let new_path = format!(
            "{};{};{}",
            natives_dir.to_string_lossy(),
            java_bin_dir.to_string_lossy(),
            current_path
        );
        cmd.env("PATH", new_path);
    }

    // Redirect stdout and stderr to launcher_log.txt in game directory for easy debugging
    let log_file_path = game_dir.join("launcher_log.txt");
    if let Ok(log_file) = fs::File::create(&log_file_path) {
        if let Ok(err_file) = log_file.try_clone() {
            cmd.stdout(Stdio::from(log_file));
            cmd.stderr(Stdio::from(err_file));
        }
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn Minecraft process: {}", e))?;

    let pid = child.id();
    MINECRAFT_PID.store(pid, Ordering::SeqCst);
    IS_MINECRAFT_RUNNING.store(true, Ordering::SeqCst);
    WAS_FORCE_CLOSED.store(false, Ordering::SeqCst);
    let app_handle = app.clone();
    let game_dir_buf = game_dir.to_path_buf();

    let profile_id_str = profile_id.to_string();
    std::thread::spawn(move || {
        let _ = app_handle.emit("game-status", true);
        let wait_res = child.wait();
        MINECRAFT_PID.store(0, Ordering::SeqCst);
        IS_MINECRAFT_RUNNING.store(false, Ordering::SeqCst);
        let _ = app_handle.emit("game-status", false);

        let was_forced = WAS_FORCE_CLOSED.swap(false, Ordering::SeqCst);
        if !was_forced {
            if let Ok(exit_status) = wait_res {
                if !exit_status.success() {
                    let diagnosis = crate::diagnostics::diagnose_instance_crash(
                        &profile_id_str,
                        &game_dir_buf,
                        exit_status.code(),
                    );
                    let err_summary = format!(
                        "[{}] {}\n\nRoot Cause:\n{}\n\nSuggested Fix:\n{}",
                        diagnosis.issue_category,
                        diagnosis.summary,
                        diagnosis.root_cause,
                        diagnosis.suggested_fix
                    );
                    let _ = app_handle.emit("game-error", err_summary);
                    let _ = app_handle.emit("game-crash-diagnosis", diagnosis);
                }
            }
        }
    });

    Ok(pid)
}

pub fn launch_fabric_or_vanilla(
    app: &AppHandle,
    profile_id: &str,
    java_bin: &Path,
    instance_dir: &Path,
    profile: &MinecraftProfile,
    ram_gb: u32,
    custom_jvm_flags: &str,
) -> Result<u32, String> {
    if IS_MINECRAFT_RUNNING.load(Ordering::SeqCst) {
        return Err("Minecraft is already running.".to_string());
    }
    if !java_bin.exists() {
        return Err(format!("Java executable not found at: {:?}", java_bin));
    }
    if !instance_dir.exists() {
        return Err(format!("Instance directory not found at: {:?}", instance_dir));
    }

    let meta_path = instance_dir.join("fabric_launch_meta.json");
    if !meta_path.exists() {
        return Err(
            "Instance metadata not found. Please sync this profile first before launching."
                .to_string(),
        );
    }

    let meta_content = fs::read_to_string(&meta_path)
        .map_err(|e| format!("Failed to read fabric_launch_meta.json: {}", e))?;
    let meta: crate::profiles::FabricLaunchMeta = serde_json::from_str(&meta_content)
        .map_err(|e| format!("Invalid fabric_launch_meta.json: {}", e))?;

    let refresh_rate = detect_refresh_rate();
    sanitize_options_txt(instance_dir, refresh_rate);

    let separator = if cfg!(target_os = "windows") { ";" } else { ":" };
    let classpath = meta.classpath.join(separator);
    let natives_dir = PathBuf::from(&meta.natives_dir);
    let _ = fs::create_dir_all(&natives_dir);

    let mut cmd = Command::new(java_bin);
    cmd.current_dir(instance_dir);

    cmd.arg("-Xms512M");
    cmd.arg(format!("-Xmx{}G", ram_gb));

    let mut final_jvm_flags: Vec<String> = Vec::new();
    for flag in custom_jvm_flags.split_whitespace() {
        if !flag.is_empty() && !final_jvm_flags.iter().any(|f| f == flag) {
            final_jvm_flags.push(flag.to_string());
        }
    }

    let java_major = detect_java_major_version(java_bin);
    let has_gc_flag = final_jvm_flags
        .iter()
        .any(|f| f.starts_with("-XX:+Use") && f.ends_with("GC"));

    if !has_gc_flag {
        if java_major < 21 {
            for fallback in ["-XX:+UseG1GC", "-XX:+AlwaysPreTouch"] {
                if !final_jvm_flags.iter().any(|f| f == fallback) {
                    final_jvm_flags.push(fallback.to_string());
                }
            }
        } else {
            for fallback in ["-XX:+UseZGC", "-XX:+ZGenerational", "-XX:+AlwaysPreTouch"] {
                if !final_jvm_flags.iter().any(|f| f == fallback) {
                    final_jvm_flags.push(fallback.to_string());
                }
            }
        }
    }

    for flag in &final_jvm_flags {
        cmd.arg(flag);
    }

    cmd.arg(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
    cmd.arg(format!("-Djna.tmpdir={}", natives_dir.to_string_lossy()));
    cmd.arg(format!(
        "-Dorg.lwjgl.system.SharedLibraryExtractPath={}",
        natives_dir.to_string_lossy()
    ));
    cmd.arg(format!(
        "-Dio.netty.native.workdir={}",
        natives_dir.to_string_lossy()
    ));
    cmd.arg("-Dminecraft.launcher.brand=CreateClient");
    cmd.arg("-Dminecraft.launcher.version=1.1.0");
    cmd.arg("-Djava.net.preferIPv6Addresses=system");

    if cfg!(target_os = "macos") {
        cmd.arg("-XstartOnFirstThread");
    }

    cmd.arg("-cp").arg(&classpath);
    cmd.arg(&meta.main_class);

    let version_type = if meta.loader_version == "vanilla" {
        "release"
    } else {
        "Fabric"
    };

    cmd.arg("--username").arg(&profile.username);
    cmd.arg("--version").arg(&meta.mc_version);
    cmd.arg("--gameDir").arg(instance_dir.to_string_lossy().to_string());
    cmd.arg("--assetsDir").arg(&meta.assets_dir);
    cmd.arg("--assetIndex").arg(&meta.asset_index);
    cmd.arg("--uuid").arg(&profile.uuid);
    cmd.arg("--accessToken").arg(&profile.access_token);
    cmd.arg("--clientId").arg(crate::auth::ALLOWLISTED_CLIENT_ID);
    cmd.arg("--xuid").arg("0");
    cmd.arg("--userType").arg("msa");
    cmd.arg("--versionType").arg(version_type);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.stdin(Stdio::null());
        cmd.process_group(0);

        let current_ld = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        let new_ld = if current_ld.is_empty() {
            natives_dir.to_string_lossy().to_string()
        } else {
            format!("{}:{}", natives_dir.to_string_lossy(), current_ld)
        };
        cmd.env("LD_LIBRARY_PATH", new_ld);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);

        let current_path = std::env::var("PATH").unwrap_or_default();
        let java_bin_dir = java_bin.parent().unwrap_or(Path::new(""));
        let new_path = format!(
            "{};{};{}",
            natives_dir.to_string_lossy(),
            java_bin_dir.to_string_lossy(),
            current_path
        );
        cmd.env("PATH", new_path);
    }

    let log_file_path = instance_dir.join("launcher_log.txt");
    if let Ok(log_file) = fs::File::create(&log_file_path) {
        if let Ok(err_file) = log_file.try_clone() {
            cmd.stdout(Stdio::from(log_file));
            cmd.stderr(Stdio::from(err_file));
        }
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn Minecraft process: {}", e))?;

    let pid = child.id();
    MINECRAFT_PID.store(pid, Ordering::SeqCst);
    IS_MINECRAFT_RUNNING.store(true, Ordering::SeqCst);
    WAS_FORCE_CLOSED.store(false, Ordering::SeqCst);
    let app_handle = app.clone();
    let instance_dir_buf = instance_dir.to_path_buf();

    let profile_id_str = profile_id.to_string();
    std::thread::spawn(move || {
        let _ = app_handle.emit("game-status", true);
        let wait_res = child.wait();
        MINECRAFT_PID.store(0, Ordering::SeqCst);
        IS_MINECRAFT_RUNNING.store(false, Ordering::SeqCst);
        let _ = app_handle.emit("game-status", false);

        let was_forced = WAS_FORCE_CLOSED.swap(false, Ordering::SeqCst);
        if !was_forced {
            if let Ok(exit_status) = wait_res {
                if !exit_status.success() {
                    let diagnosis = crate::diagnostics::diagnose_instance_crash(
                        &profile_id_str,
                        &instance_dir_buf,
                        exit_status.code(),
                    );
                    let err_summary = format!(
                        "[{}] {}\n\nRoot Cause:\n{}\n\nSuggested Fix:\n{}",
                        diagnosis.issue_category,
                        diagnosis.summary,
                        diagnosis.root_cause,
                        diagnosis.suggested_fix
                    );
                    let _ = app_handle.emit("game-error", err_summary);
                    let _ = app_handle.emit("game-crash-diagnosis", diagnosis);
                }
            }
        }
    });

    Ok(pid)
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

pub fn detect_refresh_rate() -> u32 {
    #[cfg(target_os = "linux")]
    {
        // 1. Try hyprctl monitors (detect focused monitor first)
        if let Ok(output) = Command::new("hyprctl").arg("monitors").output() {
            if let Ok(text) = String::from_utf8(output.stdout) {
                let monitors: Vec<&str> = text.split("Monitor ").collect();
                for m in &monitors {
                    if m.contains("focused: yes") {
                        if let Some(pos) = m.find('@') {
                            let sub = &m[pos + 1..];
                            let end = sub.find(|c: char| !c.is_numeric() && c != '.').unwrap_or(sub.len());
                            if let Ok(rate) = sub[..end].parse::<f32>() {
                                return rate.round() as u32;
                            }
                        }
                    }
                }
                // Fallback to first monitor's rate
                for m in &monitors {
                    if let Some(pos) = m.find('@') {
                        let sub = &m[pos + 1..];
                        let end = sub.find(|c: char| !c.is_numeric() && c != '.').unwrap_or(sub.len());
                        if let Ok(rate) = sub[..end].parse::<f32>() {
                            return rate.round() as u32;
                        }
                    }
                }
            }
        }

        // 2. Try xrandr
        if let Ok(output) = Command::new("xrandr").output() {
            if let Ok(text) = String::from_utf8(output.stdout) {
                for line in text.lines() {
                    if line.contains('*') {
                        for token in line.split_whitespace() {
                            if token.ends_with('*') || token.ends_with("*+") {
                                let num = token.trim_end_matches('*').trim_end_matches('+');
                                if let Ok(rate) = num.parse::<f32>() {
                                    return rate.round() as u32;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty CurrentRefreshRate"])
            .output()
        {
            if let Ok(text) = String::from_utf8(output.stdout) {
                for line in text.lines() {
                    if let Ok(rate) = line.trim().parse::<u32>() {
                        if rate > 0 {
                            return rate;
                        }
                    }
                }
            }
        }
    }

    60
}

pub fn detect_gpu_name() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("lspci").output() {
            if let Ok(text) = String::from_utf8(output.stdout) {
                for line in text.lines() {
                    if line.contains("VGA compatible controller") || line.contains("3D controller") {
                        if let Some(pos) = line.find(": ") {
                            return line[pos + 2..].to_string();
                        }
                        return line.to_string();
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name"])
            .output()
        {
            if let Ok(text) = String::from_utf8(output.stdout) {
                for line in text.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        return trimmed.to_string();
                    }
                }
            }
        }
    }

    "Standard Graphics Device".to_string()
}

pub fn detect_gpu_tier() -> &'static str {
    let name = detect_gpu_name().to_uppercase();
    if name.contains("3070") || name.contains("3080") || name.contains("3090")
        || name.contains("4070") || name.contains("4080") || name.contains("4090")
        || name.contains("6800") || name.contains("6900") || name.contains("7800") || name.contains("7900")
        || name.contains("RX 7900") || name.contains("RX 7800")
    {
        return "ultra";
    }
    if name.contains("2060") || name.contains("2070") || name.contains("2080")
        || name.contains("3060") || name.contains("4060") || name.contains("6600")
        || name.contains("6700") || name.contains("5700") || name.contains("GTX 1080")
    {
        return "medium";
    }
    "low"
}

pub fn apply_graphics_preset(game_dir: &Path, requested_preset: &str) {
    let mode = if requested_preset.is_empty() || requested_preset == "auto" {
        detect_gpu_tier()
    } else {
        requested_preset
    };

    let preset_dir = game_dir.join("config").join("presets").join(mode);
    if preset_dir.exists() {
        // 1. Copy shaderpack config
        let preset_shader = preset_dir.join("ComplementaryUnbound_r5.9.3.zip.txt");
        let target_shader = game_dir.join("shaderpacks").join("ComplementaryUnbound_r5.9.3.zip.txt");
        if preset_shader.exists() {
            let _ = fs::copy(&preset_shader, &target_shader);
        }

        // 2. Copy Distant Horizons config
        let preset_dh = preset_dir.join("DistantHorizons.toml");
        let target_dh = game_dir.join("config").join("DistantHorizons.toml");
        if preset_dh.exists() {
            let _ = fs::copy(&preset_dh, &target_dh);
        }

        // 3. Update options.txt with preset's render distance and simulation distance
        let (rd, sd) = match mode {
            "low" => (8, 6),
            "ultra" => (14, 10),
            _ => (12, 8),
        };
        let options_file = game_dir.join("options.txt");
        if options_file.exists() {
            if let Ok(content) = fs::read_to_string(&options_file) {
                let mut new_lines = Vec::new();
                for line in content.lines() {
                    if line.starts_with("renderDistance:") {
                        new_lines.push(format!("renderDistance:{}", rd));
                    } else if line.starts_with("simulationDistance:") {
                        new_lines.push(format!("simulationDistance:{}", sd));
                    } else {
                        new_lines.push(line.to_string());
                    }
                }
                let _ = fs::write(&options_file, new_lines.join("\n") + "\n");
            }
        }
    }
}

pub fn sanitize_options_txt(game_dir: &Path, refresh_rate: u32) {
    let options_file = game_dir.join("options.txt");
    if options_file.exists() {
        if let Ok(content) = fs::read_to_string(&options_file) {
            let mut modified = false;
            let mut new_lines = Vec::new();
            let mut found_max_fps = false;

            for line in content.lines() {
                if line.starts_with("overrideWidth:") && line != "overrideWidth:0" {
                    new_lines.push("overrideWidth:0".to_string());
                    modified = true;
                } else if line.starts_with("overrideHeight:") && line != "overrideHeight:0" {
                    new_lines.push("overrideHeight:0".to_string());
                    modified = true;
                } else if line.starts_with("maxFps:") {
                    found_max_fps = true;
                    let target = format!("maxFps:{}", refresh_rate);
                    if line != target {
                        new_lines.push(target);
                        modified = true;
                    } else {
                        new_lines.push(line.to_string());
                    }
                } else {
                    new_lines.push(line.to_string());
                }
            }

            if !found_max_fps {
                new_lines.push(format!("maxFps:{}", refresh_rate));
                modified = true;
            }

            if modified {
                let _ = fs::write(&options_file, new_lines.join("\n") + "\n");
            }
        }
    }
}

