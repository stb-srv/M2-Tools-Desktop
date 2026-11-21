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

---

## Gesammelt am 2026-08-14

**Alle 5 Ideen unten wurden noch am selben Tag umgesetzt** — Details siehe `STATUS.md` ("Alle 5 Ideen aus einer neuen Ideen-Session umgesetzt"). Diese Datei bleibt als historische Ideensammlung erhalten.

## Konfigurierbare/höhere Seitengröße im EntityBrowser
Nutzerkritik: aktuell sieht man pro Seite nur wenige Treffer (`EntityBrowser.tsx`s `PAGE_SIZE = 20`, vom Nutzer als "maximal 10" wahrgenommen — tatsächlicher Wert noch zu verifizieren gegen das, was live im UI ankommt). Wunsch: einstellbar (z.B. Dropdown 20/50/100) statt fest verdrahtet. Betrifft Item Editor, Aufwertungs-Editor, Mob-Proto-Editor (alle nutzen den geteilten `EntityBrowser`).

## Export-Funktion für Suchergebnisse
Beispiel-Anwendungsfall des Nutzers: im Mob Drop Editor nach "Metin" suchen und sich alle Treffer (alle Mobs, deren Drops das gesuchte Item enthalten, bzw. alle Mobs mit "Metin" im Namen) exportieren lassen. Vermutlich CSV/JSON-Export einer aktuell angezeigten (gefilterten) Ergebnisliste — noch zu klären, für welche Module das zuerst gebaut werden soll (Mob Drop Editor, EntityBrowser, DB Explorer sind Kandidaten).

## Item-Proto-Explorer
Ein durchblätterbarer/durchsuchbarer Überblick über die komplette `item_proto`-Tabelle (ähnlich DB Explorer, aber item-spezifisch aufbereitet — vermutlich mit Icons/Namen/Typ statt Rohspalten). Abgrenzung zum bereits vorhandenen DB Explorer und EntityBrowser noch zu klären.

## Cube.txt-Editor ("Verwandlung"/Kombinations-System)
Liegt laut Nutzer auf dem Server. Format/Speicherort (Datei vs. DB-Tabelle) und Boot-Time-only-vs.-reload-fähig sind noch nicht gegen den echten Server-Quellcode verifiziert (Recherche läuft) — ähnliches Muster wie beim Kisten-Editor/Aufwertungs-Editor, wo die reale Mechanik erst im Quellcode nachgewiesen wurde, bevor gebaut wurde.

## Schnelleres GM-Anlegen
Modul, um neue GM-Accounts schneller anzulegen als über die bestehende Account-Verwaltung. Wie GM-Rechte auf diesem Server tatsächlich vergeben werden (Spalte in `account`, oder eine separate serverseitige Konfigurationsdatei mit Account-Namen+Level) ist noch nicht verifiziert (Recherche läuft) — sensibler Bereich, da er Berechtigungen vergibt, ähnlich vorsichtig zu behandeln wie die bestehende Account-Verwaltung.
