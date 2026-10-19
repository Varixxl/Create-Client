import os, sys, json, requests, subprocess

TOKEN = os.environ.get('GITHUB_TOKEN', '')
if not TOKEN:
    try:
        TOKEN = subprocess.check_output(['gh', 'auth', 'token']).decode().strip()
    except Exception:
        pass

REPO = 'Varixxl/Create-Client'
TAG = 'v1.2.1'

headers = {
    'Authorization': f'token {TOKEN}',
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'CreateClient-Uploader'
}

# 1. Create release
release_payload = {
    'tag_name': TAG,
    'target_commitish': 'main',
    'name': f'Create Client {TAG} - Per-Profile Java Manager, JVM Sanitization & Crash Diagnostics',
    'body': """## What's Changed in Create Client v1.2.1

### ☕ Per-Profile Java Manager & Automatic Version Compatibility
- **Per-Profile Java Versioning:** Assign a specific Java version (Java 8, 11, 16, 17, 21, or Auto-Detect) directly on individual profiles via the Edit Profile settings.
- **Strict Compatibility Enforcement:** Automatically detects and enforces runtime compatibility requirements for specific Minecraft and modloader combinations (e.g., enforcing Java 17 for Forge 1.20.1).
- **Multi-Version Runtime Provisioning:** Seamless background provisioning for Adoptium Temurin OpenJDK across platforms without manual Java environment setup.

### 🛡️ Smart JVM Argument Sanitization & VM Error Prevention
- **Eliminated JVM Startup Failures:** Resolved `Could not create the Java Virtual Machine: Unrecognized VM option 'ZGenerational'` crashes.
- **Dynamic Garbage Collector Fallback:** Inspects target Java version before launch; automatically removes JDK 21+ only flags like `-XX:+ZGenerational` and cleanly falls back to `-XX:+UseG1GC` on Java < 21.

### 🔍 Enhanced Crash Diagnostics & Real-Time Troubleshooting
- **Automated Exit Code & Stderr Inspection:** Inspects exit codes and standard error output to detect JVM misconfigurations, memory errors, or missing dependencies.
- **Actionable Error Feedback:** Shows instant, readable crash diagnosis modals with direct pointers to the root cause.

### 🎨 UI Polish & Toast Improvements
- **Bottom-Right Toast Positioning:** Moved toast notifications to the bottom-right corner to prevent obstructing navigation bars and action buttons.
- **Faded Panorama Background:** Added subtle panorama styling to enhance launcher aesthetics.
- **Scoped Server Status Badges:** Improved server status indicator scoping for multi-profile workflows.""",
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
    ('Create-Client-1.2.1-windows-x64.zip', 'Create-Client-1.2.1-windows-x64.zip', 'application/zip'),
    ('create-client', 'create-client', 'application/octet-stream'),
    ('create-client-1.2.1-linux-x86_64.tar.gz', 'create-client-1.2.1-linux-x86_64.tar.gz', 'application/gzip')
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
