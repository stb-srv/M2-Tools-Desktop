# Server-Events

Schaltet Server-Events (Doppel-Drop, Rate-Erhöhungen usw.) an/aus und stellt Prozentwerte ein.

## Zwei grundverschiedene Arten von Werten

- **Raten-Multiplikatoren** (Item-/EXP-/Gold-Drop, Schaden usw. + Premium-Varianten): unverändert = 100 = normal.
- **Saisonale Sonder-Drop-Flags** (17 Stück): unverändert = Event komplett **aus**. Je kleiner der eingetragene Wert, desto häufiger der Drop (Formel direkt aus dem Quellcode nachvollzogen). Manche haben zusätzliche fest einprogrammierte Bedingungen (z.B. Mindestlevel).

Der Tab erklärt bei jedem Wert, welche der beiden Logiken gilt - das ist real leicht zu verwechseln, da "ungesetzt" bei den beiden Kategorien genau das Gegenteil bedeutet.

## Wichtig zu wissen

- Event-Flags landen in derselben Datenbank-Tabelle wie Spieler-Quest-Fortschritt, werden aber **nur einmal beim Start** eingelesen - eine reine DB-Änderung hier wirkt erst nach einem **Server-Neustart**.
- Für sofortige Wirkung ohne Neustart bietet der Tab zu jedem Wert einen kopierbaren `eventflag`-GM-Befehl - der schreibt direkt in dieselbe Tabelle und wirkt sofort.
- Es gibt außerdem einen offenen Bereich, um beliebige weitere/eigene Flag-Namen zu setzen, falls dein Server eigene Events hat.
