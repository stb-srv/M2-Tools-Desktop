# Backup-Browser

Durchsucht alle verstreuten `m2manager_backups`-Ordner (jeder Editor legt sein Backup direkt neben der Originaldatei an, es gibt keinen zentralen Speicherort) und stellt eine Sicherung an ihrem Ursprungsort wieder her.

## Was du hier tun kannst

- Backups durchsuchen und ansehen, egal von welchem Editor sie stammen.
- Vor dem Wiederherstellen den **Diff** ansehen - ein Zeilen-Vergleich zwischen Backup-Inhalt und dem aktuellen Stand der Zieldatei, auch für den Fall, dass die Zieldatei inzwischen gar nicht mehr existiert.
- Wiederherstellen - die aktuell dort liegende Datei wird dabei selbst zuerst gesichert, nicht einfach überschrieben.

## Wichtig zu wissen

Manuelle Änderungen außerhalb dieses Programms (z.B. direkt per SSH) hinterlassen **kein** Backup hier - das Sicherheitsnetz greift nur für Änderungen, die tatsächlich über M2Manager gelaufen sind.
