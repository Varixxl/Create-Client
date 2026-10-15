# Create Client ⚙️

[![Minecraft](https://img.shields.io/badge/Minecraft-1.20.1-brightgreen.svg)](https://minecraft.net/)
[![Forge](https://img.shields.io/badge/Forge-47.4.20-orange.svg)](https://files.minecraftforge.net/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-blue.svg)](https://tauri.app/)
[![Platform](https://img.shields.io/badge/Platform-Arch%20Linux%20%7C%20Windows%2010%2F11-blueviolet.svg)](#system-prerequisites)
[![License](https://img.shields.io/badge/License-MIT-lightgrey.svg)](LICENSE)

A standalone, zero-dependency, one-click launcher for the **Create Server** modpack. Built with **Tauri v2 (Rust)** and **React 18 / TypeScript / Tailwind CSS**.

---

## 📥 Download Create Client

Download the latest release (**v1.0.0**) for your operating system:

| Platform | Format | Direct Download | Installation & Notes |
| :--- | :--- | :--- | :--- |
| **Windows 10 / 11** | Standalone Executable (`.exe`) | [**Create-Client.exe**](https://github.com/JaviDaBomb/Create-Client/releases/download/v1.0.0/Create-Client.exe) | **1-Click Launch.** No installer needed; double-click to run. |
| **Windows 10 / 11** | Portable ZIP Archive (`.zip`) | [**Create-Client-1.0.0-windows-x64.zip**](https://github.com/JaviDaBomb/Create-Client/releases/download/v1.0.0/Create-Client-1.0.0-windows-x64.zip) | Portable bundle with quick-start instructions. |
| **Linux (Universal)** | Standalone AppImage (`.AppImage`) | [**Create-Client-1.0.0.AppImage**](https://github.com/JaviDaBomb/Create-Client/releases/download/v1.0.0/Create-Client-1.0.0.AppImage) | **Self-contained.** Bundles WebKitGTK and dependencies. Works on Arch, Ubuntu, Fedora, Debian, Mint. Run `chmod +x` and double-click. |
| **Linux (Arch / Any)** | Portable Tarball (`.tar.gz`) | [**create-client-1.0.0-linux-x86_64.tar.gz**](https://github.com/JaviDaBomb/Create-Client/releases/download/v1.0.0/create-client-1.0.0-linux-x86_64.tar.gz) | Includes automated `./install.sh` script, icon, and desktop launcher. |
| **Ubuntu / Debian** | Debian Package (`.deb`) | [**create-client_1.0.0_amd64.deb**](https://github.com/JaviDaBomb/Create-Client/releases/download/v1.0.0/create-client_1.0.0_amd64.deb) | Install with `sudo dpkg -i create-client_1.0.0_amd64.deb` or Software Center. |
| **Fedora / RHEL** | RPM Package (`.rpm`) | [**create-client-1.0.0-1.x86_64.rpm**](https://github.com/JaviDaBomb/Create-Client/releases/download/v1.0.0/create-client-1.0.0-1.x86_64.rpm) | Install with `sudo dnf install ./create-client-1.0.0-1.x86_64.rpm`. |

All downloads and checksums are also hosted on the [GitHub Releases page](https://github.com/JaviDaBomb/Create-Client/releases/latest).

---

## 🚀 Key Features & Functionality

- **Zero-Dependency One-Click Launch:**
  Players do not need Java installed, nor do they need the official Minecraft launcher, Modrinth, or CurseForge. Create Client automatically provisions an isolated **Eclipse Temurin Java 17 JRE** and manages all Minecraft 1.20.1 and Forge 47.4.20 dependencies in a self-contained sandbox.

- **Sequential Differential Delta Updating:**
  Never re-download 1.4 GB for a minor update again. Create Client calculates and downloads only changed, added, or modified files via sequential delta ZIP patches directly from GitHub Releases, saving bandwidth and update time.

- **Microsoft Device Code OAuth Flow:**
  No confusing port forwarding or local redirect URIs. Players receive an 8-character device code that can be authenticated on any browser or mobile device (`microsoft.com/link`). Credentials and refresh tokens are securely encrypted on disk.

- **Live Server Update Log & Patch Notes Drawer:**
  View the real-time server changelog directly in the launcher. The update log drawer renders formatted Markdown directly from the in-game FancyMenu configuration (`updatelog_layout.txt`), keeping players informed of server voting systems, balance adjustments, and mod updates before jumping into the server.

- **Direct GitHub Client Self-Updater:**
  Check for and apply launcher updates with a single click. Create Client queries the GitHub Releases API and seamlessly updates the launcher binary in place.

- **Game Lifecycle Monitoring:**
  The launcher tracks the running Minecraft instance in real time. The **Play Game** button dynamically transitions to an active glowing **Running** state while the game is open, preventing accidental multi-instances and reverting to **Play Game** as soon as Minecraft exits.

- **Direct Community & Directory Integration:**
  - **Game Folder:** Open the active modpack directory in your system file manager with one click.
  - **Discord Button:** Quick access to the official Create Server Discord community (`https://discord.gg/CFqskvhF8k`).
  - **Player Data Protection:** Personal keybinds, settings (`options.txt`), multiplayer servers (`servers.dat`), worlds (`saves/`), and screenshots are strictly preserved across modpack updates.

---

## 💻 System Prerequisites

Create Client is designed to be completely frictionless:

### Windows 10 & 11
- **Prerequisites:** **None.**
- Windows 10/11 includes the **Microsoft Edge WebView2** runtime out of the box.
- Java 17 is **auto-downloaded** by the client on first launch into the user app directory (`%LOCALAPPDATA%\createclient\runtime`).
- No admin privileges required.

### Arch Linux (and other Linux distributions)
- **AppImage:** **Zero prerequisites.** The AppImage bundles WebKitGTK and desktop dependencies self-contained.
- **Native Binary / Tarball:** Standard desktop WebKitGTK (`webkit2gtk-4.1`), which is pre-installed on virtually all Linux desktop environments (GNOME, KDE Plasma, Hyprland, etc.).
  - If missing on minimal setups: `sudo pacman -S webkit2gtk-4.1` (Arch) or `sudo apt install libwebkit2gtk-4.1-0` (Debian/Ubuntu).
- Java 17 is **auto-downloaded** by the client on first launch into `~/.local/share/createclient/runtime/java17`. No system Java packages needed.

---

## 🛠️ Modpack & Game Directory Layout

The launcher stores all game and runtime assets in standard OS user directories:

- **Linux:**
  - Game Directory: `~/.local/share/createclient/game`
  - Java 17 JRE: `~/.local/share/createclient/runtime/java17`
  - Settings & Auth: `~/.config/createclient/`
  - Diagnostic Log: `~/.local/share/createclient/game/launcher_log.txt`
- **Windows:**
  - Game Directory: `%LOCALAPPDATA%\createclient\game`
  - Java 17 JRE: `%LOCALAPPDATA%\createclient\runtime\java17`
  - Settings & Auth: `%APPDATA%\createclient\`

---

## 👨‍💻 Building from Source

If you want to contribute or compile Create Client from source:

### Requirements
- **Node.js** (v18+) & **pnpm**
- **Rust** & **Cargo** (1.78+)
- Linux build dependencies: `sudo pacman -S base-devel openssl webkit2gtk-4.1`

### Build Instructions

```bash
# Clone the repository
git clone https://github.com/JaviDaBomb/Create-Client.git
cd Create-Client

# Install web dependencies
pnpm install

# Run in development mode with hot-reloading
pnpm tauri dev

# Build production binary
pnpm tauri build --no-bundle
```

The compiled release executable will be located at `src-tauri/target/release/create-client` (Linux) or `create-client.exe` (Windows).

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.
