# Create Client

A dedicated, lightweight custom Minecraft launcher for the **Create Server** modpack (Forge 1.20.1), built with **Tauri v2 (Rust)** and **React 18 / TypeScript / Tailwind CSS**.

Supports **Arch Linux** and **Windows 10/11**.

---

## Features

- **Direct Microsoft OAuth PKCE Flow:** Clean browser login with automatic local redirect (`http://localhost:8483/callback`) and secure credential persistence so players only sign in once.
- **Adoptium Java 17 Auto-Provisioning:** Automatically resolves, downloads, and caches the correct Eclipse Temurin Java 17 JRE per OS on first run.
- **GitHub Releases Modpack Distribution:** Syncs modpack files (`mods/`, `config/`, `defaultconfigs/`, `libraries/`) directly from GitHub Releases with real-time download and verification progress tracking.
- **Preserved Player Data:** Protects personal settings (`options.txt`, `servers.dat`, `screenshots/`, `saves/`, shaderpacks) from being wiped during modpack updates.
- **Live Server Status:** Queries **Create Server** via the Minecraft Server List Ping (SLP) protocol over TCP, displaying real-time latency and online player counts without exposing the raw server IP.
- **Customizable JVM & RAM:** Built-in settings slider for RAM allocation (default 6–8 GB) and custom JVM runtime optimization flags.
- **Multi-Platform CI/CD:** GitHub Actions workflow (`.github/workflows/release.yml`) builds Windows `.exe`/`.msi` and Linux `.AppImage` automatically on version tag push.

---

## Getting Started

### Prerequisites

- **Node.js** & **pnpm**
- **Rust** (`cargo`, `rustc`)
- On Linux (Arch): `sudo pacman -S webkit2gtk-4.1 base-devel openssl`

### Development

To start the launcher in development mode with hot-reloading:

```bash
pnpm install
pnpm tauri dev
```

### Production Build

To build the production installer for your current OS:

```bash
pnpm tauri build
```

- **Linux:** Generates `.AppImage` and `.deb` in `src-tauri/target/release/bundle/`
- **Windows:** Generates `.msi` and `.exe` (NSIS) in `src-tauri/target/release/bundle/`

---

## Microsoft Azure App Registration (Required for Login)

To enable player logins:
1. Visit [portal.azure.com](https://portal.azure.com/) and go to **App registrations**.
2. Click **+ New registration**:
   - **Name:** `Create Client`
   - **Supported account types:** Select **Personal Microsoft accounts only**.
   - **Redirect URI:** Select **Mobile and desktop applications** and enter:
     `http://localhost:8483/callback`
3. Click **Register** and copy your **Application (client) ID**.
4. In the launcher, click the gear icon to open **Settings** and paste your Client ID.

---

## Modpack Hosting & Updates

To push updates to players:
1. Package your modpack files (`mods/`, `config/`, `libraries/`, `version.json`) into `modpack.zip`.
2. Create a GitHub Release with the tag `v1.0.0` (or your current version).
3. Upload `modpack.zip` to the release assets.
4. Update `manifest.json` with the new version and release download URL:
   ```json
   {
     "version": "1.0.0",
     "modpack_url": "https://github.com/varixxl/create-client/releases/download/v1.0.0/modpack.zip",
     "changelog": "Initial release of Create Server Modpack!"
   }
   ```
5. Players launching the game will automatically detect the new version, download the update, and launch into the game.
