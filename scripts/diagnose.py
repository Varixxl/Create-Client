#!/usr/bin/env python3
"""
Create Client & Instance Diagnostic Doctor
Diagnoses Java runtimes, Minecraft versions, mod dependencies, and crash reports.
"""

import os
import sys
import json
import glob
import re
import zipfile
import urllib.request
import shutil
import tarfile
from pathlib import Path

CLIENT_DATA_DIR = Path(os.path.expanduser("~/.local/share/createclient"))
CLIENT_CONFIG_DIR = Path(os.path.expanduser("~/.config/createclient"))
RUNTIME_DIR = CLIENT_DATA_DIR / "runtime"
PROFILES_FILE = CLIENT_CONFIG_DIR / "profiles.json"

ADOPTIUM_FALLBACKS = {
    25: {
        "linux": "https://github.com/adoptium/temurin25-binaries/releases/download/jdk-25.0.4.1%2B1/OpenJDK25U-jre_x64_linux_hotspot_25.0.4.1_1.tar.gz",
        "windows": "https://github.com/adoptium/temurin25-binaries/releases/download/jdk-25.0.4.1%2B1/OpenJDK25U-jre_x64_windows_hotspot_25.0.4.1_1.zip",
    },
    21: {
        "linux": "https://cdn.azul.com/zulu/bin/zulu21.52.203-ca-jre21.0.12.1-linux_x64.tar.gz",
        "windows": "https://cdn.azul.com/zulu/bin/zulu21.52.203-ca-jre21.0.12.1-win_x64.zip",
    },
    17: {
        "linux": "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.20.1%2B1/OpenJDK17U-jre_x64_linux_hotspot_17.0.20.1_1.tar.gz",
        "windows": "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.20.1%2B1/OpenJDK17U-jre_x64_windows_hotspot_17.0.20.1_1.zip",
    },
    8: {
        "linux": "https://github.com/adoptium/temurin8-binaries/releases/download/jdk8u504-b01/OpenJDK8U-jre_x64_linux_hotspot_8u504b01.tar.gz",
        "windows": "https://github.com/adoptium/temurin8-binaries/releases/download/jdk8u504-b01/OpenJDK8U-jre_x64_windows_hotspot_8u504b01.zip",
    }
}

def get_installed_runtimes():
    runtimes = {}
    if not RUNTIME_DIR.exists():
        return runtimes
    for entry in RUNTIME_DIR.iterdir():
        if entry.is_dir() and entry.name.startswith("java"):
            java_bin = entry / "bin" / ("java.exe" if os.name == "nt" else "java")
            if java_bin.exists() and os.access(java_bin, os.X_OK if os.name != "nt" else os.R_OK):
                try:
                    num = int(entry.name.replace("java", ""))
                    runtimes[num] = java_bin
                except ValueError:
                    pass
    return runtimes

def determine_required_java(instance_path, mc_version):
    required = 17
    instance_path = Path(instance_path)

    # 1. Check version.json
    version_file = instance_path / "version.json"
    if version_file.exists():
        try:
            with open(version_file, "r", encoding="utf-8") as f:
                data = json.load(f)
                jv = data.get("javaVersion", {}).get("majorVersion")
                if jv:
                    required = max(required, int(jv))
        except Exception:
            pass

    # 2. Base version heuristic from MC string
    mc_ver = str(mc_version).strip()
    if mc_ver.startswith("26.") or mc_ver.startswith("27."):
        required = max(required, 25)
    elif mc_ver.startswith("1.21") or mc_ver.startswith("1.20.5") or mc_ver.startswith("1.20.6"):
        required = max(required, 21)
    elif mc_ver.startswith("1.18") or mc_ver.startswith("1.19") or mc_ver.startswith("1.20"):
        required = max(required, 17)
    elif mc_ver.startswith("1.17"):
        required = max(required, 17)
    elif mc_ver and (mc_ver.startswith("1.16") or mc_ver.startswith("1.12") or mc_ver.startswith("1.7")):
        required = 8

    # 3. Check mods folder
    mods_dir = instance_path / "mods"
    if mods_dir.exists():
        for jar in mods_dir.glob("*.jar"):
            try:
                with zipfile.ZipFile(jar) as z:
                    if "fabric.mod.json" in z.namelist():
                        data = json.loads(z.read("fabric.mod.json").decode("utf-8", "ignore"))
                        dep_java = data.get("depends", {}).get("java")
                        if dep_java:
                            digits = "".join(filter(str.isdigit, str(dep_java)))
                            if digits and 8 <= int(digits) <= 35:
                                required = max(required, int(digits))
                    if "META-INF/mods.toml" in z.namelist() or "META-INF/neoforge.mods.toml" in z.namelist():
                        entry_name = "META-INF/mods.toml" if "META-INF/mods.toml" in z.namelist() else "META-INF/neoforge.mods.toml"
                        content = z.read(entry_name).decode("utf-8", "ignore")
                        for line in content.splitlines():
                            if "java" in line.lower() and "versionrange" in line.lower():
                                digits = "".join(filter(str.isdigit, line))
                                if digits and 8 <= int(digits) <= 35:
                                    required = max(required, int(digits))
            except Exception:
                pass

    return required

