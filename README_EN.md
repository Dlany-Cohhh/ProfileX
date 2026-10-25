# ProfileX

> A multi-account browser environment manager — one profile, one isolated identity.

[![Version](https://img.shields.io/badge/version-1.0.0-2ea043)](https://github.com/Dlany-Cohhh/ProfileX/releases)
[![Platform](https://img.shields.io/badge/platform-Windows-0a84ff)](#download--install)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24c8db)](https://tauri.app)
[![Vue](https://img.shields.io/badge/Vue-3-42b883)](https://vuejs.org)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

English | [简体中文](README.md)

ProfileX is a desktop app built with **Rust + Tauri v2 + Vue 3** for managing dozens of isolated Chrome environments from one clean window. Every profile gets its own user-data directory, cookies, extensions and launch flags, so accounts never bleed into each other. All data stays in a local SQLite file — nothing is uploaded.

![Main window](docs/screenshots/01-main-grid.png)

## Features

| Capability | Details |
| --- | --- |
| **Isolation** | Dedicated `--user-data-dir` per profile: cookies, cache, extensions and sessions are fully separate |
| **Workspaces** | Group profiles in the sidebar (with icon and color); launch or back up a whole workspace in one click |
| **Templates** | Save a tuned environment as a template; clone it as "extensions & scripts only" or "full copy with login state" |
| **Batch actions** | Multi-select to launch, close, move, clear cache, back up or delete in bulk |
| **Launch flags** | Launch port, CDP debug port, port forwarding, debugger mode, disable extensions, proxy, user agent, window size |
| **Performance report** | Startup time recorded and split into process creation / DNS / DOM estimates, plus actionable suggestions |
| **Backup & restore** | Zip a single profile or a whole workspace (metadata included) and import it on another machine; auto-prune to N copies |
| **System tray** | Close to tray, launch at login, start favourite profiles right from the tray menu |
| **Looks** | 8 accent colors + light/dark mode, custom avatar and icon color per profile |
| **Bilingual UI** | Simplified Chinese / English, switched instantly |

## Screenshots

| Profile form | Performance report |
| --- | --- |
| ![Profile form](docs/screenshots/02-profile-form.png) | ![Performance](docs/screenshots/03-performance.png) |

| Settings (directories / theme / window behaviour) |
| --- |
| ![Settings](docs/screenshots/04-settings.png) |

> Profile names and paths in the screenshots are sample data.

## Download & install

Grab the latest Windows build from [Releases](https://github.com/Dlany-Cohhh/ProfileX/releases):

- `ProfileX_1.0.0_x64-setup.exe` — NSIS installer (recommended)
- `ProfileX_1.0.0_x64_en-US.msi` — MSI installer

More details (SmartScreen warning, uninstall, data locations) in [docs/install.md](docs/install.md).

**Requirements**: Windows 10 / 11 (x64) and Google Chrome installed (any Chromium-based browser can be pointed to manually in a profile).

## Quick start

1. Open ProfileX and click **New profile** in the top-right corner.
2. Give it a name; tags, notes and icon color are optional. Leave the Chrome path empty to auto-detect.
3. Hit the blue launch button on the card — the browser opens in its own isolated environment.
4. Need another account? Create another profile, or clone an existing one as a template.
5. Put long-lived profiles into a workspace, then launch the whole group from the sidebar.

## How it works

### Workspaces

Profiles belong to at most one workspace; the rest live under "Ungrouped". Each workspace has an icon and color, plus two action buttons at the bottom of the sidebar to launch or back up everything inside it.

### Templates

Any profile can be marked as a template. When creating a profile you can pick one and choose:

- **Extensions & scripts only** — copies extensions, bookmarks and similar static data with a fresh login state.
- **Full copy** — copies cookies and login state too, giving you an independent duplicate of the same account.

### Backups

A backup is a zip archive containing the browser user data plus profile metadata (name, icon, tags, ports, launch flags, workspace). Importing it on another machine therefore restores both the environment and its settings. Backup directory and retention count are configurable (0 = unlimited).

## Data & privacy

- Profile database: `%APPDATA%\com.bujic.profilex\profiles.db`
- Browser data: `%APPDATA%\com.bujic.profilex\profiles\` (relocatable in Settings)
- Backups: `Documents\ProfileX\Backups\`
- No telemetry, no analytics, no cloud sync. The only outbound request is the one you trigger yourself with "Test proxy".

## Build from source

Node.js 20+ and a stable Rust toolchain are required (MSVC build tools on Windows).

```bash
git clone https://github.com/Dlany-Cohhh/ProfileX.git
cd ProfileX
yarn install
yarn tauri dev      # development
yarn tauri build    # production, output in src-tauri/target/release/bundle/
```

## Tech stack

| Layer | Choice |
| --- | --- |
| Shell | Tauri v2 (Rust) |
| Frontend | Vue 3 + TypeScript + Vite + Pinia |
| UI | Element Plus with custom theme variables |
| Storage | SQLite (rusqlite) + zip backups |
| Browser control | Chrome DevTools Protocol |

## Acknowledgements

ProfileX is a **derivative work** based on [chao-eng/SpaceShift](https://github.com/chao-eng/SpaceShift).

Huge thanks to [@chao-eng](https://github.com/chao-eng) for open-sourcing SpaceShift — its architecture, profile isolation model and backup format are the starting point of this project. ProfileX rebuilds the UI and interactions on top of it, unifies the branding, and adds list filtering, performance reporting, theming and workspace improvements.

Thanks as well to the open-source projects this app stands on: Tauri, Vue, Element Plus, rusqlite, zip-rs, walkdir and chrono.

## License

[MIT](LICENSE) © 2026 bujic
