# Item-Übersicht

Durchblättert die komplette `item_proto`-Tabelle - wahlweise als **Liste** (Tabelle mit kleinem Icon, Typ-Label statt Rohwert) oder als **Kacheln** (großes Icon, VNUM und Name, zum visuellen Wiedererkennen eines Items).

## Was du hier tun kannst

- Nach Name oder VNUM filtern, zusätzlich nach Item-Typ eingrenzen (Dropdown).
- Oben zwischen **Liste** und **Kacheln** umschalten - die gewählte Ansicht wird gemerkt.
- Seitengröße wählen (24/48/96/192).
- **Alle Treffer exportieren**: exportiert nicht nur die sichtbare Seite, sondern alle Zeilen der aktuellen Suche/des Filters als CSV.
- **Verwendung prüfen** (Liste): zeigt, wo ein Item überall vorkommt (Shops, Drops, Rezepte …).
- Über "Im Item Editor öffnen" bzw. "Bearbeiten" direkt in den Item Editor wechseln.

## Abgrenzung zu anderen Bereichen

- **DB Explorer** zeigt `item_proto` (und jede andere Tabelle) roh mit allen Spalten - dieser Bereich ist bewusst schmaler, dafür mit Icon/Typ-Label aufbereitet.
- Der **Item-Picker** innerhalb von Item Editor/Aufwertungs-Editor/Mob-Proto-Editor (`EntityBrowser`) ist eine eingebettete Auswahlkomponente ohne eigenen Navigationspunkt - dieser Bereich ist die eigenständige, immer erreichbare Vollansicht.
- **Icon-Browser** durchsucht die rohen Icon-Dateien im Client-Ordner (nach Dateiname, nicht nach Item/VNUM) - nützlich, um ein Icon für ein neues Item auszuwählen, nicht um bestehende Items nachzuschlagen.