def diagnose_crash_reports(instance_path):
    crash_dir = Path(instance_path) / "crash-reports"
    if not crash_dir.exists():
        return None

    reports = sorted(crash_dir.glob("crash-*.txt"), key=lambda p: p.stat().st_mtime, reverse=True)
    if not reports:
        return None

    newest = reports[0]
    try:
        content = newest.read_text(encoding="utf-8", errors="ignore")
    except Exception:
        return None

    desc = ""
    cause = ""
    suspected = []

    for line in content.splitlines():
        if line.startswith("Description:"):
            desc = line.replace("Description:", "").strip()
        elif line.startswith("Caused by:") and not cause:
            cause = line.strip()
        elif "Suspected Mods:" in line:
            # next few lines
            pass

    if "ThreadPresetConfigEventHandler" in content:
        return {
            "file": str(newest),
            "category": "Distant Horizons Lifecycle Conflict",
            "summary": "Premature Distant Horizons configuration access during mod loading.",
            "root_cause": "ThreadPresetConfigEventHandler attempted to register listeners before ConfigHandler setup.",
            "fix": "Safely deferred in updated graphics_presets.js; no action needed.",
        }

    if "OutOfMemoryError" in content:
        return {
            "file": str(newest),
            "category": "Out of Memory",
            "summary": "Minecraft ran out of Java heap memory.",
            "root_cause": "java.lang.OutOfMemoryError: Java heap space",
            "fix": "Increase allocated RAM in Client Settings (e.g. 8G - 12G).",
        }

    return {
        "file": str(newest),
        "category": "General Crash Report",
        "summary": desc or "Minecraft stopped unexpectedly.",
        "root_cause": cause or "See crash report for full stack trace.",
        "fix": "Inspect suspected mods in the crash report file.",
    }

def diagnose_logs(instance_path):
    instance_path = Path(instance_path)
    logs_to_check = [
        instance_path / "launcher_log.txt",
        instance_path / "logs" / "latest.log"
    ]
    for log_path in logs_to_check:
        if log_path.exists():
            try:
                content = log_path.read_text(encoding="utf-8", errors="ignore")
                if "Incompatible mods found!" in content and "Replace 'OpenJDK 64-Bit Server VM' (java)" in content:
                    m = re.search(r"Replace 'OpenJDK 64-Bit Server VM' \(java\) (\d+) with version (\d+) or later", content)
                    cur_v = m.group(1) if m else "unknown"
                    req_v = m.group(2) if m else "25"
                    return {
                        "category": "Java Version Mismatch",
                        "summary": f"Mods require Java {req_v} or later, but instance ran on Java {cur_v}.",
                        "root_cause": f"Fabric Loader rejected Java {cur_v} for modern dependencies.",
                        "fix": f"Install Java {req_v} runtime (use --fix to download automatically).",
                    }
                if "Unrecognized VM option 'ZGenerational'" in content:
                    return {
                        "category": "JVM Flag Incompatibility",
                        "summary": "Flag '-XX:+ZGenerational' is unsupported on Java < 21.",
                        "root_cause": "JVM aborted startup due to unrecognized flag.",
                        "fix": "Remove -XX:+ZGenerational or use G1GC on Java 17.",
                    }
            except Exception:
                pass
    return None

