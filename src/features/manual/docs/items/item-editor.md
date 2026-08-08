# Item Editor

Legt neue Items an und bearbeitet bestehende - Werte, Flags, Icon und (optional) Beschreibungstexte, alles über eine Oberfläche statt roher `item_proto`-Zeilen.

## Was er macht

- **Neu anlegen** oder **Bestehendes bearbeiten**: schreibt in `item_proto`, inklusive aller Flag-/Typ-Dropdowns (Typ, Subtyp, Trageort, Anti-/Wear-/Immun-Flags, Bonus-Attribute) - direkt aus dem echten Server-Quellcode abgeleitet, nicht geraten.
- **Icon**: wird beim Anlegen/Bearbeiten automatisch nach `pack/icon/icon/item/<vnum>.tga` geschrieben, `icon.epk` wird danach automatisch neu gepackt.
- **item_proto neu erzeugen & einspielen**: läuft automatisch über die externen Tools `EterPackConsoleLz4.exe`/`Mysql2Proto.exe` (Pfade in den Einstellungen), inklusive Backup vor jedem Überschreiben und automatischem Rollback bei Fehlschlag.
- **Als neues Item duplizieren**: im Bearbeiten-Modus holt das direkt die nächste freie vnum und übernimmt die aktuellen Werte als Vorlage - nur das Icon muss neu gewählt werden.
- **Beschreibung/Kurzbeschreibung**: das ist **keine** `item_proto`-Spalte, sondern reiner Client-Text (`itemdesc.txt`) - wird hier trotzdem mit bearbeitet, rein lokal, kein Server-Neustart nötig.

## Wichtige Hinweise

- Waffen/Rüstungen mit einem **neuen 3D-Modell** gehören in den **Modul-Importer**, nicht hierher - der Item Editor selbst kann kein `.gr2`-Modell zuweisen.
- value0-5 haben für Waffen/Rüstungen echte, verifizierte Bedeutungen (Schaden, Verteidigung, Angriffstempo, Zusatzwerte) mit eigenen Feldnamen und einer Live-Vorschau. Für alle anderen Typen bleiben es rohe `value`-Felder - dort ist "Referenz-Item übernehmen" der zuverlässigste Weg.
- Nach dem Speichern über "Neu erzeugen & einspielen" ist die Änderung sofort im Client sichtbar (kein Server-Neustart nötig), vorausgesetzt der Client wird wirklich neu gestartet (nicht nur neu eingeloggt).
