# Quest Builder

Verwaltet die echten Quest-Skripte auf dem Server (Lua) - ansehen, bearbeiten, neu anlegen, ohne die Serverkonsole zu benutzen.

## Grundfunktionen

- Links Kategorie-/Datei-Browser, rechts ein Code-Editor mit Lua-Syntax-Hervorhebung. Speichern legt vorher ein Backup an, Löschen verschiebt die Datei nur ins Backup-Verzeichnis (kein Hard-Delete).
- **Volltextsuche** durchsucht den Inhalt aller Quest-Dateien, nicht nur Datei-/Kategorienamen.
- **Kompilieren & Neuladen** übernimmt und lädt die Quests direkt neu (derselbe Befehl wie "Quests neu laden" in der Server-Steuerung), mit Live-Ausgabe des Compilers.
- Eine durchsuchbare **Funktionsreferenz** aller Quest-Lua-Funktionen lässt sich per Klick direkt an der Cursor-Position einfügen.

## Drei Wege, eine neue Quest zu bauen

1. **Vorlagen** (Knopf "+"): NPC-Dialog+Belohnung, Sammel-Quest, Kill-Quest, Item-Benutzung, Instanz-Dungeon ("Run"), Sammel-Quest mit Erfolgschance - jeweils mit Item-/Mob-Auswahl statt Lua von Hand zu tippen.
2. **Mehrschritt-Baukasten**: verkettet mehrere der obigen Bausteine zu einer echten mehrstufigen Quest, inklusive optionalem "wiederholbar mit Cooldown in Tagen" und festen Bonus-Attributen als Belohnung.
3. **Freitext-Assistent**: eine normale Satzbeschreibung ("Rede mit Hans, sammle 10 Wolfsfelle, dann...") wird per Mustererkennung automatisch in den Mehrschritt-Baukasten vorausgefüllt - **keine echte KI**, reine Stichwort-/Satzmuster-Erkennung. Nicht erkannte NPCs/Items bekommen einen "Suchen"-Knopf zur manuellen Klärung.

## Wichtig zu wissen

- Die Vorlagen decken bewusst nur überschaubare Fälle ab - wirklich komplexe Quests bleiben Sache des Rohcode-Editors.
- Der Freitext-Assistent ist Mustererkennung, kein Sprachverständnis - grammatisch mehrdeutige Sätze (z.B. wer bekommt was, wenn Empfänger und Item nur durch ein Komma getrennt sind) werden nicht immer richtig aufgelöst. Der "Suchen"-Knopf ist genau dafür das Sicherheitsnetz.
- Über den Knopf **"Wiki öffnen"** gibt es zusätzlich eine komplette Referenz zur Metin2-Quest-Lua-Sprache selbst (Community-Dokumentation, separates Fenster) - für Fragen zur Sprache selbst, nicht zur Bedienung dieses Editors.
