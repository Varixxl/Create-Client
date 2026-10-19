import os, sys, json, requests, subprocess

TOKEN = os.environ.get('GITHUB_TOKEN', '')
if not TOKEN:
    try:
        TOKEN = subprocess.check_output(['gh', 'auth', 'token']).decode().strip()
    except Exception:
        pass

REPO = 'Varixxl/Create-Client'
TAG = 'v1.2.0'

headers = {
    'Authorization': f'token {TOKEN}',
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'CreateClient-Uploader'
}

# 1. Create release
release_payload = {
    'tag_name': TAG,
    'target_commitish': 'main',
    'name': f'Create Client {TAG} - Content Manager, Dynamic Loaders & Dedicated Server Creator',
    'body': """## What's Changed in Create Client v1.2.0

### 🧩 Per-Profile Content Manager (Modrinth & CurseForge)
- **Multi-Source Content Discovery:** Search and install content directly from **Modrinth** and **CurseForge** with a unified, high-speed interface.
- **Triple Content Support:** Native management for **Mods**, **Resource Packs**, and **Shaders** with isolated directories per instance (`mods/`, `resourcepacks/`, `shaderpacks/`).
- **User-Configured CurseForge API Key:** Simple, secure integration for personal CurseForge API keys configurable directly in Settings.
- **Fixed Image & Icon Rendering:** Resolved WebView CSP restrictions for Modrinth CDN and CurseForge avatars, complete with graceful fallback avatars.

### ⚡ Dynamic Modloader & Version Engine
- **Universal Loader Support:** Create profiles with **Fabric**, **Quilt**, **Forge**, **NeoForge**, or clean **Vanilla**.
- **Real-Time Loader Metadata Resolution:** Queries official loader metadata APIs to fetch recommended and stable loader builds automatically.
- **Snapshot & Pre-Release Support:** 1-click toggle to browse and install Minecraft snapshots and pre-releases alongside stable releases.

### 🖥️ Pre-Installed Dedicated Server Creator (Beta)
- **1-Click Dedicated Server Generation:** Generate a ready-to-run dedicated server instance from *any* profile directly in the client.
- **Smart Client-Only Mod Filtering:** Automatic multi-tier filtering (file heuristics, `fabric.mod.json`, and `META-INF/mods.toml` inspection) removes client-only rendering engines, HUDs, FancyMenu, minimaps, and shaders so the server starts cleanly without crashing.
- **Ready to Run:** Pre-accepts `eula.txt`, generates optimized start scripts (`run.sh` / `run.bat`), pre-configures `server.properties`, and tracks server manifest.

### 📊 Persistent Sidebar Download Progress
- **Consistent Progress Tracking:** Download progress bar and status text placed permanently at the bottom of the left sidebar (`ProfileSidebar`), displaying active tasks, progress percentage, current/total size, and real-time transfer speeds in MB/s.

### 🌐 Scoped Server Management Tab (`[Servers (N)]`)
- **Profile-Scoped Visibility:** Dedicated `[Servers (N)]` workspace tab conditionally appears only on profiles that have created server instances.
- **Live Server List Ping (SLP):** Real-time server status polling showing Online/Offline status, ping (ms), live online player counts, max capacity, MOTD, and version info.
- **1-Click Folder Access:** Instant access to open server directories in your system file manager.

### 🔄 Profile Change Detection & Divergence Sync
- **Automated Divergence Tracking:** Automatically compares profile mods against server mods to detect when mods have been added or removed in the profile.
- **1-Click Server Sync:** Prominently alerts you when divergence occurs with a 1-click action to sync added and removed mods to the dedicated server instance.""",
    'draft': False,
    'prerelease': False
}

resp = requests.post(f'https://api.github.com/repos/{REPO}/releases', headers=headers, json=release_payload)
if resp.status_code == 201:
    release_data = resp.json()
    print(f"Created release ID {release_data['id']}")
else:
    print(f"Status {resp.status_code}, checking existing release: {resp.text}")
    resp = requests.get(f'https://api.github.com/repos/{REPO}/releases/tags/{TAG}', headers=headers)
    resp.raise_for_status()
    release_data = resp.json()

upload_url_base = release_data['upload_url'].split('{')[0]

assets = [
    ('Create-Client.exe', 'Create-Client.exe', 'application/vnd.microsoft.portable-executable'),
    ('Create-Client-1.2.0-windows-x64.zip', 'Create-Client-1.2.0-windows-x64.zip', 'application/zip'),
    ('create-client', 'create-client', 'application/octet-stream'),
    ('create-client-1.2.0-linux-x86_64.tar.gz', 'create-client-1.2.0-linux-x86_64.tar.gz', 'application/gzip')
]

for file_path, asset_name, content_type in assets:
    full_path = os.path.join('/home/varixxl/create-client', file_path)
    file_size = os.path.getsize(full_path)
    print(f'Uploading {asset_name} ({file_size} bytes)...')
    with open(full_path, 'rb') as f:
        file_bytes = f.read()

    upload_headers = {
        'Authorization': f'token {TOKEN}',
        'Content-Type': content_type,
        'User-Agent': 'CreateClient-Uploader'
    }
    r = requests.post(f'{upload_url_base}?name={asset_name}', headers=upload_headers, data=file_bytes)
    r.raise_for_status()
    print(f'Uploaded {asset_name}: {r.status_code}')

print('All assets uploaded successfully!')
