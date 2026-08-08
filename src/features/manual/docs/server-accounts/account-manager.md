# Account-Verwaltung

Verwaltet Spieler-Accounts: durchblätterbare Liste, neue Accounts anlegen, Passwörter zurücksetzen, Items geben/entfernen.

## Was du hier tun kannst

- Accounts **suchen und auflisten** (Suchspalte einstellbar, Standard `login`).
- **Neue Accounts** direkt anlegen (Login + Passwort).
- **Passwort zurücksetzen** - Account-Passwörter sind ein MySQL-Einweg-Hash und können technisch nicht ausgelesen werden, nur neu gesetzt.
- Generisches Geben/Entfernen von Items über die `item`-Tabelle für den ausgewählten Account.

## Wichtig zu wissen

- Es gibt keine Verbindung zu einem laufenden Spielprozess - Änderungen wie "Item geben" wirken bei online befindlichen Spielern ggf. erst nach einer Neuanmeldung.
- Beim Item-Geben werden Fenster/Position nicht automatisch geraten - diese Felder müssen selbst ausgefüllt werden.
