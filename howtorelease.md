So baust du zukünftig selbst einen Release

Ich habe dir dafür einen Shortcut eingerichtet: npm run release (ruft tauri build auf). Grundablauf:

cd C:\Users\DevSteven\Documents\OpenCode\M2-Tools-Desktop
npm run release

Das macht automatisch alles in einem Rutsch:
1. Frontend-Build (tsc && vite build)
2. Rust-Release-Build (optimiert, dauert einige Minuten — deutlich länger als die cargo check-Läufe, die wir bisher gemacht haben)
3. Windows-Installer werden gepackt (NSIS .exe-Setup und MSI)

Wo die Ergebnisse landen (unter src-tauri/target/release/):
- m2manager.exe — die rohe, portable Exe direkt zum Doppelklicken, keine Installation nötig
- bundle/nsis/M2Manager Community_0.1.0_x64-setup.exe — Installer
- bundle/msi/M2Manager Community_0.1.0_x64_en-US.msi — alternativer Installer

Versionsnummer ändern: steht an zwei Stellen synchron zu halten — package.json ("version") und src-tauri/tauri.conf.json ("version"). Für den nächsten Release z.B. beide auf 0.2.0 setzen, dann npm run release erneut.

Voraussetzungen (bei dir schon alles vorhanden, da wir während der Entwicklung damit gearbeitet haben): Node/npm, Rust/Cargo. Falls npm run release mit cargo: command not found fehlschlägt (mir ist das gerade in meiner Sandbox-Shell passiert, weil cargo dort nicht im PATH war) — einfach ein neues Terminal öffnen, das sollte bei dir normalerweise kein Problem sein, da rustup das PATH dauerhaft einträgt.

Der erste Build für v0.1.0 läuft gerade bei mir im Hintergrund (der cargo-PATH-Fehler ist inzwischen behoben) — melde mich, sobald er fertig ist und sage dir die genauen Pfade.