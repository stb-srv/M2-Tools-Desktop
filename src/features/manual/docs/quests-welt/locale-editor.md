# Locale-Verwaltung

Bearbeitet `share/translate.lua` - die Textbausteine, auf die Quests über `gameforge.questname._key` verweisen (z.B. für Missionstext, der nicht direkt im Quest-Skript steht).

## Was du hier tun kannst

- Bestehende Namespaces/Keys bearbeiten, neue hinzufügen oder entfernen.
- Der `[ENTER]`-Marker aus der Datei wird wie im Quest Builder als echter Zeilenumbruch im Textfeld angezeigt, nicht als Rohtext.

## Wichtig zu wissen

- Es wird **nicht** die ganze ~8800-Zeilen-Datei neu geschrieben - nur die Zeilen des bearbeiteten Namespace werden per Präfix-Abgleich ersetzt, der Rest bleibt byte-identisch erhalten. Das minimiert das Risiko, versehentlich woanders etwas kaputt zu machen.
- Das Namensschema (`gameforge.questname._key`) ist die offizielle Konvention und direkt in der Datei selbst so dokumentiert.
