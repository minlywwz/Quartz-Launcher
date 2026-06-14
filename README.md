# Quartz Launcher

A fast, Prism-inspired Minecraft launcher for Windows with an Apple visionOS–style glass UI.

## Features

- **Offline & Microsoft login** — play without a license (offline UUID) or sign in with Microsoft (device code flow)
- **Onboarding wizard** — theme (light/dark), Discord Rich Presence, default version preset
- **Catalog navigation** — Minecraft versions as categories, modpacks as sub-items (Vanilla + Modrinth)
- **Modrinth & CurseForge** — search and install modpacks (CurseForge requires API key)
- **Rust core + Tauri 2 + Svelte 5** — low RAM, fast startup, native WebView2 shell

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+
- WebView2 (included on Windows 11)

## Setup

```powershell
# Clone and enter project
cd Nebula

# Copy env template and fill in optional keys
copy .env.example .env

# Install UI + Tauri CLI dependencies
npm install --prefix apps/quartz-ui
npm install --prefix apps/quartz-launcher

# Dev mode (starts Vite + Tauri)
npm run dev
```

If dev fails with **Port 1420 is already in use**, a previous session is still running. Either close the old Quartz/Vite window, or run:

```powershell
node scripts/free-port.mjs 1420
npm run dev
```

(`npm run dev` now runs this cleanup automatically via `predev`.)

## Environment variables

| Variable | Purpose |
|----------|---------|
| `AZURE_CLIENT_ID` | Microsoft OAuth app for login |
| `CURSEFORGE_API_KEY` | CurseForge modpack search ([console.curseforge.com](https://console.curseforge.com/)) |
| `DISCORD_APP_ID` | Discord Rich Presence application ID |

## Build

```powershell
npm run build:ui
npm run build
```

Output: `apps/quartz-launcher/target/release/quartz-launcher.exe`

## Project structure

```
crates/quartz-auth/          Offline UUID + MSA auth
crates/quartz-meta/          Mojang version manifest
crates/quartz-modplatforms/  Modrinth + CurseForge clients
crates/quartz-catalog/       Version → modpack sidebar tree
crates/quartz-instance/      Instance CRUD
crates/quartz-download/      Parallel downloads
crates/quartz-launch/        JVM launch args + spawn
apps/quartz-launcher/        Tauri shell
apps/quartz-ui/              Svelte 5 frontend
```

## Data locations

- Settings: `%USERPROFILE%\.quartz\settings.json`
- Game data: `%APPDATA%\Quartz\`

## Microsoft login (Azure)

1. Open [Azure Portal](https://portal.azure.com) → **App registrations** → **New registration**
2. Name: `Quartz Launcher`, supported account types: **Personal Microsoft accounts only**
3. Redirect URI: leave blank (device code flow does not need one)
4. After creation, copy **Application (client) ID** → set as `AZURE_CLIENT_ID` in `.env`
5. Under **Authentication**, enable **Allow public client flows** (required for device code)
6. No API permissions needed beyond defaults — Xbox/Minecraft auth uses consumer endpoints

## License

MIT — not affiliated with Mojang or Microsoft.
