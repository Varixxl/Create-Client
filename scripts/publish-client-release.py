import os, sys, json, requests, subprocess

TOKEN = os.environ.get('GITHUB_TOKEN', '')
if not TOKEN:
    try:
        TOKEN = subprocess.check_output(['gh', 'auth', 'token']).decode().strip()
    except Exception:
        pass

REPO = 'Varixxl/Create-Client'
TAG = 'v1.2.4'

headers = {
    'Authorization': f'token {TOKEN}',
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'CreateClient-Uploader'
}

# 1. Create release
release_payload = {
    'tag_name': TAG,
    'target_commitish': 'main',
    'name': f'Create Client {TAG} - UI Performance Overhaul & Input Latency Fix',
    'body': """## What's Changed in Create Client v1.2.4

### ⚡ UI Performance Overhaul & Instant Input Responsiveness
- **Eliminated Text Field & Modal Lag:** Resolved severe input lag when typing into profile names, descriptions, and search fields by eliminating expensive software backdrop-filter blurs and full-tree re-renders.
- **Intelligent Background Animation Control:** The continuous 165 FPS background panorama animation now automatically pauses whenever any modal, drawer, or dialog is active, or when the launcher window loses focus—dropping idle CPU/GPU usage to near-zero.
- **Removed Heavy CSS Filters:** Eliminated all `backdrop-blur-*` and large 64px Gaussian `blur-3xl` layers, replacing them with high-performance solid obsidian panels (`#0a0d13` / `#0e121a`), crisp 1px borders, and drop shadows.
- **React Component & List Memoization:** Wrapped `CustomDropdown`, `ProfileModal`, `ModsPanel`, `SettingsModal`, and `CreateServerModal` in `React.memo` and memoized version mapping arrays (`useMemo`), preventing unneeded DOM reconciliation on every keystroke.
- **Linux NVIDIA Wayland Driver Guard:** Automatically sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` in Rust `main.rs` on Linux, resolving known NVIDIA proprietary driver frame pacing stutters and input latency under Wayland/Hyprland.""",
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
    ('Create-Client-1.2.4-windows-x64.zip', 'Create-Client-1.2.4-windows-x64.zip', 'application/zip'),
    ('create-client', 'create-client', 'application/octet-stream'),
    ('create-client-1.2.4-linux-x86_64.tar.gz', 'create-client-1.2.4-linux-x86_64.tar.gz', 'application/gzip')
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
