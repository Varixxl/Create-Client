import os, sys, json, requests, subprocess

TOKEN = os.environ.get('GITHUB_TOKEN', '')
if not TOKEN:
    try:
        TOKEN = subprocess.check_output(['gh', 'auth', 'token']).decode().strip()
    except Exception:
        pass

REPO = 'Varixxl/Create-Client'
TAG = 'v1.2.3'

headers = {
    'Authorization': f'token {TOKEN}',
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'CreateClient-Uploader'
}

# 1. Create release
release_payload = {
    'tag_name': TAG,
    'target_commitish': 'main',
    'name': f'Create Client {TAG} - Removable Stock Profiles & Strict Mod Compatibility',
    'body': """## What's Changed in Create Client v1.2.3

### 🗑️ Removable Stock Profiles & Customization
- **Delete Built-in Profiles:** Users can now delete any profile including built-in stock profiles (`Create Server` and `Fabric Performance & QoL`).
- **Persistent Profile Management:** Launcher tracks deleted built-in profile IDs across app restarts so deleted stock profiles are not resurrected.
- **Editable Names & Descriptions:** Stock profiles can now have their names and descriptions fully edited in the profile settings modal.

### 🛡️ Strict Mod Compatibility Enforcement
- **Eliminated Incompatible Fallbacks:** Removed legacy fallbacks in Modrinth and CurseForge installation logic that previously downloaded incompatible jars when no exact version match was found.
- **Strict Minecraft & Loader Checks:** Mod installations strictly enforce exact Minecraft version matching (`resolved_mc_version` / `mc_version`) and loader compatibility (`Forge`, `Fabric`, `Quilt`, `NeoForge`).
- **Clear Actionable Error Messages:** Attempting to install an incompatible mod returns an explicit, user-friendly error explaining why the mod cannot be installed on the selected profile.
- **Vanilla Profile Guard:** Mod installs are prohibited on Vanilla Minecraft profiles, with an in-UI warning banner and disabled installation buttons.
- **Active Profile Scope Badge:** The Mods & Content manager displays an active profile scope indicator badge (`Scope: [Profile Name] | [LOADER] [MC Version]`) showing exactly what Minecraft environment mods are being filtered against.""",
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
    ('Create-Client-1.2.3-windows-x64.zip', 'Create-Client-1.2.3-windows-x64.zip', 'application/zip'),
    ('create-client', 'create-client', 'application/octet-stream'),
    ('create-client-1.2.3-linux-x86_64.tar.gz', 'create-client-1.2.3-linux-x86_64.tar.gz', 'application/gzip')
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
