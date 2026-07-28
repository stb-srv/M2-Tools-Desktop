M2-Tools

A modern, cross-platform desktop application for managing Metin2 game files. M2Manager streamlines your workflow with visual editors, integrated connections, 3D previews, and automatic updates — no command line required.

---

## Features

### 🛒 Shop Editor

Manage your in-game shops visually. M2Manager connects to your game database and lets you:

- **Create shops** with a step-by-step dialog — pick an NPC, name your shop, and start adding items
- **Edit items** in a grid layout with inline quantity adjustments (plus/minus controls)
- **Search & filter** items by name or VNUM from the full game item database
- **Bulk operations** — synchronize stack counts across all shops when limits change
- **Delete shops** with a confirmation dialog that prevents accidental data loss

### 🎮 3D Model Viewer

Preview Granny 3D (`.gr2`) game models directly inside the app:

- Interactive orbit controls (rotate, zoom, pan)
- Phong-shaded rendering with configurable lighting
- Supports characters, mobs, NPCs, items, and equipment
- Integrated into the NPC picker — see the model before assigning it to a shop

### 🔌 Connection Manager

Configure and manage your server connections from the Settings panel:

| Connection | Description |
|---|---|
| **SSH** | Secure shell access to your game server. Supports password and key-based authentication. |
| **MySQL** | Direct database connection for shop management and item lookups. Test connections before saving. |

Switch between connection profiles instantly. All credentials are stored locally on your machine.

### 🗺️ Path & Mapping Configuration

Set up your game file paths once, use them everywhere:

- **Game Client Path** — points to your local Metin2 client files
- **Server Path** — the root directory of your game server
- **Working Directory** — where M2Manager stores temporary and generated files
- **Mapping Paths** — configure client/server atlas and map data directories

### 📊 Dashboard

A central overview of your environment:

- Connection status for SSH and MySQL
- Database statistics
- Quick access to common actions

### 🌍 Multi-Language Interface

M2Manager is fully localized. Switch languages on the fly from the settings panel.

| Language | Status |
|---|---|
| 🇬🇧 English | ✅ Complete |
| 🇩🇪 German | ✅ Complete |
| 🇫🇷 French | ✅ Complete |
| 🇪🇸 Spanish | ✅ Complete |
| 🇵🇹 Portuguese | ✅ Complete |
| 🇮🇹 Italian | ✅ Complete |
| 🇳🇱 Dutch | ✅ Complete |
| 🇹🇷 Turkish | ✅ Complete |
| 🇷🇺 Russian | ✅ Complete |
| 🇷🇴 Romanian | ✅ Complete |
| 🇨🇿 Czech | ✅ Complete |
| 🇭🇺 Hungarian | ✅ Complete |
| 🇬🇷 Greek | ✅ Complete |

### 🔄 Automatic Updates

M2Manager checks for updates automatically and notifies you when a new version is available:

1. **Update available** — a badge appears with release notes
2. **"Update"** — downloads and installs immediately, then restarts
3. **"Later"** — the update downloads in the background and installs when you close the app

No manual downloads. No installers. Just click and go.

### ⚙️ Customization

- **Dark / Light theme** — with automatic OS preference detection
- **Setup Wizard** — guided first-run configuration
- **Keyboard shortcuts** — power-user hotkeys for common actions
- **Responsive layout** — works on any screen size from laptops to ultrawide monitors

---

## Installation

### Windows

1. Download `M2Manager Community (x64).exe` from the [latest release](../../releases/latest)
2. Run the executable — no installation required
3. Follow the setup wizard on first launch

### Linux

1. Download `M2Manager Community (x64)` from the [latest release](../../releases/latest)
2. Make it executable: `chmod +x "M2Manager Community (x64)"`
3. Run: `./M2Manager\ Community\ \(x64\)`

### macOS

1. Download `M2Manager Community (arm64).dmg` (Apple Silicon) or `M2Manager Community (x64).dmg` (Intel)
2. Open the DMG and drag M2Manager to your Applications folder
3. Launch from Applications

---

## Getting Started

1. **Launch M2Manager** — the Setup Wizard will guide you through initial configuration
2. **Set your paths** — point M2Manager to your game client and server directories
3. **Configure connections** — add your SSH and MySQL server details in Settings → Connections
4. **Start managing** — open the Shop Editor to create and edit your game shops
