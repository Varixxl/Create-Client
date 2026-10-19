import os, sys, json, requests, subprocess

TOKEN = os.environ.get('GITHUB_TOKEN', '')
if not TOKEN:
    try:
        TOKEN = subprocess.check_output(['gh', 'auth', 'token']).decode().strip()
    except Exception:
        pass

REPO = 'Varixxl/Create-Client'
TAG = 'v1.2.2'

headers = {
    'Authorization': f'token {TOKEN}',
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'CreateClient-Uploader'
}

# 1. Create release
release_payload = {
    'tag_name': TAG,
    'target_commitish': 'main',
    'name': f'Create Client {TAG} - Obsidian Custom Dropdowns & Dark Theme Fix',
    'body': """## What's Changed in Create Client v1.2.2

### 🎨 Obsidian Custom Dropdown Menus & Dark Theme Visibility Fix
- **Eliminated White Dropdown Lists:** Resolved OS and WebKit/WebView2 default light styling where dropdown menus rendered with white backgrounds and invisible white text.
- **Custom-Rendered Dropdowns:** Replaced all native `<select>` elements across Profile Settings and Edit Modals with custom React `CustomDropdown` components styled to match the obsidian (`#0e131d`) and cyan (`#12c3ca`) theme.
- **Instant Version Search:** Integrated a real-time search filter for the Minecraft version selector, allowing instant filtering of 50+ release and snapshot versions.
- **Rich Loader & Java Badges:** Enhanced loader and Java runtime options with recommended star badges (`★ Recommended`), runtime descriptions, and GC tags (`G1GC Optimized`, `ZGC Ready`).
- **Enforced Global Dark Scheme:** Configured `color-scheme: dark` across `index.html`, global CSS, and Tauri window configuration to ensure all UI elements and browser popups render dark.""",
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
    ('Create-Client-1.2.2-windows-x64.zip', 'Create-Client-1.2.2-windows-x64.zip', 'application/zip'),
    ('create-client', 'create-client', 'application/octet-stream'),
    ('create-client-1.2.2-linux-x86_64.tar.gz', 'create-client-1.2.2-linux-x86_64.tar.gz', 'application/gzip')
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
