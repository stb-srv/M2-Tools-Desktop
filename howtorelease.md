So machst du zukünftig einen Release

## Automatisch (seit 2026-08-18, Normalfall)

Ein gepushter Git-Tag baut, signiert und veröffentlicht die neue Version komplett automatisch per GitHub Actions (`.github/workflows/release.yml`) - die App selbst prüft dann beim Start (und über den "Nach Updates suchen"-Knopf in den Einstellungen) automatisch, ob eine neuere Version verfügbar ist, und kann sie direkt installieren.

Ablauf:
1. Version an drei Stellen synchron hochzählen: `src-tauri/tauri.conf.json` ("version"), `src-tauri/Cargo.toml` und `ui/Cargo.toml` (`version = ...`), z.B. alle auf `0.15.0`. Der Tag muss genau dazu passen - sonst bricht die Pipeline gleich am Anfang ab (Prüfschritt seit 0.17.1). Hintergrund: v0.17.0 wurde ohne Hochzählen getaggt, wurde dadurch als 0.16.0 gebaut, und installierte Apps fanden kein Update.
2. **Release-Notes**: in `CHANGELOG.md` den Abschnitt `## [Unveröffentlicht]` umbenennen in `## [0.15.0] - <heutiges Datum>` (und prüfen, dass alles Wichtige drinsteht). Genau dieser Text erscheint in der App als „Was ist neu“, im Update-Hinweis und als Text des GitHub-Release. Ohne Abschnitt schlagen CI und die Release-Pipeline fehl. Lokal prüfen: `cargo run --manifest-path tools/release/Cargo.toml --bin changelog -- check`
3. Committen: `git commit -am "Release 0.15.0"`
4. Taggen und pushen:
   ```
   git tag v0.15.0
   git push origin master v0.15.0
   ```
5. Fertig - GitHub Actions baut, signiert, legt ein GitHub Release mit den Windows-Installern an, übernimmt die Release-Notes aus `CHANGELOG.md` und aktualisiert `updater/latest.json` im Repo, worüber die App die neue Version findet. Fortschritt live unter github.com/stb-srv/M2-Tools-Desktop/actions verfolgbar.

**Einmalige Einrichtung, bevor das zum ersten Mal läuft** (siehe `~/.claude/plans/immutable-mapping-snowglobe.md` für die volle Herleitung):
- (Seit 0.17.1 nicht mehr nötig: das Repo ist öffentlich, die App braucht keinen eingebetteten `GH_RELEASE_TOKEN` mehr. Das Secret kann gelöscht werden.)
- Tauri-Signierschlüssel als Secrets `TAURI_SIGNING_PRIVATE_KEY`/`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` hinterlegt, öffentlicher Teil in `tauri.conf.json` eingetragen

**Wichtig für den allerersten Tag:** noch nicht live getestet (kein vorheriger echter Release zum Gegenprüfen) - beim ersten Mal das Actions-Log genau beobachten, und danach mit einer älteren lokal installierten Version einmal "Nach Updates suchen" → "Installieren" wirklich durchklicken, bevor du dich darauf verlässt.

## Manuell (Fallback, falls die Pipeline mal ausfällt)

Lokal mit der Tauri-CLI bauen:

```
cd C:\Users\DevSteven\Documents\OpenCode\M2-Tools-Desktop
cargo tauri build
```

Macht automatisch alles in einem Rutsch:
1. Frontend-Build (Trunk: Rust → WebAssembly + Tailwind, in `ui/`)
2. Rust-Release-Build (optimiert, dauert einige Minuten)
3. Windows-Installer werden gepackt (NSIS .exe-Setup und MSI)

Wo die Ergebnisse landen (unter `src-tauri/target/release/`):
- `m2manager.exe` — die rohe, portable Exe direkt zum Doppelklicken, keine Installation nötig
- `bundle/nsis/M2Manager Community_<version>_x64-setup.exe` — Installer
- `bundle/msi/M2Manager Community_<version>_x64_en-US.msi` — alternativer Installer

Bei diesem Weg musst du die Dateien selbst verteilen - die automatische Update-Erkennung in der App greift nur bei Releases, die über die GitHub-Actions-Pipeline oben entstanden sind (nur die schreiben `updater/latest.json` fort).

Voraussetzungen (einmalig): Rust/Cargo, `rustup target add wasm32-unknown-unknown`, `cargo install trunk tauri-cli --locked`. Node.js wird nicht mehr gebraucht.
