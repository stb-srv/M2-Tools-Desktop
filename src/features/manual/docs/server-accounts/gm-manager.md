# GM-Verwaltung

Verwaltet `common.gmlist` - die Tabelle, aus der GM-Rechte kommen. Gedacht, um schneller einen neuen GM anzulegen, statt das manuell per SQL/DB Explorer zu machen.

## Was du hier tun kannst

- Bestehende GM-Einträge auflisten (Account, Charakter, Rang, Server-/Kontakt-IP).
- **Neuer GM**: Account (mit Such-Picker), Charaktername, Rang (Dropdown) und optional Server-/Kontakt-IP eintragen und direkt anlegen.
- Bestehenden Eintrag bearbeiten (generischer Zeilen-Editor, wie bei Mob-Proto-Editor) oder entfernen.

## Wichtig zu wissen

- Ein GM-Eintrag ist an einen **konkreten Charakternamen** gebunden, nicht nur an den Account - Groß-/Kleinschreibung muss exakt stimmen (`gm_new_get_level` sucht per Namens-Lookup).
- Wirkt nach dem Ingame-Befehl **`/reload a`** - kein Server-Neustart nötig (quellcode-verifiziert: `cmd_gm.cpp` löst darüber ein `HEADER_GD_RELOAD_ADMIN` zum DB-Kern aus).
- Server-IP wird vom DB-Kern per SQL gefiltert (nur `ALL` oder die passende Channel-IP werden überhaupt geladen) - `ALL` ist der sichere Standard, wenn der GM auf allen Channels gelten soll.
- Kontakt-IP wird gespeichert/geloggt, aber in der geprüften Rang-Prüfung (`gm_new_get_level`) nicht ausgewertet - kein Zugriffsschutz, nur Information.
- Diese Tabelle wurde diese Session nicht live gegen die echte Datenbank geprüft (nur gegen den Server-Quellcode) - das Modul prüft die Spalten beim Öffnen und zeigt eine klare Fehlermeldung statt zu raten, falls sie nicht wie erwartet aussehen.
