# Mob-Proto-Editor

Bearbeitet bestehende Monster-Werte (`mob_proto`) - HP, Level, Schaden, Loot-Verhalten und mehr - über eine generische Zeilen-Ansicht.

## Was er macht

Jede Spalte wird roh mit einem typgerechten Eingabefeld angezeigt; die Primärschlüssel-Spalte wird automatisch aus der Datenbank erkannt statt "vnum" anzunehmen.

## Wichtig zu wissen

- **Nur Bearbeiten, kein Neuanlegen** - für `mob_proto` gibt es kein bekanntes Äquivalent zu `Mysql2Proto` für Items, ein sicherer Neuanlage-Weg ist nicht verifiziert.
- Änderungen wirken in der Regel erst nach einem **Server-Neustart**.
- Diese Tabelle wurde (anders als `item_proto`) nie live gegen einen laufenden Server durchgetestet - ein Blick auf die tatsächliche Auswirkung einer Änderung lohnt sich beim ersten Einsatz.
