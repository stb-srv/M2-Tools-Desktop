Die Idee ist es eine Desktop Anwendung zu erstellen die sich mit der Datenbank, Gameserver, ssh sftp mit einem Metin2 Server verbinden kann und man darüber dann viele dinge direkt erledigen kann ohne viele weitere tools zu benötigen.

1. SSH/SFTP Verbindung Port 22 oder andere, Nutzer gibt IP, Nutzername und Passwortt an Verbindung wird getestet, wenn erfolgreich, kann der Nutzer den Pfad zum Starten/Stopen/Quest Reload angeben zusätzlich soll man dann auf auf gewisse Ordner und strukturen zugreifen können, wie Quest und weitere, kommt in den nächsten schritten dazu
    Beispiel Command für meine Entwicklungsumgebung: cd /usr/home/game && sh index.sh
        Was moechtest du tun?

            (1)     Server/Channel starten
            (2)     Server/Channel schliessen
            (3)     Logs loeschen
            (4)     Quests reloaden
            (5)     Nichts

2. Datenbank Verbindung verbindung einstellungen wie bei SSH nur dann Port 3306 oder andere Nutzername und Passwort, analysie der aktuellen datenbank struktur und automatischen mappen der tabellen -> hier fehlen noch viele Informationen diese will ich ggfs. mit Dir hinzufügen und als Basis wird meine Entwicklungs Server genommen, ggfs. Einstellbaren Variabelen für Datenbankennamen etc.



Feature	die Implementiert werden sollen
Shop Editor (Ingame Shops, Darstellung der Shops wie sie im Client sind ggfs. Client Datenabrufen fürs aufbauen)
3D Model Viewer
Connection Manager
Path Configuration
Multi-Language
Auto-Updates -> Via Github wenn möglich ansonsten anders
Dark/Light Theme
Role-Based Access Control
Team Management
Advanced Dashboard
