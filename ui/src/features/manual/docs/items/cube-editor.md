# Cube-Editor

Bearbeitet `cube.txt` - die Rezepte für das "Cube"-System (Verwandlung/Kombination an einem NPC).

## Was das Cube-System macht

Ein oder mehrere NPCs bieten ein Rezept an: der Spieler gibt bestimmte Material-Items (und optional Gold) hinein, mit einer festen Erfolgschance gibt es dafür eine Belohnung zurück.

## Was du hier tun kannst

- Rezepte anlegen/bearbeiten/löschen: anbietende NPCs, benötigte Materialien, Belohnung, Erfolgschance (%), Gold-Kosten.
- NPC-/Item-Auswahl über den bekannten Suchen-Picker (wie in Item Editor/Aufwertungs-Editor).

## Wichtig zu wissen

- Die Erfolgschance ist ein **direkter Prozentwert** (0-100), anders als beim Mob Drop Editor - dort wird ein anderer Wert real durch 4 geteilt (siehe dortiges Info-Popover). Beim Cube-System gibt es diese Umrechnung nicht.
- Wirkt **ohne Server-Neustart**: der Ingame-GM-Befehl `/reload c` lädt `cube.txt` sofort neu (im Quellcode als `Cube_init()` verifiziert) - anders als Kisten-Editor/Aufwertungs-Editor, die einen echten Neustart brauchen.
- Das genaue Datei-Format (Zeilenenden, Kommentare) wurde diese Session nur gegen den Server-Quellcode geprüft, **nicht** gegen eine echte `cube.txt`-Datei byte-verifiziert - beim ersten Speichern die Datei einmal gegenkontrollieren.
