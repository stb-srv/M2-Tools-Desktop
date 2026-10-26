# Einstellungen

Alles, was die anderen Module brauchen, an einer Stelle - links nach Bereichen gegliedert. Textfelder speichern beim Verlassen des Feldes (oder mit Enter), jede Speicherung wird kurz unten rechts bestätigt.

## Allgemein

- **Darstellung**: Design (Hell/Dunkel/System) und Sprache.
- **Item Editor**: der vnum-Startwert, ab dem der Item Editor neue Items vorschlägt.

## Verbindung

- **Server-Profile**: jedes Profil merkt sich SSH- und MySQL-Zugang inklusive Passwörter (sicher im Windows Credential Manager, nicht im Klartext). „Aktivieren“ wechselt den Server, „Neues Profil“ legt einen weiteren an (erst Name, dann Zugangsdaten). Den Server wechseln kannst du genauso direkt auf der Startseite.
- **SSH-Zugang** des aktiven Profils: Host/Port/Benutzername + Passwort oder privater Schlüssel - für alles, was direkt auf dem Server liest/schreibt (Quest Builder, Regen-Editor, Mob Drop Editor, Server-Steuerung, Build & Deploy).
- **MySQL-Zugang** des aktiven Profils: für alle Datenbank-Editoren (Item-, Shop-, Mob-Proto-Editor, Account-Verwaltung, Datenbank-Explorer, Aufwertungs-Editor, Statistiken auf der Startseite).
- „Testen & speichern“ prüft die Verbindung zuerst und übernimmt die Werte nur, wenn sie klappt - auch ins aktive Profil.

## Server-Pfade

Wo die Dateien auf dem Gameserver liegen, nach Zweck gruppiert:

- **Drop-Dateien**: `mob_drop_item.txt`, `common_drop_item.txt`, `etc_drop_item.txt`, `drop_item_group.txt`, `special_item_group.txt`, `cube.txt`.
- **Quests & Welt**: Quest-Ordner, Basisordner für Regen-Dateien, `translate.lua`.
- **Server-Überwachung**: Prozessnamen und Festplatten-Pfad für die Startseite.
- **Datenbank-Backups**: Backup-Ordner auf dem Server und die zu sichernden Datenbanken.

## Client & Werkzeuge

- **Client-Ordner**: der lokale Ordner deines Spiel-Clients - Grundlage für Item-Icons, Modul-Importer, Icon-Browser, 3D-Modell-Viewer.
- **NPC-Liste**: nur nötig, wenn `npclist.txt` nicht automatisch gefunden wird.
- **EterPackConsoleLz4.exe** und **Mysql2Proto-Ordner**: externe Werkzeuge, die der Item Editor beim Anlegen/Bearbeiten von Items aufruft.

## Benachrichtigungen

- **Webhook-URL** (Discord o.ä.): meldet fehlgeschlagene DB-Backups und wenn der Server-Prozess unerwartet verschwindet (nur solange M2Manager selbst offen ist, kein eigenständiger Server-Watchdog).

## Updates

- Aktuelle Version, „Nach Updates suchen“ und Installation neuer Versionen.
