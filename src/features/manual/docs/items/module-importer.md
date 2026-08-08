# Modul-Importer

Importiert fertige Waffen-/Rüstungs-/Zubehör-Pakete (wie man sie oft aus Community-Quellen bekommt) automatisch als neue Items, statt jede Datei von Hand an die richtige Stelle zu kopieren.

## Die drei Modi

- **Waffen/Rüstung mit 3D-Modell**: beliebige Ordnerstruktur wird gescannt, Subtyp per Dateinamen erkannt, Icon per Fuzzy-Matching gefunden, das `.gr2`-Modell wird nach `{vnum}.gr2` umbenannt. Bei Rüstung wird die weibliche 3D-Verknüpfung automatisch aus der Client-`.msm`-Datei übernommen - männlich bleibt manuell (dafür gibt es clientseitig keine datei-basierte Zuordnung).
- **Automatische Aufwertungs-Kette**: optional bekommt das importierte Item direkt eine ganze `+0` bis `+N`-Kette statt nur einer Stufe, inklusive passender `refine_proto`-Rezepte (Gold-Kosten/Chance nach Server-Vorlage, Materialien lassen sich danach im Aufwertungs-Editor ergänzen).
- **Icon-Item (kein 3D-Modell)**: für Zubehör ohne eigenes Modell (Schuhe, Ketten, Verbrauchsgegenstände) - ein Ordner voller loser Bilder, pro Bild ein Item mit frei wählbarem Typ/Subtyp/Werten.

## Wichtige Hinweise

- Jeder Import landet im **Verlauf** mit vollständigem Rückgängig-machen - inklusive nicht mehr benötigter `refine_proto`-Zeilen, aber nur, wenn wirklich kein anderes Item sie noch nutzt.
- Bekannte, bereits gefixte Fallstricke bei schlecht vorbereiteten Paketen: RLE-komprimierte TGA-Icons und Waffen mit absolutem (statt relativem) Textur-Pfad im Modell wurden früher unsichtbar/weiß im Client - beides wird jetzt automatisch korrigiert.
