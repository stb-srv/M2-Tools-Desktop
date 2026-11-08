# Gilden-Verwaltung

Bearbeitet die Gilden-Tabellen direkt in der Datenbank - generisch wie Mob-Proto-Editor und Account-Verwaltung: jede Spalte wird roh angezeigt, ohne geratene Bedeutung.

## Was du hier tun kannst

- **Gilde suchen** und als Tabelle durchsehen.
- **Bearbeiten**: öffnet die Gilden-Zeile im allgemeinen Zeilen-Editor (Spalte für Spalte).
- **Kriege anzeigen** (Schwerter-Symbol): listet alle Gildenkriege, an denen die Gilde beteiligt ist.
- **Auflösen** (Papierkorb-Symbol): führt dieselbe Kaskade aus wie der echte Server beim Auflösen einer Gilde - löscht die Gilde, ihre Rangstufen, Mitglieder und Kommentare und setzt für jedes ehemalige Mitglied dieselbe Beitritts-Sperre wie der Server. Gildenkriege werden dabei bewusst nicht angefasst (der echte Server räumt die ebenfalls nicht auf).

## Wichtig zu wissen

- **Auflösen ist endgültig**: es gibt kein Backup und kein Rückgängig-machen, die Zeilen werden direkt in der Live-Datenbank gelöscht.
- Oben lässt sich die **Datenbank** einstellen, in der die Gilden-Tabellen liegen (Standard `player`). Das ist noch nicht gegen einen echten Server geprüft - falls nichts gefunden wird, hier zuerst nachsehen.
- Änderungen an Gilden, deren Mitglieder gerade online sind, wirken unter Umständen erst nach einem erneuten Login.
