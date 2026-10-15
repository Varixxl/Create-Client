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
}

pub fn launch_minecraft(
    java_bin: &Path,
    game_dir: &Path,
    profile: &MinecraftProfile,
    ram_gb: u32,
    custom_jvm_flags: &str,
    auto_connect: bool,
) -> Result<u32, String> {
    if !java_bin.exists() {
        return Err(format!("Java executable not found at: {:?}", java_bin));
    }
    if !game_dir.exists() {
        return Err(format!("Game directory not found at: {:?}", game_dir));
    }

    let lib_dir = game_dir.join("libraries");
    let separator = if cfg!(target_os = "windows") { ";" } else { ":" };

    // 1. Gather all classpath libraries
    let mut classpath_entries: Vec<PathBuf> = Vec::new();
    if lib_dir.exists() {
        collect_jars(&lib_dir, &mut classpath_entries);
    }

    // Add vanilla client jar if present
    let client_jar = game_dir.join("client.jar");
    if client_jar.exists() {
        classpath_entries.push(client_jar);
    } else {
        let v_jar = game_dir.join("1.20.1.jar");
        if v_jar.exists() {
            classpath_entries.push(v_jar);
        }
    }

    let classpath = classpath_entries
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(separator);

    // 2. Module Path (Required for Java 17 Forge 1.20.1)
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
    cmd.arg("-Xms2G");
    cmd.arg(format!("-Xmx{}G", ram_gb));

    // Custom JVM flags
    for flag in custom_jvm_flags.split_whitespace() {
        if !flag.is_empty() {
            cmd.arg(flag);
        }
    }

    // Standard JVM Properties
    cmd.arg(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
    cmd.arg("-Dminecraft.launcher.brand=CreateClient");
    cmd.arg("-Dminecraft.launcher.version=0.1.0");
    cmd.arg("-Djava.net.preferIPv6Addresses=system");
    cmd.arg("-DignoreList=bootstraplauncher,securejarhandler,asm-commons,asm-util,asm-analysis,asm-tree,asm,JarJarFileSystems,client-extra,fmlcore,javafmllanguage,lowcodelanguage,mclanguage,forge-,1.20.1.jar");
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
    cmd.arg("--clientId").arg("43e4ff18-9c63-455f-9224-a124a6b71086");
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
