# Einstellungen

Alle Verbindungsdaten und Pfade, die die anderen Module brauchen, an einer Stelle.

## Allgemein

- **Design** (Hell/Dunkel/System) und **Sprache**.

## Server

- **SSH**: Host/Port/Benutzername + Passwort oder privater Schlüssel - wird für alles gebraucht, was direkt auf dem Server liest/schreibt (Quest Builder, Regen-Editor, Mob Drop Editor, Server-Steuerung, Build & Deploy, System-Installer bei Server-Patches). Zugangsdaten liegen sicher im Windows Credential Manager, nicht im Klartext.
- **MySQL**: Host/Port/Benutzername + Passwort - für alle Datenbank-Editoren (Item-, Shop-, Mob-Proto-Editor, Account-Verwaltung, Datenbank-Explorer, Aufwertungs-Editor, Dashboard-Statistiken).
- **Mysql2Proto-Pfad**: lokaler Ordner des externen `Mysql2Proto.exe`-Tools - wird vom Item Editor gebraucht, um `item_proto` clientseitig neu zu erzeugen.
- **VNUM-Bereich**: der vnum-Startwert, ab dem der Item Editor neue Items vorschlägt.
- **Mob-Drop-Dateipfad, Regen-Basisordner, Locale-Dateipfad**: Server-Pfade zu den jeweiligen Dateien für Mob Drop Editor, Regen-Datei-Editor und Locale-Verwaltung.
- **Server-Prozessnamen, Festplatten-Pfad**: für das Ressourcen-Monitoring im Dashboard (welche Prozesse gezählt werden, welcher Pfad für die Festplattenbelegung geprüft wird).
- **Server-Neustart-Wartezeit**: Pause zwischen Stop und Start beim "Neustarten"-Knopf in der Server-Steuerung.

## Client

- **Client-Pfad**: der lokale Ordner deines Spiel-Clients - Grundlage für Item-Icons, Modul-Importer, Icon-Browser, 3D-Modell-Viewer, System-Installer (Client-Installationsdateien).
- **Client-Quellcode-Ordner (`binary_src_path`)**: dein lokaler Checkout des Client-C++-Quellcodes - nur für den System-Installer gebraucht, wenn ein Paket Client-Quellcode-Patches enthält.
- **NPC-Liste, EterPackConsoleLz4-Pfad**: für die 3D-NPC-Vorschau bzw. das Neu-Packen von `icon.epk` beim Anlegen/Bearbeiten von Items.

## Sonstiges

- **Datenbank-Backup-Ordner**: wo `mysqldump`-Sicherungen abgelegt werden.
- **Webhook-URL** (Discord o.ä.): für automatische Benachrichtigungen bei fehlgeschlagenen DB-Backups und wenn der Server-Prozess unerwartet verschwindet (nur solange M2Manager selbst offen ist, kein eigenständiger Server-Watchdog).
