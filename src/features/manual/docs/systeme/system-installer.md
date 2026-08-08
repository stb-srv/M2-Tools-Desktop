# System-Installer

Baut fertige Community-"Systeme" (Server-/Client-Erweiterungen wie ein Fenster-Größenänderungs-Patch oder ein Admin-Panel-Modul) automatisiert ein, statt jede Datei von Hand mit dem echten Quellcode abzugleichen.

## Wie so ein System-Paket aufgebaut ist

Diese Pakete liefern **keine vollständigen Dateien**, sondern kleine Schnipsel mit eingebetteten Anweisungen (`// search` gefolgt vom vorhandenen Code, dann `// add above/below/inside/at the end` oder `// replace` gefolgt vom neuen Code) - der Installer erkennt diese Konvention automatisch, inklusive mehrerer Schreibvarianten und Tippfehler-Toleranz.

## Ablauf

1. **System-Ordner wählen** - der komplette Paketordner wird eingelesen und automatisch klassifiziert (Server-Quellcode / Client-Quellcode / Client-Installationsdatei).
2. Für jede Datei wird automatisch nach der echten Zieldatei gesucht (per Dateiname) - bei eindeutigem Treffer direkt übernommen, bei mehreren Treffern oder keinem Treffer kannst du den Zielpfad manuell setzen oder erneut suchen.
3. **"Bereite Blöcke anwenden"** schreibt alle sicher erkannten Änderungen in einem Rutsch, mit Backup vor jeder Datei und einem Verlaufs-Eintrag zum Ein-Klick-Rückgängig-machen.
4. Alles, was nicht automatisch sicher genug ist (Freitext-Anweisungen, mehrdeutige oder fehlende Treffer), bleibt sichtbar zur manuellen Klärung stehen - wird **nie** geraten.

## Wichtige Einstellungen

- **Server-Quellcode-Patches** gehen live per SSH auf den Server.
- **Client-Quellcode-Patches** brauchen die Einstellung `binary_src_path` (dein lokaler Client-Quellcode-Checkout).
- **Client-Installationsdateien** (Python, Assets) nutzen den bereits vorhandenen Client-Pfad.

## Wichtig zu wissen

- Kein Client-Neu-Kompilieren inklusive - das Tool spart nur das Suchen/Einfügen, das Bauen des Clients bleibt manuell.
- Es gibt (bewusst noch) keinen vollständigen, mehrsprachigen Syntax-Checker mit Auto-Reparatur - nur eine eingebaute Klammern-/`#if`-`#endif`-Balance-Prüfung als Sicherheitsnetz nach dem Einbau.
