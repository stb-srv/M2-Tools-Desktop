# Regen-Datei-Editor

Bearbeitet die Monster-Spawn-Dateien (`*_regen.txt`), auf die z.B. die Dungeon-Etagen-Vorlage im Quest Builder verweist.

## Was du hier tun kannst

- Per SFTP-Ordnerbrowser die passende Regen-Datei finden und öffnen.
- Jede Zeile strukturiert bearbeiten - normale Spawn-Punkte und Ausnahmezonen (Sonderfall `e`) als eigene Felder statt als rohe Textzeile. Kommentar- und Leerzeilen bleiben unverändert erhalten.

## Wichtig zu wissen

- Format ist 1:1 aus dem echten Server-Quellcode übernommen (`regen.cpp`), nicht angenommen: `<typ> x y radius_x radius_y z_sektion richtung zeit prozent max_anzahl vnum`.
- Wie beim Quest-Skript-Editor: Speichern legt vorher ein Backup auf dem Server an.
- Die neue Einstellung `regen_base_dir` bestimmt, wo der Ordnerbrowser startet.
