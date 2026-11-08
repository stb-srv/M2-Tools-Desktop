# Wirtschafts-Dashboard

Eine Momentaufnahme der Server-Wirtschaft. Reine Lesewerte - hier wird nichts verändert.

## Was du hier siehst

- **Spieler**: Anzahl der Charaktere.
- **Gold im Umlauf**: Summe über das Gold aller Charaktere (`player.player.gold`).
- **Gelistete Shop-Items**: wie viele Items tatsächlich in NPC-Shops stehen.
- **Ø / Min. / Max. Shop-Preis**: Preisspanne über alle in Shops gelisteten Items (`item_proto.gold`, der NPC-Verkaufspreis).
- **Top 20 Gold-Bestände**: die Charaktere mit dem meisten Gold - nützlich, um auffällige Ausreißer (z.B. durch einen Bug oder Dupe) schnell zu erkennen.

## Wichtig zu wissen

- Die Werte werden beim Öffnen geladen; mit **Neu laden** holst du den aktuellen Stand.
- Gezählt wird nur die Spalte `player.player.gold` - Gold, das woanders gespeichert ist, taucht hier nicht auf.
