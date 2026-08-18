# Server-Events

Schaltet Server-Events (Doppel-Drop, Rate-Erhöhungen usw.) an/aus und stellt Prozentwerte ein.

## Drei grundverschiedene Arten von Werten

- **Raten-Multiplikatoren** (Item-/EXP-/Gold-Drop, Schaden usw. + Premium-Varianten): unverändert = 100 = normal.
- **Saisonale Sonder-Drop-Flags** (17 Stück): unverändert = Event komplett **aus**. Je kleiner der eingetragene Wert, desto häufiger der Drop (Formel direkt aus dem Quellcode nachvollzogen). Manche haben zusätzliche fest einprogrammierte Bedingungen (z.B. Mindestlevel).
- **Sichtbare Events** (Schneefall, Musik, Event-Helfer-NPC, Weihnachtsbaum-Mob, Sonnenfinsternis): reines An/Aus (1/0), mit echten Spawn-/Sichtbarkeits-Effekten statt einer Drop-Chance - kein separates Event-System, sondern derselbe Flag-Mechanismus mit eigenen Hooks im Server-Quellcode.

Der Tab erklärt bei jedem Wert, welche der drei Logiken gilt - das ist real leicht zu verwechseln, da "ungesetzt" bei den Kategorien unterschiedliche Dinge bedeutet.

## Wichtig zu wissen

- Event-Flags landen in derselben Datenbank-Tabelle wie Spieler-Quest-Fortschritt, werden aber **nur einmal beim Start** eingelesen - eine reine DB-Änderung hier wirkt erst nach einem **Server-Neustart**.
- Für sofortige Wirkung ohne Neustart bietet der Tab zu jedem Wert einen kopierbaren `eventflag`-GM-Befehl - der schreibt direkt in dieselbe Tabelle und wirkt sofort.
- Es gibt außerdem einen offenen Bereich, um beliebige weitere/eigene Flag-Namen zu setzen, falls dein Server eigene Events hat.
