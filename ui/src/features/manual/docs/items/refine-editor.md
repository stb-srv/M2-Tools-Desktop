# Aufwertungs-Editor (Refine)

Zeigt und bearbeitet die komplette Aufwertungs-Kette eines Items (alle Stufen, benötigte Materialien, Gold-Kosten, Erfolgschance) statt roher `refine_proto`-Zeilen.

## Was du hier tun kannst

- Eine bestehende Kette **ansehen**, inklusive "Woher bekomme ich das?"-Hinweisen zu jedem Material (Mob-Drops + NPC-Shops).
- Eine Kette **bearbeiten** oder ein Rezept **wiederverwenden**.
- Eine **neue** Kette anlegen.

## Wichtig zu wissen

- `item_proto.refine_set` ist nur eine **Rezept-ID**, keine Item-vnum - ein Rezept wird oft von mehreren Items gleichzeitig geteilt (z.B. nutzen 8 verschiedene Items dasselbe Rezept 1). Das bearbeitest du hier entsprechend als eigenständige Rezepte, nicht 1:1 pro Item.
- **Anders als fast jeder andere Editor gibt es hier keinen Client-Repack.** `refine_proto` wird vom Server nur beim Start eingelesen - eine Änderung wirkt erst nach einem echten **Server-Neustart** (Server-Steuerung).
- Noch nicht live im laufenden Client durchgetestet, nur gegen die echte Datenbank.
