# Hinweise für die Arbeit an M2Manager (M2 Tools Desktop)

Tauri-App: Backend in `src-tauri/` (Rust), Frontend in `ui/` (Rust/Leptos →
WASM), Release-Hilfsprogramme in `tools/release/`. Antworten und Texte in der
App sind auf Deutsch.

## Release-Notes - bei JEDER nutzersichtbaren Änderung

`CHANGELOG.md` ist die einzige Quelle der Release-Notes. Die App bettet die
Datei beim Bauen ein und zeigt daraus „Was ist neu“ nach einem Update und die
Notizen unter Einstellungen › Updates; die Release-Pipeline schreibt denselben
Abschnitt in das GitHub-Release und in `updater/latest.json` (Update-Hinweis
vor dem Installieren).

1. Jeder Pull Request mit einer Änderung, die Nutzer merken (Feature, Fix,
   geändertes Verhalten, entferntes Modul), ergänzt `CHANGELOG.md` unter
   `## [Unveröffentlicht]` (Unterabschnitte `### Hinzugefügt`, `### Geändert`,
   `### Behoben`, `### Entfernt`, `### Entwicklung`). Fehlt der Abschnitt,
   oben neu anlegen.
2. Schreibstil: für Anwender, auf Deutsch, ein Punkt pro Änderung,
   `- **Kurztitel**: was ist anders und wo findet man es`. Keine
   Code-Interna, außer unter `### Entwicklung`.
3. Beim Release (Version hochzählen): `## [Unveröffentlicht]` umbenennen in
   `## [X.Y.Z] - JJJJ-MM-TT` und die Version synchron in
   `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` und `ui/Cargo.toml`
   setzen. Details: `howtorelease.md`.

Abgesichert durch:
- CI-Job „Release-Notes“ (`.github/workflows/ci.yml`) und den Test
  `current_version_has_release_notes` in `ui/src/util/release_notes.rs`:
  scheitern, wenn die App-Version keinen nicht leeren Abschnitt hat.
- `release.yml` bricht vor dem Bauen ab, wenn der Abschnitt fehlt.

Lokal prüfen: `cargo run --manifest-path tools/release/Cargo.toml --bin changelog -- check`

## Tests

- Frontend: `cd ui && cargo test`
- Backend: `cd src-tauri && cargo test --lib` (unter Linux vorher
  `libwebkit2gtk-4.1-dev` und `libgtk-3-dev` installieren)
- Release-Tools: `cd tools/release && cargo test`
