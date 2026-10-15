import os, sys, json, requests

TOKEN = os.environ.get('GITHUB_TOKEN', '')
REPO = 'Varixxl/Create-Client'
TAG = 'v1.0.8'

headers = {
    'Authorization': f'token {TOKEN}',
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'CreateClient-Uploader'
}

# 1. Create release
release_payload = {
    'tag_name': TAG,
    'target_commitish': 'main',
    'name': f'Create Client {TAG} - Windows OpenAL & Native Library Fixes',
    'body': """## What's Changed in Create Client v1.0.7

### 🎧 Critical OpenAL Audio & Video Sync Fixes for Windows
- **Native Architecture Filtering:** Accurately filtered out 32-bit x86 and ARM native jars (`natives-windows-x86.jar`, `natives-windows-arm64.jar`) from the 64-bit Java runtime classpath, preventing library collisions.
- **Process PATH Injection:** Explicitly prepended `<game_dir>/natives` and the JRE runtime `bin/` directory to the Windows child process `PATH` environment variable, enabling dynamic linking resolution for `OpenAL.dll` and its dependent helper libraries.
- **Shared Library Extract Paths:** Configured `-Dorg.lwjgl.system.SharedLibraryExtractPath`, `-Djna.tmpdir`, and `-Dio.netty.native.workdir` JVM properties so LWJGL 3 locates OpenAL and GLFW native libraries directly in the native folder.
- **In-Game Sound & Video Restoration:** Resolved silent block interaction acoustics (Sound Physics Remastered occlusion calculations) and FancyMenu main menu video background rendering (missing texture watchdog timeout) on Windows.""",
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
    ('Create-Client-1.0.8-windows-x64.zip', 'Create-Client-1.0.8-windows-x64.zip', 'application/zip'),
    ('create-client-1.0.8-linux-x86_64.tar.gz', 'create-client-1.0.8-linux-x86_64.tar.gz', 'application/gzip')
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
