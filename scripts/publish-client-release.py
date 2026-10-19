import os, sys, json, requests, subprocess

TOKEN = os.environ.get('GITHUB_TOKEN', '')
if not TOKEN:
    try:
        TOKEN = subprocess.check_output(['gh', 'auth', 'token']).decode().strip()
    except Exception:
        pass

REPO = 'Varixxl/Create-Client'
TAG = 'v1.1.0'

headers = {
    'Authorization': f'token {TOKEN}',
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'CreateClient-Uploader'
}

# 1. Create release
release_payload = {
    'tag_name': TAG,
    'target_commitish': 'main',
    'name': f'Create Client {TAG} - Multi-Modpack Profiles, Mod Manager & Live Game Console',
    'body': """## What's Changed in Create Client v1.1.0

### 📦 Multi-Instance Modpack Profiles & Profile Rail
- **Custom Modpack Instances:** Create, select, duplicate, edit JVM/RAM overrides, and delete isolated Minecraft profiles directly in Create Client.
- **Dedicated Directory Isolation:** Each instance maintains its own isolated `mods/`, `config/`, `saves/`, and `options.txt` sandbox.
- **Pinned Create Server Instance:** The official Create Server profile is permanently pinned and guarded with automated differential delta patch updates.

### ⚡ Built-in Fabric Performance & QoL Pack
- **Instant Modern Fabric Setup:** Launch clean, high-performance vanilla/Fabric gameplay dynamically linked to the latest stable Minecraft and Fabric Loader versions.
- **12 Curated Optimization Mods:** Automatically downloads and synchronizes top-tier optimization & quality-of-life mods (`Sodium`, `Lithium`, `FerriteCore`, `ImmediatelyFast`, `EntityCulling`, `Mod Menu`, `Cloth Config`, `Jade`, `AppleSkin`, `Xaero's Minimap`, `Zoomify`, `Fabric API`) via Modrinth's API.

### 🧩 Per-Profile Mod Manager & Modrinth Browser
- **Direct Mod Control:** View installed mods, toggle `.jar` / `.jar.disabled` states without closing the launcher, and safely delete mods.
- **Modrinth Search & 1-Click Install:** Browse thousands of compatible Fabric and Forge mods directly inside Create Client with automatic dependency resolution.

### 🖥️ Live Game Console & Crash Inspector
- **Real-Time Stream:** Inspect game output and log events live while Minecraft is running with ALL, WARN+, and ERROR filters.
- **Crash Detection & Diagnostics:** Automatically scans for crash reports and stacktraces with 1-click Copy Log.

### 🛡️ Security Hardening & Platform Integrity
- **Frontend CSP:** Enforced strict Content Security Policy for embedded WebViews.
- **HTTPS & Binary Verification:** Validates GitHub release endpoints, verifies ELF and PE executable headers, and unpacks Linux tarballs safely.""",
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
    ('Create-Client-1.1.0-windows-x64.zip', 'Create-Client-1.1.0-windows-x64.zip', 'application/zip'),
    ('create-client', 'create-client', 'application/octet-stream'),
    ('create-client-1.1.0-linux-x86_64.tar.gz', 'create-client-1.1.0-linux-x86_64.tar.gz', 'application/gzip')
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
