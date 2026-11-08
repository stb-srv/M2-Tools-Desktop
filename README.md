# M2Manager

Desktop-Werkzeugkasten für Metin2-Privatserver: Server steuern, Datenbank und Server-Dateien bearbeiten, Items anlegen, Quests bauen - alles aus einer App, ohne ständig zwischen SSH-Konsole, MySQL-Client und Texteditor zu wechseln.

Gebaut mit [Tauri 2](https://tauri.app/) (Rust) und React/TypeScript. Läuft unter Windows.

## Was M2Manager kann

**Übersicht**
- **Startseite**: Server-Auswahl (mehrere Server-Profile, z.B. Live und Test), Verbindungsstatus, RAM/Festplatte, Prozesse, Datenbank-Statistiken, System-Diagnose.
- **Server-Steuerung**: Starten, Stoppen, Neustarten, Logs leeren, Quests neu laden, Live-Log.
- **Quellcode bauen & einspielen**: Server-Quellcode auf dem Server bauen und die neue Programmdatei einspielen.

**Items & Ausrüstung**
- **Item Editor**: neue Items anlegen oder bestehende bearbeiten, inkl. Icon, `item_proto`-Neuerzeugung und `icon.epk`-Packen.
- **Item-Übersicht**: komplette `item_proto` als Liste oder Kacheln.
- **Modul-Importer**, **Aufwertungs-Editor** (Refine), **Cube-Editor**.

**Shops & Monster**
- **Shop-Editor**, **Mob-Proto-Editor**, **Drop-Generator** (alle Drop-Dateien inkl. Mob-Drop- und Kisten-Editor).

**Quests & Welt**
- **Quest Builder**, **Regen-Datei-Editor** (Monster-Spawns), **Locale-Verwaltung** (`translate.lua`).

**Server & Accounts**
- **Server-Events** (inkl. Wetter), **Account-Verwaltung** (inkl. GM-Rechte), **Gilden-Verwaltung**, **Wirtschafts-Dashboard**, **Datenbank-Explorer**, **Broadcast-System**.

**Backups & Assets**
- **Backups** (Datei- und Datenbank-Backups), **Änderungsprotokoll**, **TGA-Konverter**, **Icon-Browser**, **3D-Modell-Viewer** (GR2).

Jede Seite hat einen Hilfe-Knopf (?) neben der Überschrift, der das eingebaute Handbuch an der passenden Stelle öffnet.

## Installation

1. Den neuesten Installer von der [Releases-Seite](https://github.com/stb-srv/M2-Tools-Desktop/releases) laden (`…_x64-setup.exe` oder `.msi`) und ausführen.
2. Beim ersten Start führt ein kurzer Assistent durch die Einrichtung: Profilname, SSH-Zugang, MySQL-Zugang, Client-Ordner.
3. Weitere Server legst du direkt auf der Startseite über „Neues Profil“ an.

Updates erkennt die App selbst (Einstellungen → Updates) und installiert sie auf Knopfdruck.

Passwörter liegen in der Windows-Anmeldeinformationsverwaltung, nicht im Klartext. Alle anderen Einstellungen speichert M2Manager in `%APPDATA%\com.m2manager.app\m2manager.sqlite`.

## Entwicklung

Alles ist Rust - kein Node.js nötig. Voraussetzungen: [Rust](https://rustup.rs/) (stable) mit WebAssembly-Ziel, [Trunk](https://trunkrs.dev/), die Tauri-CLI und die [Tauri-Voraussetzungen für Windows](https://tauri.app/start/prerequisites/) (WebView2, MSVC Build Tools).

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk tauri-cli --locked
cargo tauri dev       # App mit Hot-Reload starten (Trunk baut ui/ automatisch)
```

Der 3D-Modell-Viewer braucht zusätzlich den 32-Bit-Sidecar, der die `granny2.dll` des Clients lädt:

```bash
cd src-tauri/gr2-sidecar
cargo build --release --target i686-pc-windows-msvc
```

### Prüfen

```bash
cd ui && cargo test               # Frontend-Logik-Tests
cd ui && trunk build --release    # Frontend-Build (WebAssembly + Tailwind)
cd src-tauri && cargo test --lib  # Backend-Tests
```

Dieselben Prüfungen laufen bei jedem Pull Request automatisch (`.github/workflows/ci.yml`).

Einige Rust-Tests brauchen echte Client-Dateien, eine echte Metin2-Datenbank oder einen erreichbaren Server und sind deshalb mit `#[ignore]` markiert. Sie nutzen die in M2Manager eingerichtete Verbindung bzw. den Client-Pfad, oder die Umgebungsvariablen `M2_TEST_CLIENT_DIR` und `M2_TEST_SETTINGS_DB` (siehe `src-tauri/src/test_support.rs`):

```bash
cd src-tauri && cargo test --lib -- --ignored
```

### Aufbau

- `ui/` - Oberfläche in Rust (Leptos, läuft als WebAssembly im Tauri-Fenster), ein Ordner pro Modul unter `ui/src/features/`, Handbuch unter `ui/src/features/manual/docs/`. Details in `ui/README.md`.
- `src-tauri/src/` - Rust-Backend: SSH/SFTP, MySQL, Datei-Parser, Tauri-Befehle unter `commands/`.
- `src-tauri/gr2-sidecar/` - 32-Bit-Hilfsprogramm für GR2-Modelle.
- `tools/release/` - Hilfsprogramm der Release-Pipeline (schreibt `updater/latest.json`).
- `e2e/` - E2E-Smoke-Test gegen die gebaute .exe (`cargo run --manifest-path e2e/Cargo.toml`).

### Release

Siehe [howtorelease.md](howtorelease.md): Version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` und `ui/Cargo.toml` hochzählen, Tag `vX.Y.Z` pushen, GitHub Actions baut und veröffentlicht den Rest. Änderungen stehen im [CHANGELOG](CHANGELOG.md).
