# Shop-Editor

Bearbeitet NPC-Shops als Raster, genau wie es später im Client aussieht.

## Was du hier tun kannst

- NPC auswählen (mit 3D-Vorschau), dessen Shop-Inhalt als Raster bearbeiten - Items hinzufügen/entfernen, Mengen anpassen.
- Item-Suche/-Filter wie überall sonst im Programm, inklusive Icon-Anzeige.
- **Bestandsware synchronisieren**: setzt `shop_item.count` für den aktuellen Shop oder global für alle Shops auf einen Ziel-Maximalwert - praktisch, um viele Shops auf einmal aufzufüllen, mit Bestätigungsdialog (Bulk-Aktion).

## Wichtig zu wissen

- Ein Shop-Fenster im Client hat **maximal 40 Slots** - der Editor verhindert, mehr einzutragen.
- Änderungen wirken direkt über die Datenbank, kein Client-Repack und in der Regel kein Server-Neustart nötig.
