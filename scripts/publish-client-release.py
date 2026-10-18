import os, sys, json, requests

TOKEN = os.environ.get('GITHUB_TOKEN', '')
REPO = 'JaviDaBomb/Create-Client'
TAG = 'v1.0.9'

headers = {
    'Authorization': f'token {TOKEN}',
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'CreateClient-Uploader'
}

# 1. Create release
release_payload = {
    'tag_name': TAG,
    'target_commitish': 'main',
    'name': f'Create Client {TAG} - Persistent Microsoft Session Auto-Refresh & Graphics Presets',
    'body': """## What's Changed in Create Client v1.0.9

### 🔐 Persistent Microsoft & Minecraft Session Auto-Refresh
- **Eliminated 24-Hour Re-Login Requirement:** Fixed an OAuth Client ID mismatch during background token refresh so saved sessions renew seamlessly without requiring manual sign-out and sign-in every 24 hours.
- **Automatic Pre-Launch Token Renewal:** Every game launch now proactively refreshes your Minecraft access token, guaranteeing a full 24-hour session validity window whenever you click **Play Game**.
- **Continuous Background Refresh & Retry Resilience:** Added automatic 30-minute background session checks while the launcher is open along with multi-attempt retry logic for transient network/DNS hiccups.

### ⚡ Graphics Presets Engine & Display Refresh Rate Lock
- **Low, Medium, and Ultra Presets:** Select your graphics tier directly in Client Settings or let Auto Tier detect your GPU capabilities.
- **Native Refresh Rate Sync:** Automatically detects your monitor's refresh rate and locks your frame cap to it for tear-free performance.""",
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
    ('Create-Client-1.0.9-windows-x64.zip', 'Create-Client-1.0.9-windows-x64.zip', 'application/zip'),
    ('create-client', 'create-client', 'application/octet-stream'),
    ('create-client-1.0.9-linux-x86_64.tar.gz', 'create-client-1.0.9-linux-x86_64.tar.gz', 'application/gzip')
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
