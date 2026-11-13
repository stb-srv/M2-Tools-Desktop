So machst du zukünftig einen Release

## Automatisch per Knopf (seit 2026-11-13, Normalfall)

Du musst weder Versionen hochzählen noch Tags setzen. Das erledigt GitHub Actions (`.github/workflows/release.yml`) in einem Lauf.

Ablauf:
1. Alle Pull Requests für die Version sind gemergt, und ihre Änderungen stehen in `CHANGELOG.md` unter `## [Unveröffentlicht]` (das macht jeder PR selbst, siehe `CLAUDE.md`).
2. Auf github.com/stb-srv/M2-Tools-Desktop/actions links **Release** wählen, rechts **Run workflow** drücken.
3. Branch `master` lassen und die Art wählen:
   - **patch**: nur Fehlerbehebungen, 0.30.1 → 0.30.2
   - **minor**: neue Funktionen, 0.30.1 → 0.31.0
   - **major**: 0.30.1 → 1.0.0
   - **aktuell**: Version bleibt. Nur nötig, wenn ein Release dieser Version abgebrochen ist und wiederholt werden soll.
   - Oder im Feld darunter eine Version ausschreiben (z. B. `0.40.0`).
4. Grün auf **Run workflow** drücken. Fertig.

Was der Lauf macht:
- **Vorbereiten** (ein paar Sekunden): Version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `ui/Cargo.toml` und beiden `Cargo.lock` setzen, `## [Unveröffentlicht]` in `## [X.Y.Z] - <heute>` umbenennen, als „Release X.Y.Z“ auf master committen und den Tag `vX.Y.Z` setzen. Fehlen die Release-Notes, bricht er hier ab, bevor irgendwas geändert ist.
- **Bauen** (ca. 20 Minuten): Windows-Installer bauen und signieren, GitHub-Release mit den Notizen aus `CHANGELOG.md` anlegen, `updater/latest.json` auf master schreiben. Darüber findet die App das Update.

Sicherungen:
- Gibt es den Tag schon mit fertigem Release, bricht der Lauf ab. Gibt es ihn ohne Release (z. B. weil er früher zu früh gesetzt wurde), wird er auf den richtigen Stand verschoben.
- Es läuft immer nur ein Release gleichzeitig.
- Ist master während des Baus weitergewandert, wird `updater/latest.json` trotzdem geschrieben (bei v0.30.0 scheiterte genau das).

### Fallback: Tag von Hand

Ein von Hand gepushter Tag `vX.Y.Z` startet weiterhin nur den Bau. Dann musst du vorher selbst die Version in allen Dateien setzen und den Changelog-Abschnitt umbenennen, sonst bricht die Pipeline ab (Prüfschritt seit 0.17.1). Lieber den Knopf nehmen.

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
