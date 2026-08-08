# Mob Drop Editor

Bearbeitet `mob_drop_item.txt` direkt auf dem Server - welches Item ein Monster mit welcher Chance droppt.

## Was du hier tun kannst

- Drop-Einträge pro Mob bearbeiten, mit Item-Suche/-Icons wie im Shop-Editor.
- **Vier Bulk-Änderungsmodi** für Drop-Prozente: um einen Betrag ändern (Delta), auf einen festen Wert setzen, auf einen Zufallsbereich, oder ein bestimmtes Item überall ändern - jeweils global oder nur für den aktuellen Mob.
- **"Wer droppt…?"**: Reverse-Suche, welche Mobs ein bestimmtes Item überhaupt droppen.
- Zeigt zusätzlich "≈X% real" an - die tatsächliche Drop-Chance ist ungefähr der eingetragene Prozentwert geteilt durch 4 (aus dem echten Server-Quellcode nachgerechnet, nicht geschätzt).
- Zweiter Tab **"Lokale Datei prüfen/reparieren"**: prüft eine beliebige lokale Kopie der Datei auf Syntaxfehler und bietet bei Problemen eine Roh-Text-Ansicht zum Korrigieren an, bevor es im normalen Editor weitergeht.

## Wichtig zu wissen

- Jedes Speichern sichert vorher die Datei auf dem Server.
- Bei mehrfachen Testläufen (z.B. "wie oft droppt X bei 10 Kills") den GM-Befehl mit Anzahl nutzen (`/mob_kill` o.ä. mit mehreren Kills), sonst testet man nur eine einzelne Chance.
