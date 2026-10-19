import os, sys, json, requests, subprocess

TOKEN = os.environ.get('GITHUB_TOKEN', '')
if not TOKEN:
    try:
        TOKEN = subprocess.check_output(['gh', 'auth', 'token']).decode().strip()
    except Exception:
        pass

REPO = 'Varixxl/Create-Client'
TAG = 'v1.2.5'

headers = {
    'Authorization': f'token {TOKEN}',
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'CreateClient-Uploader'
}

# 1. Create release
release_payload = {
    'tag_name': TAG,
    'target_commitish': 'main',
    'name': f'Create Client {TAG} - One-Click Authorization Refresh & In-Game Auth Support',
    'body': """## What's Changed in Create Client v1.2.5

### 🔄 One-Click Microsoft & Minecraft Session Refresh
- **Header Profile Refresh Button:** Added a dedicated refresh button (`RefreshCw`) beside the user profile avatar in the launcher header to immediately re-authenticate with Microsoft, Xbox Live, and Minecraft Services without having to sign out or reopen browser login flows.
- **Settings Troubleshooting Action:** Integrated a manual 'Refresh Authorization' troubleshooting button inside Global Settings -> Troubleshooting & Integrity to verify and renew active sessions.
- **Real-Time Visual Feedback:** Status badge displays 'Refreshing...' and spins during renewal, showing immediate toast feedback ('Authorization refreshed successfully! Ready to play') or actionable error reporting if Microsoft credentials have expired.
- **Backend Token Renewal Engine:** Added the `refresh_auth` Tauri command and `force_refresh_profile` Rust service, refreshing Microsoft OAuth tokens, renewing XSTS claims, and saving the updated session.""",
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
    ('Create-Client-1.2.5-windows-x64.zip', 'Create-Client-1.2.5-windows-x64.zip', 'application/zip'),
    ('create-client', 'create-client', 'application/octet-stream'),
    ('create-client-1.2.5-linux-x86_64.tar.gz', 'create-client-1.2.5-linux-x86_64.tar.gz', 'application/gzip')
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
