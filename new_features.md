# M2Manager – Ideen für neue Funktionen

Gesammelt am 2026-08-03. **Alle 8 Ideen unten wurden noch am selben Tag umgesetzt** — Details, Dateipfade und offene Verifikationspunkte siehe `STATUS.md` (Abschnitt "Alle 8 Ideen aus new_features.md umgesetzt"). Diese Datei bleibt als historische Ideensammlung erhalten.

---

## Mob-Proto-Editor
Es gibt bereits einen Shop-, Item- und Mob-Drop-Editor, aber keinen Editor für die Monster-Stats selbst (`mob_proto`: HP, EXP, Level, AI-Flags, Schaden, Resistenzen etc.). Wäre die logische Lücke zwischen Item Editor und Mob Drop Editor — nutzt vermutlich dieselben Patterns (DB-Read/Write + evtl. Mysql2Proto-Neugenerierung wie beim Item Editor).

## Account-/Spieler-Verwaltung
GM-Level ändern, Bann/Entbann, Item an (online/offline) Spieler geben oder entfernen, Teleport. Nutzt die vorhandene DB-Anbindung, ist aber ein neuer, deutlich sensiblerer Bereich, weil er auf Live-Spielerdaten schreibend zugreift (anders als reine Serverfile-/DB-proto-Bearbeitung bisher). Sollte mit besonderer Vorsicht (Bestätigungsdialoge, evtl. eigenes Audit-Log) angegangen werden.

## Reverse-Lookup für Drops
Globale Suche "welche Mobs droppen Item X?" über alle `mob_drop_item.txt`-Einträge hinweg, statt wie bisher nur mob-für-mob nachzusehen. Würde eine serverweite Auswertung aller Drop-Dateien voraussetzen (nicht nur der aktuell geöffneten).

## Server-Ressourcen-Monitoring im Dashboard
CPU-/RAM-Auslastung des Serverprozesses per SSH abfragen und im Dashboard anzeigen — die SSH-Verbindung besteht ja bereits für die Server-Steuerung. Könnte später auch die Basis für Alerts (z.B. Discord-Webhook bei Serverabsturz) sein.

## Automatisierte Datenbank-Backups
Vollständige `mysqldump`-Backups der Spieler-DB anstoßen und verwalten, zusätzlich zu den bisherigen Backups, die jeder Editor nur für seine eigene bearbeitete Datei anlegt. Wäre die Grundlage für eine echte Disaster-Recovery-Strategie statt nur Einzeldatei-Wiederherstellung.

## Discord-/Webhook-Benachrichtigungen
Bei Serverabsturz, fehlgeschlagenem Backup oder anderen kritischen Ereignissen automatisch eine Nachricht an einen konfigurierbaren Webhook (z.B. Discord) senden.

## Command Palette (Ctrl+K)
Schnelle Navigation über alle Editor-Tabs und evtl. auch direkte Aktionen (z.B. "Item 3219 öffnen") per Tastenkombination. Hängt mit dem in `STATUS.md` bereits vermerkten offenen Punkt "Keyboard Shortcuts" zusammen.

## Icon-/Textur-Browser
Visuelles Durchblättern aller Icons in `icon.epk`, um beim Anlegen eines Items ein Icon auszuwählen statt die vnum-Zuordnung schon zu kennen.
