# Willkommen im M2Manager-Handbuch

M2Manager bündelt die Werkzeuge, die für den täglichen Betrieb eines Metin2-Servers gebraucht werden - Datenbank-Editoren, Server-Steuerung, Datei-Werkzeuge für den Client - an einer Stelle, mit Backups vor jeder Änderung und, wo möglich, einem Rückgängig-machen-Verlauf.

## Wie dieses Handbuch aufgebaut ist

Links findest du dieselben Gruppen wie in der Sidebar der App:

- **Übersicht** - Dashboard, Server-Steuerung, Quellcode bauen & einspielen
- **Items & Ausrüstung** - Item Editor, Item-Übersicht, Modul-Importer, Aufwertungs-Editor, Cube-Editor
- **Shops & Monster** - Shop-Editor, Mob-Proto-Editor, Drop-Generator (inkl. Mob-Drops und Kisten)
- **Quests & Welt** - Quest Builder, Regen-Datei-Editor, Locale-Verwaltung
- **Server & Accounts** - Server-Events (inkl. Tag/Nacht & Schnee), Account-Verwaltung (inkl. GM-Verwaltung), Gilden-Verwaltung, Wirtschafts-Dashboard, Datenbank-Explorer, Broadcast-System
- **Backups** - Backups (Dateien + Datenbank), Änderungsprotokoll
- **Client-Assets** - TGA Konverter, Icon-Browser, 3D-Modell-Viewer
- **Einstellungen**

Am schnellsten kommst du zu einem Abschnitt über den **Hilfe-Knopf direkt im jeweiligen Modul** (kleines Fragezeichen-Symbol neben dem Titel) - das öffnet dieses Fenster (falls noch nicht offen) direkt auf der passenden Seite.

## Grundprinzipien, die für fast jedes Modul gelten

- **Backups vor jedem Schreiben.** Jeder Editor, der eine Datei überschreibt, legt vorher automatisch eine Sicherung an (lokal in einem `m2manager_backups`-Ordner neben der Originaldatei, oder auf dem Server daneben). Über den Bereich **Backups** lässt sich jede dieser Sicherungen wiederherstellen.
- **Manche Änderungen brauchen einen Server-Neustart.** Einige Server-Dateien werden nur beim Start eingelesen (z.B. Aufwertungs-Rezepte, Kisten-Inhalte, manche Server-Events) - eine Änderung über den Editor wirkt dann erst nach dem nächsten Neustart über die Server-Steuerung. Das steht jeweils im passenden Abschnitt.
- **Verbindung nötig.** Die meisten Module brauchen eine funktionierende SSH- und/oder MySQL-Verbindung (siehe Einstellungen) sowie einen konfigurierten Client-Pfad.
