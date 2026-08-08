# Kisten-Editor

Bearbeitet `special_item_group.txt`, die Beute-Tabelle einer GIFTBOX (Kiste).

## Was du hier tun kannst

- Beute-Einträge (Item + Chance) einer Gruppe bearbeiten, hinzufügen, entfernen.
- Über den eingebauten **Item-Picker** direkt nach dem Kisten-Item suchen und es automatisch korrekt einrichten lassen (Typ `GIFTBOX` + Stapelbar-Flag setzen) - vorher musste man das von Hand im Item Editor machen.
- Eine **Live-VNUM-Prüfung** zeigt sofort Name/Typ zur eingetragenen Kisten-vnum inklusive Warnung bei falschem Typ oder unbekannter vnum.

## Wichtig zu wissen

- Die "verbleibende Anzahl" beim mehrfachen Öffnen einer Kiste ist **keine Extra-Funktion** - es ist schlicht die Stapelanzahl des Kisten-Items selbst (jedes Öffnen zieht 1 vom Stapel ab). Du brauchst also nur Typ `GIFTBOX` + "Stapelbar", keine zusätzliche Einstellung.
- Eine Gruppe **ohne** eigene `Type`-Zeile ("Normal") kann beim Öffnen **nie** leer ausgehen - nur Gruppen mit `Type pct` können "nichts" ergeben, weil dort jeder Eintrag unabhängig würfelt.
- **Braucht einen Server-Neustart**, um wirksam zu werden - genau wie `refine_proto` wird die Datei nur beim Start eingelesen, es gibt dafür kein `/reload`.
- Eine falsche Kisten-vnum führt zum selben stillen Fehlschlag wie ein falscher Typ (die Vergabe sucht rein über die vnum) - die eingebaute Live-Prüfung fängt genau das ab.
