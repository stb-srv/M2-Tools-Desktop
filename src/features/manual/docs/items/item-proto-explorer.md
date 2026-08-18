# Item-Proto-Explorer

Durchblättert die komplette `item_proto`-Tabelle als Übersichtsliste - mit Icon, Namen und einem lesbaren Typ-Label statt Rohwerten.

## Was du hier tun kannst

- Nach Name oder VNUM filtern, zusätzlich nach Item-Typ eingrenzen (Dropdown).
- Seitengröße wählen (20/50/100/200).
- **Alle Treffer exportieren**: exportiert nicht nur die sichtbare Seite, sondern alle Zeilen der aktuellen Suche/des Filters als CSV.
- Über "Im Item Editor öffnen" direkt in den Item Editor wechseln, um ein Item zu bearbeiten.

## Abgrenzung zu anderen Bereichen

- **DB Explorer** zeigt `item_proto` (und jede andere Tabelle) roh mit allen Spalten - dieser Bereich ist bewusst schmaler, dafür mit Icon/Typ-Label aufbereitet.
- Der **Item-Picker** innerhalb von Item Editor/Aufwertungs-Editor/Mob-Proto-Editor (`EntityBrowser`) ist eine eingebettete Auswahlkomponente ohne eigenen Navigationspunkt - dieser Bereich ist die eigenständige, immer erreichbare Vollansicht.
