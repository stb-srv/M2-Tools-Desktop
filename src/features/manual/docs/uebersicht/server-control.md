# Server-Steuerung

Startet/stoppt den Spielserver und die Channels, ohne dass du selbst per SSH das interaktive Menüskript (`index.sh`) bedienen musst.

## Funktionen

- **Server/Channel starten** und **stoppen** - läuft im Hintergrund über dieselbe interaktive `index.sh`, die du auch von Hand nutzen würdest; die Ausgabe wird live im Log-Bereich mitgeschrieben.
- **Neustarten** - führt Stop und Start automatisch nacheinander aus, mit einer konfigurierbaren Wartezeit dazwischen (Standard 5 Sekunden). Praktisch nach jeder Änderung, die einen Neustart braucht.
- **Quests neu laden** - lädt die Quest-Skripte neu, ohne den ganzen Server neu zu starten (derselbe Befehl wird auch vom "Kompilieren & Neuladen"-Knopf im Quest Builder genutzt).
- **Logs löschen**.

## Wichtige Hinweise

- Vor destruktiven Aktionen (Stoppen, Neustarten) erscheint ein Bestätigungsdialog.
- Ein echter Neustart ist der **einzige** Weg, Änderungen wirksam zu machen, die nur beim Server-Start eingelesen werden (Aufwertungs-Rezepte, Kisten-Inhalte, manche Server-Events, `special_item_group.txt`) - ein einfaches "Quests neu laden" reicht dafür nicht.
- Automatisierte Tests laufen bisher nur mit harmlosen Platzhalter-Befehlen - ein echter Start/Stop-Lauf wurde noch nicht automatisiert abgesichert, funktioniert aber im normalen Gebrauch.
