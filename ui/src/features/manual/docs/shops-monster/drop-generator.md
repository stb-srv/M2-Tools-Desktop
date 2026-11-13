# Drop-Generator

Bündelt alle 5 Drop-relevanten Server-Dateien in einem Tab-System. Zwei der Tabs (Mob-Drops, Item-Gruppen) sind die bereits bestehenden, eigenständigen Editoren, hier eingebettet und nur noch über diesen Bereich erreichbar.

Alle 5 Dateien liegen im selben Server-Ordner (Standard `/usr/home/game/share/`, in den Einstellungen pro Datei änderbar) und nutzen dieselbe Prozent-Kodierung: der Wert in der Datei ist **nicht** die reale Prozentzahl - die reale Chance ist ungefähr **Datei-Wert ÷ 4** (bei jedem Prozentfeld als "≈X%" angezeigt).

## Drop-Check

Ganz oben im Drop-Generator. „Jetzt prüfen“ liest alle 5 Dateien vom Server und prüft jeden Verweis gegen die Datenbank:

- **Fehler** (der Server startet nicht oder nimmt ein falsches Item): Item gibt es nicht in item_proto, eine Nummer, die der Server als internen Namen eines anderen Items liest (z.B. 1, 2, 14, 18, 24), ein unbekannter interner Name in etc_drop_item.txt, eine Datei, die sich nicht lesen lässt.
- **Hinweise**: Mob gibt es nicht in mob_proto (die Drops wirken dann nie), Kisten-Item einer Kisten-Gruppe fehlt.

Die Editoren speichern eine Datei mit solchen Fehlern nicht mehr. Der Speichern-Dialog zeigt außerdem, welche Zeilen sich gegenüber der Datei auf dem Server ändern. Bei Dateien, die erst nach einem Neustart wirken, erscheint nach dem Speichern „Server jetzt neu starten“.

## 1. Mob-Drops (`mob_drop_item.txt`)

Der ursprüngliche, bereits bestehende Editor. Ein Item-Set **pro einzelnem Mob** - z.B. "Wildhund (VNUM 101) droppt Item A mit 0,5%, Item B mit 0,3%".

**Neu ergänzt:** anders als die meisten anderen Dateien hier ist diese live per GM-Befehl `/reload m` neu ladbar - nach dem Speichern reicht der Befehl, kein Server-Neustart nötig (siehe "i"-Info-Button im Tab).

## 2. Common-Drops (`common_drop_item.txt`)

Zusatz-Drops **nach Mob-Rang** statt pro Mob. Jede Zeile der Datei ist eine Regel mit 4 Blöcken, einem pro Rang (Spalte `RANK` in mob_proto):

| Block | Rang | Beispiele |
|---|---|---|
| 1 | PAWN | Wildhund, Wolf, Alpha-Wolf |
| 2 | S_PAWN | Roter Keiler, Dryade |
| 3 | KNIGHT | Verfl. Wolf, Goldener Frosch |
| 4 | S_KNIGHT | Affenanführer, Böser Höllengeneral |

Bosse (BOSS, KING) stehen nicht in dieser Datei, für sie gibt es die Mob-Drops (Tab 1).

Jeder Block hat 6 TAB-getrennte Felder: **Name** (nur eine Notiz, der Server liest sie nicht), **LevelVon**, **LevelBis** (Level des Spielers mit dem meisten Schaden, nicht des Mobs), **Chance**, **VNUM** und **Anzahl** (wird gelesen, aber nicht benutzt, es dropt immer 1 Stück).

**So arbeitest du im Editor**
- Links stehen alle Regeln mit Item, Level-Bereich und den 4 Rängen als Chips (farbig = aktiv, durchgestrichen = aus). Suche nach Item, Notiz oder VNUM, Filter nach Rang.
- Rechts bearbeitest du eine Regel: Item (über die Suche, nie frei getippt), Notiz und darunter eine Karte pro Rang mit Level von/bis, Chance und dem Schalter „aktiv“. „Pro Rang ein eigenes Item“ erlaubt unterschiedliche Items je Rang.
- Die **Chance gibst du in echten Prozent ein**. In der Datei steht der Wert mal 4 (1 % = 4, 100 % = 400). Daneben siehst du den Datei-Wert und „≈ jeder X. Mob“. Das gilt bei gleichem Level von Spieler und Mob; ist der Spieler höher, sinkt die Chance.
- Unten steht die Zeile genau so, wie sie in die Datei geschrieben wird (→ = TAB).
- Kommentarzeilen (`#…`) und Leerzeilen bleiben unverändert an ihrer Stelle. Neue Regeln kommen ans Ende.

**Was geprüft wird.** Fehler sperren das Speichern, Hinweise nicht.
- Fehler: Zeilen mit falscher Feldanzahl (fehlender oder doppelter TAB), Komma statt Punkt, VNUM gibt es nicht in item_proto, VNUM 1, 2, 14, 18 oder 24 (der Server verwechselt sie mit den Goldringen), eine VNUM, die der Server als Namen eines anderen Items liest, Chance über 400, LevelBis 0 oder kleiner als LevelVon bei einem aktiven Block.
- Hinweise: Regel ist komplett aus, die Ränge einer Regel haben unterschiedliche Level-Bereiche.
- Fehlerhafte Zeilen zeigt die Liste rot mit Zeilennummer. Du kannst sie direkt korrigieren oder löschen.

**Nach dem Speichern** (mit Backup) braucht der Server einen **Neustart**, ein Live-Neuladen gibt es für diese Datei nicht.

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
- Beide Dateien waren auf diesem Server bis 2026-08-18 leer - auf Nutzerwunsch wurde je ein einzelner, harmloser Test-Eintrag angelegt (siehe echte Beispiele oben), damit es beim ersten Ausprobieren etwas Reales zum Anschauen gibt, statt nur die konstruierten Erklärungen hier. Beide Dateien wurden vorher gesichert (`m2manager_backups/`). Das Grundformat stammt weiterhin aus dem generischen Server-Quellcode und wurde **nicht** byte-für-byte gegen eine echte, gewachsene Datei geprüft (anders als `mob_drop_item.txt`/`special_item_group.txt`/`common_drop_item.txt`). Der einzelne Test-Eintrag prüft nur, dass der Editor grundsätzlich mit echten Daten umgehen kann - kein Beleg, dass der Server ihn beim nächsten Neustart tatsächlich akzeptiert.