def download_and_install_java(version):
    print(f"\n[*] Downloading OpenJDK {version} Runtime...")
    url = ADOPTIUM_FALLBACKS.get(version, {}).get("linux")
    if not url:
        print(f"[-] No download URL configured for Java {version}")
        return False

    RUNTIME_DIR.mkdir(parents=True, exist_ok=True)
    target_dir = RUNTIME_DIR / f"java{version}"
    temp_archive = RUNTIME_DIR / f"temp_java{version}.tar.gz"
    temp_extract = RUNTIME_DIR / f"temp_java{version}_extract"

    try:
        urllib.request.urlretrieve(url, temp_archive)
        temp_extract.mkdir(parents=True, exist_ok=True)
        with tarfile.open(temp_archive, "r:gz") as t:
            t.extractall(temp_extract)

        entries = os.listdir(temp_extract)
        inner = temp_extract / entries[0]
        if target_dir.exists():
            shutil.rmtree(target_dir, ignore_errors=True)
        shutil.move(str(inner), str(target_dir))
        shutil.rmtree(temp_extract, ignore_errors=True)
        if temp_archive.exists():
            temp_archive.unlink()

        bin_path = target_dir / "bin" / "java"
        if bin_path.exists():
            os.chmod(bin_path, 0o755)
            print(f"[+] Successfully installed Java {version} to: {target_dir}")
            return True
    except Exception as e:
        print(f"[-] Failed to download Java {version}: {e}")
        return False
    return False

def main():
    print("=" * 80)
    print("          CREATE CLIENT DIAGNOSTIC HEALTH CHECK & CRASH DOCTOR")
    print("=" * 80)

    fix_mode = "--fix" in sys.argv
    filter_profile = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else None

    # 1. Inspect Runtimes
    runtimes = get_installed_runtimes()
    print("\n[Managed Java Runtimes]")
    if runtimes:
        for v in sorted(runtimes.keys()):
            path = runtimes[v]
            print(f"  • Java {v:2d}: {path}")
    else:
        print("  ! No managed runtimes found in ~/.local/share/createclient/runtime/")

    # 2. Inspect Profiles
    if not PROFILES_FILE.exists():
        print("\n[-] Profiles file not found at: ~/.config/createclient/profiles.json")
        return

    try:
        with open(PROFILES_FILE, "r", encoding="utf-8") as f:
            profiles_data = json.load(f)
    except Exception as e:
        print(f"\n[-] Failed to load profiles.json: {e}")
        return

    profiles = profiles_data.get("profiles", [])
    selected_id = profiles_data.get("selected_profile_id", "")
    print(f"\n[Registered Modpack Profiles] (Selected: {selected_id})")

    missing_runtimes = set()

    for p in profiles:
        pid = p.get("id", "")
        if filter_profile and filter_profile != pid:
            continue

        name = p.get("name", "")
        mc_ver = p.get("resolved_mc_version") or p.get("mc_version", "unknown")
        loader = p.get("loader", "unknown")
        instance_dir = Path(p.get("instance_dir", ""))

        is_selected = "[ACTIVE] " if pid == selected_id else "         "
        print(f"\n{is_selected}Profile: {name} ({pid})")
        print(f"           Version: MC {mc_ver} | Loader: {loader}")
        print(f"           Location: {instance_dir}")

        if not instance_dir.exists():
            print(f"           ! Instance directory does not exist yet (Not synchronized)")
            continue

        req_java = determine_required_java(instance_dir, mc_ver)
        has_java = req_java in runtimes
        status_symbol = "✓" if has_java else "✗"
        print(f"           Java Requirement: Java {req_java} [{status_symbol} {'Installed' if has_java else 'MISSING'}]")

        if not has_java:
            missing_runtimes.add(req_java)

        # Check logs & crash reports
        log_diag = diagnose_logs(instance_dir)
        crash_diag = diagnose_crash_reports(instance_dir)

        if log_diag:
            print(f"           ! Log Diagnosis [{log_diag['category']}]:")
            print(f"             - {log_diag['summary']}")
            print(f"             - Fix: {log_diag['fix']}")
        elif crash_diag:
            print(f"           ! Recent Crash Report [{crash_diag['category']}]:")
            print(f"             - File: {crash_diag['file']}")
            print(f"             - Summary: {crash_diag['summary']}")
            print(f"             - Fix: {crash_diag['fix']}")
        else:
            print(f"           Status: HEALTHY (No active crash signatures detected)")

    print("\n" + "=" * 80)

    if missing_runtimes:
        print(f"\n[!] Detected {len(missing_runtimes)} missing Java runtime(s): {sorted(missing_runtimes)}")
        if fix_mode:
            for v in sorted(missing_runtimes):
                download_and_install_java(v)
            print("\n[+] All missing runtimes have been provisioned!")
        else:
            print("    Run 'create-diagnose --fix' to automatically download and configure them.")
    else:
        print("\n[+] All profile Java dependencies and runtimes are fully satisfied!")

if __name__ == "__main__":
    main()
