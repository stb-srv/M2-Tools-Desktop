# Dashboard

Die Startseite nach dem Öffnen von M2Manager.

## Was du hier siehst

- **Server-Auswahl** ganz oben: wählt das aktive Server-Profil (z.B. Live- und Testserver). Ein Wechsel verbindet SSH und MySQL sofort neu und lädt die ganze Startseite neu.
- **Neues Profil**: legt einen weiteren Server an - erst nur der Name, danach SSH- und MySQL-Zugang (jeweils mit „Verbindung testen“). Das neue Profil ist danach sofort aktiv. „Bearbeiten“ springt zu Einstellungen → Verbindung.
- **Verbindungsstatus** für SSH und MySQL, mit direktem "Einrichten"-Link zu Einstellungen → Verbindung, falls etwas fehlt.
- **Server-Übersicht** (sobald SSH verbunden ist): konfigurierte SSH-Adresse, RAM- und Festplattenbelegung des Servers, sowie eine Prozessliste der laufenden Server-Prozesse (Ressourcen-Monitoring, gefiltert nach den in den Einstellungen konfigurierten Prozessnamen).
- **Datenbank-Statistiken**: Anzahl Accounts, Spieler, Items, Shops, Mobs - direkt aus der Live-DB, über den "Aktualisieren"-Knopf jederzeit neu abgerufen.
- **Schnellzugriff**-Kacheln zu den am häufigsten gebrauchten Bereichen.

## Hinweis

Wenn hier "Nicht verbunden" steht, prüfe zuerst die Einstellungen (SSH-Zugangsdaten, MySQL-Zugangsdaten) - fast alle anderen Module brauchen mindestens eine der beiden Verbindungen.
