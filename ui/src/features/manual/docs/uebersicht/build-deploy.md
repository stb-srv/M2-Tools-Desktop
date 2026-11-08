# Quellcode Bauen & Einspielen

Baut den Server-Quellcode (z.B. nach einer eigenen Code-Änderung) und spielt die fertige Programmdatei kontrolliert auf dem Live-Server ein.

## Wichtig zu wissen, bevor du das nutzt

Auf diesem Server zeigen **alle 4 Channels + der Login-Server per Symlink auf dieselbe eine `game`-Programmdatei** (ebenso `db`) - es gibt **keinen separaten Testserver**. Jede eingespielte Version betrifft sofort den kompletten Live-Server. Deshalb:

- Das **Bauen** läuft immer in einer separaten, mit dem Live-Quellbaum synchronisierten Arbeitskopie - dabei wird nichts am Live-Server verändert, das ist gefahrlos.
- Das **Einspielen** ist der kritische Schritt: davor musst du eine Bestätigung **eintippen** (nicht nur anklicken), das Tool sichert automatisch die aktuelle(n) Programmdatei(en), und prüft danach mit zwei zeitversetzten Prozess-Schnappschüssen, ob der Server wirklich stabil weiterläuft (nicht nur "irgendwas läuft").
- Jeder Einspiel-Vorgang landet im **Verlauf** mit Ein-Klick-Rückgängig-machen.

## Ablauf

1. **Quellcode synchronisieren** - kopiert den aktuellen Live-Quellcode in die Arbeitskopie.
2. **Bauen** - kompiliert dort, Ausgabe wird live angezeigt.
3. **Einspielen** - nur wenn der Build erfolgreich war; verlangt die eingetippte Bestätigung.

## Wichtiger Hinweis

Wenn du stattdessen manuell per SSH direkt im Live-Quellbaum baust (z.B. `gmake clean && gmake`), greift das Sicherheitsnetz dieses Tools **nicht** - kein automatisches Backup, kein Verlaufs-Eintrag. Nutze für Live-Änderungen immer diesen Bereich, nicht die manuelle Konsole.
