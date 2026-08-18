So machst du zukünftig einen Release

## Automatisch (seit 2026-08-18, Normalfall)

Ein gepushter Git-Tag baut, signiert und veröffentlicht die neue Version komplett automatisch per GitHub Actions (`.github/workflows/release.yml`) - die App selbst prüft dann beim Start (und über den "Nach Updates suchen"-Knopf in den Einstellungen) automatisch, ob eine neuere Version verfügbar ist, und kann sie direkt installieren.

Ablauf:
1. Version an zwei Stellen synchron hochzählen: `package.json` ("version") und `src-tauri/tauri.conf.json` ("version"), z.B. beide auf `0.15.0`.
2. Committen: `git commit -am "Release 0.15.0"`
3. Taggen und pushen:
   ```
   git tag v0.15.0
   git push origin master v0.15.0
   ```
4. Fertig - GitHub Actions baut, signiert, legt ein GitHub Release mit den Windows-Installern an, und aktualisiert `updater/latest.json` im Repo, worüber die App die neue Version findet. Fortschritt live unter github.com/stb-srv/M2-Tools-Desktop/actions verfolgbar.

**Einmalige Einrichtung, bevor das zum ersten Mal läuft** (siehe `~/.claude/plans/immutable-mapping-snowglobe.md` für die volle Herleitung):
- GitHub-Fine-grained-Token (nur Lesezugriff auf dieses Repo) als Secret `GH_RELEASE_TOKEN` hinterlegt
- Tauri-Signierschlüssel als Secrets `TAURI_SIGNING_PRIVATE_KEY`/`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` hinterlegt, öffentlicher Teil in `tauri.conf.json` eingetragen

**Wichtig für den allerersten Tag:** noch nicht live getestet (kein vorheriger echter Release zum Gegenprüfen) - beim ersten Mal das Actions-Log genau beobachten, und danach mit einer älteren lokal installierten Version einmal "Nach Updates suchen" → "Installieren" wirklich durchklicken, bevor du dich darauf verlässt.

## Manuell (Fallback, falls die Pipeline mal ausfällt)

Der alte Weg funktioniert weiterhin unverändert - `npm run release` (ruft `tauri build` auf):

```
cd C:\Users\DevSteven\Documents\OpenCode\M2-Tools-Desktop
npm run release
```

Macht automatisch alles in einem Rutsch:
1. Frontend-Build (tsc && vite build)
2. Rust-Release-Build (optimiert, dauert einige Minuten)
3. Windows-Installer werden gepackt (NSIS .exe-Setup und MSI)

Wo die Ergebnisse landen (unter `src-tauri/target/release/`):
- `m2manager.exe` — die rohe, portable Exe direkt zum Doppelklicken, keine Installation nötig
- `bundle/nsis/M2Manager Community_<version>_x64-setup.exe` — Installer
- `bundle/msi/M2Manager Community_<version>_x64_en-US.msi` — alternativer Installer

Bei diesem Weg musst du die Dateien selbst verteilen - die automatische Update-Erkennung in der App greift nur bei Releases, die über die GitHub-Actions-Pipeline oben entstanden sind (nur die schreiben `updater/latest.json` fort).

Voraussetzungen: Node/npm, Rust/Cargo (bei dir schon vorhanden).
