# Drop-Generator

Bündelt alle 5 Drop-relevanten Server-Dateien in einem Tab-System. Zwei der Tabs (Mob-Drops, Item-Gruppen) sind die bereits bestehenden, eigenständigen Editoren, hier nur eingebettet - beide bleiben zusätzlich als eigene Sidebar-Punkte erreichbar.

Alle 5 Dateien liegen im selben Server-Ordner (Standard `/usr/home/game/share/`, in den Einstellungen pro Datei änderbar) und nutzen dieselbe Prozent-Kodierung: der Wert in der Datei ist **nicht** die reale Prozentzahl - die reale Chance ist ungefähr **Datei-Wert ÷ 4** (bei jedem Prozentfeld als "≈X%" angezeigt).

## 1. Mob-Drops (`mob_drop_item.txt`)

Der ursprüngliche, bereits bestehende Editor. Ein Item-Set **pro einzelnem Mob** - z.B. "Wildhund (VNUM 101) droppt Item A mit 0,5%, Item B mit 0,3%".

**Neu ergänzt:** anders als die meisten anderen Dateien hier ist diese live per GM-Befehl `/reload m` neu ladbar - nach dem Speichern reicht der Befehl, kein Server-Neustart nötig (siehe "i"-Info-Button im Tab).

## 2. Common-Drops (`common_drop_item.txt`)

**Korrigiert (2026-08-18)** nach einem echten Ladefehler: die erste Version dieses Tabs ging vom generischen Server-Quellcode aus (Rang-Level-Bracket-Format) - live gegen diesen Server geprüft, stimmt das aber nicht. Auf diesem Fork benutzt `common_drop_item.txt` tatsächlich **dasselbe Format wie Tab 1** (`Group`/`Mob`/`Type`/Item-Zeilen), pro Mob-VNUM - der Server-Quellcode-Checkout entspricht an dieser Stelle nicht dem, was auf diesem Server läuft.

In der Praxis also eine **zweite, separat gepflegte mob_drop_item.txt** - z.B. für Zusatz-Drops, die nicht in der Hauptdatei stehen sollen. Das echte Beispiel, das bereits auf dem Server liegt: drei Gruppen "MetinStein1" für die Mobs 8001/8002/8003, je mit Item 19 (100% Chance).

**Offen:** ob diese Datei wie Tab 1 per GM-Befehl live neu ladbar ist, wurde nicht geprüft - im Zweifel nach dem Speichern einen Server-Neustart einplanen.

## 3. Etc-Drops (`etc_drop_item.txt`)

Eine reine Item→Prozent-Tabelle, **ohne** Mob-Bezug in dieser Datei selbst. Welcher Mob welchen Eintrag benutzt, legt ein eigenes Feld im jeweiligen Mob-Proto fest (nicht Teil dieser Datei/dieses Editors).

**Echtes Beispiel** (auf Wunsch angelegt, da die Datei vorher leer war): `WA_7+1` (Wolfman-Ersatzrüstung +1, VNUM 21100) → 0,5%. Bedeutet: sobald irgendein Mob auf diesen Eintrag verweist, gilt für ihn diese Chance auf diese Rüstung.

**Besonderheit:** wird intern über den echten internen Item-Namen gespeichert, nicht über die VNUM - der Editor löst das automatisch auf, wenn ein Item über die Suche gewählt wird. **Vorsicht bei sehr alten/importierten Items:** manche `item_proto.name`-Werte auf diesem Server sind bereits beschädigt (z.B. VNUM 19 - ursprünglich vermutlich koreanischer Text, in der DB nur noch als Ersatzzeichen vorhanden) - für solche Items funktioniert die Namens-Auflösung nicht zuverlässig. Das ist ein bestehender Datenzustand, keine Fehlfunktion dieses Editors.

## 4. Item-Gruppen (`special_item_group.txt`)

Der bereits bestehende Kisten-Editor. Eine **Ein-Treffer-Ziehung aus einem Pool** - beim Öffnen einer Kiste (oder eines anderen Auslösers) wird genau ein Eintrag aus der Gruppe gezogen, gewichtet nach den eingetragenen Werten.

**Beispiel:** Gruppe "Waffentruhe" mit 3 Schwertern - beim Öffnen bekommt der Spieler genau eines der drei, nie mehrere gleichzeitig.

## 5. Zufalls-Gruppen (`drop_item_group.txt`)

Sieht auf den ersten Blick wie Tab 4 aus, funktioniert aber komplett anders: jede Gruppe gehört zu **einem bestimmten Mob** (über dessen VNUM), und **jeder Eintrag würfelt unabhängig** - keine Ein-Treffer-Ziehung.

**Echtes Beispiel** (auf Wunsch angelegt, da die Datei vorher leer war): Gruppe "Wildhund_Bonusdrop_Beispiel" für Mob 101 (Wildhund) mit Item 19 (Schwert+9) bei 0,5%. Bedeutet: bei jedem Wildhund-Kill wird dieser Eintrag unabhängig von allen Einträgen aus mob_drop_item.txt zusätzlich gewürfelt - kann zusätzlich zu den normalen Wildhund-Drops kommen, oder auch nicht.

## Wichtig zu wissen (gilt für Tabs 3 und 5)

- **Keine Live-Aktualisierung**: anders als Tab 1 (`/reload m`) gibt es für diese Dateien keinen GM-Befehl zum Neuladen - Änderungen wirken erst nach einem **kompletten Server-Neustart**.
- **Ein Fehler in diesen Dateien verhindert den kompletten Server-Start** (nicht nur eine einzelne Funktion) - deshalb erzwingen diese Editoren die Item-Auswahl ausschließlich über die eingebaute Suche, nie als frei getippte VNUM.
- Beide Dateien waren auf diesem Server bis 2026-08-18 leer - auf Nutzerwunsch wurde je ein einzelner, harmloser Test-Eintrag angelegt (siehe echte Beispiele oben), damit es beim ersten Ausprobieren etwas Reales zum Anschauen gibt, statt nur die konstruierten Erklärungen hier. Beide Dateien wurden vorher gesichert (`m2manager_backups/`). Das Grundformat stammt weiterhin aus dem generischen Server-Quellcode und wurde **nicht** byte-für-byte gegen eine echte, gewachsene Datei geprüft (anders als `mob_drop_item.txt`/`special_item_group.txt`) - nach dem Fund bei Common-Drops (Tab 2 wich vom generischen Quellcode ab, siehe oben) ist nicht auszuschließen, dass dieser Fork auch hier eigene Anpassungen hat. Der einzelne Test-Eintrag prüft nur, dass der Editor grundsätzlich mit echten Daten umgehen kann - kein Beleg, dass der Server ihn beim nächsten Neustart tatsächlich akzeptiert.
