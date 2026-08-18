# Account-Verwaltung

Verwaltet Spieler-Accounts: durchblätterbare Liste, neue Accounts anlegen, Passwörter zurücksetzen, sperren/entsperren, Guthaben anpassen, Spieler entbuggen, Items geben/entfernen.

## Was du hier tun kannst

- Accounts **suchen und auflisten** (Suchspalte einstellbar, Standard `login`).
- **Neue Accounts** direkt anlegen (Login + Passwort).
- **Passwort zurücksetzen** - Account-Passwörter sind ein MySQL-Einweg-Hash und können technisch nicht ausgelesen werden, nur neu gesetzt.
- **Sperren/Entsperren** mit frei formulierter Nachricht und optionaler Dauer in Tagen (oder dauerhaft). Aktive Sperren mit verbleibender Zeit werden in einer eigenen Liste angezeigt, inklusive manuellem Vorzeitig-Entsperren.
- **Guthaben anpassen**: Der "Guthaben"-Button erscheint nur, wenn auf diesem Core tatsächlich eine passende numerische Zusatzwährungs-Spalte auf `account.account` existiert (wird beim Öffnen live geprüft, nichts wird geraten).
- Bei der **Spieler-Suche**: über den "Werkzeuge"-Button pro Treffer Yang gutschreiben/abziehen und die Position (Map/X/Y) setzen, um festhängende Spieler zu entbuggen.
- Generisches Geben/Entfernen von Items über die `item`-Tabelle für den ausgewählten Account - mit Item-Suche und Spieler-Suche als Ausfüllhilfe für die passenden Spalten.

## Wichtig zu wissen

- Es gibt keine Verbindung zu einem laufenden Spielprozess - Änderungen wie "Item geben" oder "Position setzen" wirken bei online befindlichen Spielern ggf. erst nach einer Neuanmeldung, das Entbuggen der Position wirkt generell nur, wenn der Spieler gerade offline ist.
- Beim Item-Geben werden Fenster/Position nicht automatisch geraten - diese Felder müssen selbst ausgefüllt werden. Finden die Item-/Spieler-Picker keine eindeutig passende Spalte, wird der Wert stattdessen in die Zwischenablage kopiert statt eine falsche Spalte zu raten.
- Die Sperr-Nachricht wird **wörtlich als Fehlermeldung am Login-Bildschirm** des Spielers angezeigt - es gibt serverseitig keinen festen Werte-Katalog, jeder Text außer `OK` sperrt.
- Zeitgesteuerte Entsperrung gibt es serverseitig nicht - M2Manager merkt sich das Ablaufdatum lokal und hebt fällige Sperren automatisch auf, sobald der Account-Manager geöffnet wird. Läuft die App zum Fälligkeitszeitpunkt nicht, wird erst beim nächsten Öffnen entsperrt - es gibt keinen Server-Cron dafür.
