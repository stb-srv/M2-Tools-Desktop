# Changelog

Alle nennenswerten Änderungen an M2Manager werden hier dokumentiert. Format
angelehnt an [Keep a Changelog](https://keepachangelog.com/), Versionierung
nach [SemVer](https://semver.org/) (`MAJOR.MINOR.PATCH`, solange < 1.0 gilt
`MINOR` für neue Features, `PATCH` für reine Fixes). Für die vollständige
Feature-Historie mit allen Details siehe `STATUS.md`.

## [0.3.0] - 2026-08-06

### Hinzugefügt

- **Aufwertungs-Editor (Refine)**: neuer Bereich, der die komplette
  Aufwertungs-Kette eines Items visualisiert (alle Stufen, benötigte
  Materialien, Gold-Kosten, Erfolgschance) statt roher
  `refine_proto`-Zeilen im Datenbank-Explorer. Rezepte lassen sich direkt
  anlegen, bearbeiten oder wiederverwenden (inkl. Warnung, wenn ein
  Rezept von mehreren Items gemeinsam genutzt wird), und zu jedem
  Material gibt es einen „Woher bekomme ich das?"-Hinweis (Mob-Drops und
  NPC-Shops). Wichtig: Änderungen an Aufwertungs-Rezepten wirken erst
  nach einem Server-Neustart, da der Server `refine_proto` nur beim
  Start aus der Datenbank lädt (kein Client-Repack wie bei anderen
  Editoren) — im Bereich entsprechend markiert.

## [0.2.4] - 2026-08-06

### Behoben

- Modul-Import brach mit `stream did not contain valid UTF-8` ab und nahm
  alle bereits angelegten Items zurück, sobald irgendein Dateiname im
  `item`-Ordner (auch aus einem früheren, unabhängigen Import) einen
  Umlaut enthielt — die Ausgabe der Pack-Tools wird jetzt korrekt als
  Windows-1252 statt strikt als UTF-8 gelesen.
- Rückgängig-machen nach einem fehlgeschlagenen Import entfernte bisher
  nur den Datenbank-Eintrag, nicht Icon und `item_list.txt`-Zeile — blieb
  als Datenmüll liegen. Der Rollback räumt jetzt vollständig auf.

## [0.2.3] - 2026-08-06

### Behoben

- Importierte Waffen mit einem Modell, dessen Textur-Referenz vom
  Grafiker als **absoluter** Pfad exportiert wurde (z. B.
  `D:\ymir work\...\skin.dds` statt eines relativen Pfads), zeigten im
  Client keine Textur (Waffe erscheint weiß) — betraf tatsächlich
  **beide** bereits importierten Waffen-Sets (FrostEdge, FireDragon), nur
  eines wurde gemeldet. Ursache im echten Client-Quellcode verifiziert:
  bei einer absoluten Referenz sucht der Client exakt diesen Pfad und
  nur in gepackten `.epk`-Dateien, nie in losen Dateien. Der
  Modul-Importer platziert die Textur jetzt zusätzlich am exakten
  virtuellen Pfad, den das Modell erwartet (per GR2-Parser zur
  Import-Zeit ermittelt). Beide bereits live importierten Waffen-Sets
  wurden nachträglich repariert.

## [0.2.2] - 2026-08-06

### Geändert

- Modul-Importer: importierte Waffen-Modelle heißen jetzt `<vnum>.gr2`
  statt beim Original-Dateinamen des Pakets (z. B. `2h.gr2`) zu bleiben —
  konsistent mit der bereits vnum-basierten Icon-Benennung. Zugehörige
  Texturen behalten weiterhin ihren Original-Dateinamen (wird sowohl vom
  Modell selbst als auch vom Textur-Lookup so erwartet).

### Geklärt

- Die per-Item `.msm`-Dateien unter `pack/item/ymir work/item/*.msm`
  (z. B. `00010.msm`) werden von diesem Client **nicht** gelesen — im
  echten Client-Quellcode verifiziert. Für Waffen reicht der bereits vom
  Modul-Importer geschriebene `item_list.txt`-Eintrag vollständig aus,
  eigene `.msm`-Dateien müssen dafür nicht angelegt werden.

## [0.2.1] - 2026-08-06

### Geändert

- Frontend-Build gibt keine „chunks larger than 500 kB"-Warnung mehr aus:
  alle Sidebar-Bereiche außer dem Dashboard laden jetzt erst bei Aufruf
  nach (`React.lazy`), statt alle 18 Panels in ein einziges 1,5-MB-Bundle
  zu packen. Haupt-Bundle jetzt 327 kB.
