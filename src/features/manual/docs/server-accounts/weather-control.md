# Tag/Nacht & Schnee

Schaltet die serverweiten Wetter-/Zeit-Effekte Tag/Nacht und Schnee um - ohne den Server neu zu starten.

## Wie es funktioniert

Beide Schalter setzen echte Event-Flags des Spielservers (`day` und `xmas_snow`), dieselben, die auch die passenden GM-Befehle (`/eventflag day <0|1>`, `/xmas_snow <0|1>`) verändern. Ein Klick auf **Deploy & Neuladen** schreibt eine kleine generierte Quest-Datei (`Weather/Weather_System.lua`) und lädt sie live auf den Server - derselbe Mechanismus wie „Quests reloaden" in der Server-Steuerung.

Die Quest setzt die Flags einmalig beim nächsten Spieler-Login nach dem Deploy. Der Server selbst sendet danach jede Änderung sofort an **alle** bereits verbundenen Spieler, nicht nur an den, der gerade eingeloggt ist - und synchronisiert außerdem jeden künftigen Login automatisch mit dem aktuellen Stand, ganz ohne Zutun dieser Quest.

## GM-Befehle für sofortige Wirkung

Falls gerade niemand einloggt und die Änderung nicht auf den nächsten Login warten soll, zeigt das Modul die passenden GM-Befehle zum Kopieren an - dieselbe Wirkung, nur direkt im Spiel eingetippt (GM-Rang „Hoher Zauberer" oder höher nötig).

## Bewusste Einschränkungen

- **Regen gibt es nicht.** Weder Client- noch Server-Quellcode dieses Servers enthalten eine Regen-Funktion - das wäre eine echte Neuentwicklung (Partikeleffekt + Flag), kein Schalter, den man einfach aktivieren kann.
- **Kein echter Tag/Nacht-Zyklus.** Der `day`-Schalter kennt nur zwei Zustände (Tag/hell oder Nacht/dunkel erzwungen), keinen graduellen Sonnenauf-/-untergang.
- **Wirkung erst beim nächsten Login**, nicht sofort beim Klicken hier - siehe oben, dafür gibt es die GM-Befehle als sofortige Alternative.
- Ein reiner Datenbank-Eintrag für diese Flags (z. B. über einen SQL-Editor) hätte **keine** Wirkung auf einen bereits laufenden Server - die Werte werden nur beim eigenen Start der Datenbank in den Arbeitsspeicher geladen. Deshalb läuft das hier über eine Quest bzw. den GM-Befehl, nicht über einen direkten DB-Write.
