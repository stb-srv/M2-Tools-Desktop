# Datenbank-Backups

Erstellt, listet und stellt komplette Datenbank-Sicherungen wieder her (`mysqldump`/`mysql`-Restore über SSH, mit den bereits konfigurierten MySQL-Zugangsdaten).

## Was du hier tun kannst

- Sicherung anlegen (welche Datenbanken, konfigurierbar in den Einstellungen).
- Bestehende Sicherungen auflisten.
- Wiederherstellen - mit deutlicher Warnung, da das die **komplette** Live-Datenbank überschreibt.
- Endgültig löschen.

## Wichtig zu wissen

Das Passwort wird beim Dump/Restore über eine Umgebungsvariable übergeben, taucht also nicht in der Prozessliste auf dem Server auf. Ein Restore ist die einzige wirklich destruktive Aktion in diesem Bereich - die Warnung davor ist absichtlich deutlich.