- Release-Builds (`npm run release`) sind spürbar schneller: das Rust-
  Release-Profil nutzt jetzt Thin- statt Fat-LTO (gemessen auf diesem
  Rechner: 7:55 Min → 4:01 Min bei einem inkrementellen Rebuild), bei
  praktisch gleicher Programmgröße/-geschwindigkeit.

## [0.2.0] - 2026-08-06

### Hinzugefügt

- **Modul-Importer**: automatischer Import fertiger Ausrüstungs-Pakete
  (Waffen und/oder Rüstung, auch gemischt in einem Ordner) über einen
  generischen Ordner-Scan, der unabhängig von der genauen Struktur des
  gelieferten Pakets funktioniert (nicht mehr auf ein einziges festes
  Layout beschränkt).
  - Waffen: automatische Subtyp-Erkennung über Dateinamens-Aliase,
    Icon-Zuordnung per Fuzzy-Matching, Textur-Kopie neben dem Modell,
    Klassen-Beschränkung je Waffentyp.
  - Rüstung: automatische 3D-Modell-Verknüpfung für weibliche Charaktere
    über die Client-eigene `.msm`-Datei (`ShapeData`-Eintrag); männliche
    Körpermodelle bleiben vorerst manuelle Arbeit (im Importer erklärt,
    warum).
  - Import-Verlauf mit vollständigem Rückgängig-machen pro importiertem
    Paket, sowie Einzel-Item-Entfernung unabhängig vom Verlauf.
  - Referenz-Item-Übernahme für Beispielwerte (Schaden/Verteidigung,
    Preise, Flags).

### Behoben

- Neu angelegte Item-Icons wurden im Client teilweise **unsichtbar**
  dargestellt (Inventar-Slot leer, nur der Tooltip beim Hovern zeigte noch
  etwas an) — Ursache war eine RLE-komprimierte TGA-Ausgabe, die der
  Client nicht zuverlässig lesen kann. Der TGA-Export schreibt jetzt
  unkomprimiert, im selben Format wie die Original-Client-Icons.
- Die Referenz-Item-Übernahme im Modul-Importer konnte still fehlschlagen,
  ohne dass die gewählten Werte tatsächlich ins neue Item übernommen
  wurden.
- Der Waffen-Modell-Import kopierte nur die `.gr2`-Datei, nicht die
  zugehörigen Textur-Dateien — die Waffe erschien im Client untexturiert.

## [0.1.0] - 2026-08-03

Erste funktionsfähige Version. Wesentliche Bereiche:

### Hinzugefügt

- Grundgerüst (Tauri v2 + React/TS/Vite), Dark/Light-Theme, i18n (DE/EN),
  Command Palette (Strg+K)
- Verbindungsverwaltung (SSH + MySQL) mit sicherer Zugangsdaten-Speicherung
  über den Windows Credential Manager
- Dashboard mit Server-Ressourcen-Monitoring, Server-Übersicht (IP/RAM/
  Speicherplatz) und Discord-Webhook-Benachrichtigungen bei Server-Absturz
- Server-Steuerung (Start/Stop/Neustart/Logs löschen/Quests neu laden)
- Generischer Database Explorer für beliebige Tabellen
- Shop Editor mit Live-3D-NPC-Vorschau, echten Item-Icons und dem
  40-Slot-Limit des Client-Fensters
- 3D-Modell-Viewer (GR2) über einen 32-Bit-Sidecar-Prozess, der die
  eigene `granny2.dll` des Clients lädt
- Item Editor (Anlegen/Bearbeiten/Duplizieren), alle Flag-/Typ-Tabellen
  gegen den echten Server-Quellcode verifiziert
- Mob Drop Editor, Mob-Proto-Editor, Quest Builder (inkl. 6 Vorlagen für
  gängige Quest-Muster), Regen-Datei-Editor mit Kartenansicht
- Locale-String-Verwaltung, Backup-Browser mit Diff-Ansicht,
  automatisierte Datenbank-Backups
- Account-/Spieler-Verwaltung, Icon-/Textur-Browser, TGA-Konverter
- Sidebar mit Kategorien, Favoriten/Zuletzt-verwendet und Hinweis auf
  ungespeicherte Änderungen

Vollständige Detail-Historie mit allen Einzelschritten: siehe `STATUS.md`.
