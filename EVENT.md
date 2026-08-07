# Server-Events: wie sie funktionieren, und wie neue entstehen

Diese Datei beantwortet zwei Fragen, die im Rahmen des neuen "Server-Events"-Bereichs
(`src/features/server-events`) aufkamen:

1. Wie funktioniert das bestehende Event-System (Doppel-Drop, Doppel-EXP, saisonale
   Sonder-Drops), das der neue Tab bedienbar macht?
2. Wie (und wie schnell) könnte man ein wirklich **neues** Event erstellen - und wäre ein
   generischer "Event-Ersteller" ohne Programmieraufwand möglich?

Alles unten ist direkt aus dem echten Server-Quellcode verifiziert
(`C:\Users\DevSteven\Documents\OpenCode\Source\game-src`), nicht aus generischem
Metin2-Wissen ergänzt. Zeilenangaben können bei künftigen Quellcode-Änderungen abweichen -
im Zweifel neu nachschauen statt dieser Datei blind vertrauen.

## 1. Das bestehende Mechanismus: Event-Flags

Es gibt **keine feste Liste gültiger Event-Namen** im Quellcode - das System ist eine
generische `map<string, int>` auf `CQuestManager` (`source/game/src/questmanager.cpp`):
`GetEventFlag(name)` liest, `SetEventFlag(name, value)`/`RequestSetEventFlag(name, value)`
schreiben. Jeder beliebige String funktioniert als Name, ein nie gesetzter Name liefert
einfach `0` zurück.

**Zwei völlig unterschiedliche Verwendungsarten existieren nebeneinander**, das ist der
wichtigste Punkt zum Verständnis:

- **Raten-Multiplikatoren** (`mob_item`, `mob_exp`, `mob_gold`, `mob_gold_pct`, `mob_dam`,
  `user_dam` + 5 Premium-Varianten): `SetEventFlag` erkennt diese Namen namentlich und
  schreibt den Wert zusätzlich in ein eigenes Feld auf `CHARACTER_MANAGER`
  (`m_iMobItemRate` usw., `char_manager.cpp`), das **unabhängig vom Flag** einen eigenen
  Standardwert `100` hat. Ist der Flag nicht gesetzt, bleibt es bei 100% (normal) - **nicht**
  bei 0%. Diese Werte werden direkt in die Drop-/EXP-/Schadensberechnung multipliziert
  (`item_manager.cpp:685`, `char_battle.cpp:2073/505/555` usw.), jeweils `/ 100`.
- **Saisonale Sonder-Drops** (`valentine_drop`, `halloween_drop`, `mars_drop`, ... 17
  Stück, Liste siehe `ServerEvents.tsx`): gelesen über
  `GetDropPerKillPct(iMinimum, iDefault, iDeltaPercent, flagName)` (`item_manager.cpp:645`).
  Ist der Flag nicht gesetzt (`0`), liefert die Funktion `0` zurück - das Event ist dann
  **komplett aus**, nicht "Standard". Ist er gesetzt, aber unter `iMinimum` oder negativ,
  springt er auf `iDefault`. Die Formel `40000 * iDeltaPercent / iVal` bedeutet: **je
  kleiner der eingetragene Wert, desto häufiger der Drop** - das ist keine direkte
  Prozentangabe, eher eine "1-zu-N"-Stärke. Mehrere dieser Events haben zusätzlich
  fest im Quellcode verdrahtete Zusatzbedingungen (bestimmtes Level, Boss-Rang, ein
  bestimmtes Besitz-Item) - der Flag allein reicht dann nicht, siehe Hinweistexte im Tab.

**Persistenz:** beide Arten landen in derselben Tabelle wie normale Quest-Fortschritte
(`player.quest`, Zeile mit `dwPID = 0`), geladen **einmalig beim Start des DB-Prozesses**
(`ClientManagerEventFlag.cpp::LoadEventFlag`, aufgerufen aus `ClientManager.cpp:143` vor
der Hauptschleife) und danach live an alle verbundenen Game-Prozesse verteilt, sobald sich
etwas ändert. Eine Änderung direkt in der Datenbank (wie im neuen Tab) wirkt deshalb **erst
nach einem kompletten Server-Neustart** - ein laufender Prozess bemerkt eine reine
SQL-Änderung nie von selbst. Der Ingame-GM-Befehl `eventflag <name> <wert>`
(`cmd_gm.cpp:1666`, Berechtigung `GM_HIGH_WIZARD`) geht dagegen über eine Netzwerk-Nachricht
an den DB-Prozess, wirkt **sofort** auf allen Game-Prozessen und schreibt gleichzeitig in
dieselbe Tabelle - deshalb bietet der neue Tab zu jedem Wert einen kopierbaren
`eventflag ...`-Befehl als Sofort-Alternative zur reinen DB-Bearbeitung an.

## 2. Ein wirklich neues Event erstellen

**Kurze Antwort: die Raten-Multiplikatoren und die 17 saisonalen Sonder-Drops sind bereits
alle da, die man ohne Quellcode-Änderung nutzen kann - ein neuer Tab reicht dafür (schon
gebaut).** Ein *zusätzliches*, komplett neues Event (z.B. "Sommer-Drop" mit einem eigenen,
neuen Item) existiert dagegen nicht automatisch - dafür fehlt der Code-Block, der es
überhaupt erst mit einem konkreten Item/Mob verknüpft.

### Wie ein bestehendes Sonder-Drop-Event tatsächlich implementiert ist

Jedes der 17 Events in `item_manager.cpp` (Zeilen ~991-1270) ist ein eigener,
handgeschriebener Code-Block nach diesem Muster (reales Beispiel, `halloween_drop`):

```cpp
if (GetDropPerKillPct(100, 2000, iDeltaPercent, "halloween_drop") >= number(1, iRandRange))
{
    // Item(s) konkret bestimmen und droppen - hier typischerweise
    // ein Aufruf wie AutoGiveItem(vnum, count) oder ein Eintrag in
    // eine lokale Drop-Liste, je nach Event unterschiedlich gebaut.
}
```

Ein neues Event dieser Art zu bauen bedeutet: einen neuen `if`-Block nach exakt diesem
Muster hinzufügen (neuer eindeutiger Flag-Name, welches Item/welche Items droppen sollen,
ggf. zusätzliche Bedingungen wie bei den bestehenden Events), dann **neu kompilieren und
einspielen**.

### Aufwand/Geschwindigkeit-Einschätzung

Mit dem neuen "Server-Quellcode Bauen & Einspielen"-Werkzeug (siehe
[[m2manager_build_deploy]] bzw. `src/features/build-deploy`) ist der bisher mühsamste
Teil - Quellcode auf dem Live-Server neu bauen und die Programmdatei sicher austauschen -
bereits vollständig automatisiert (mit Backup, Live-Prüfung, Rückgängig-machen). Damit
reduziert sich der Aufwand für ein neues Sonder-Drop-Event auf:

1. Den neuen `if`-Block in `item_manager.cpp` schreiben (wenige Zeilen, folgt einem
   bestehenden Muster) - **das ist der einzige Teil, der noch echte C++-Kenntnis und eine
   Quellcode-Änderung braucht**, ca. 10-20 Minuten inklusive Testen der Syntax.
2. Mit dem Bauen-&-Einspielen-Werkzeug bauen (Arbeitskopie, ungefährlich) und einspielen
   (kurzer Ausfall aller Channels, mit den bereits gebauten Sicherheitsnetzen).
3. Das Event danach ganz normal über den neuen Server-Events-Tab (Flag setzen) oder den
   `eventflag`-GM-Befehl an-/ausschalten - keine weitere Quellcode-Änderung mehr nötig, um
   es später ein- oder auszuschalten oder die Stärke anzupassen.

**Realistische Gesamtzeit für ein neues, einfaches Sonder-Drop-Event (ein Item, eine
Chance, keine Sonderbedingungen): eine Stunde oder weniger**, sobald ein konkretes Item
und eine gewünschte Chance feststehen - der Großteil ist jetzt Werkzeug-Automatisierung,
nicht Handarbeit.

## 3. Wäre ein generischer "Event-Ersteller" (ganz ohne Quellcode) möglich?

Kommt darauf an, was genau "Event" bedeuten soll - drei realistische Stufen:

**Stufe 1 - bereits vollständig möglich, schon gebaut:** bestehende Raten-Multiplikatoren
und die 17 vorhandenen Sonder-Drop-Flags an-/ausschalten und in der Stärke verstellen. Das
ist exakt der neue "Server-Events"-Tab. Kein weiterer Aufwand nötig.

**Stufe 2 - realistisch machbar, einmaliger Aufwand (empfohlen als nächster Schritt, falls
gewünscht):** genau das Muster, das in dieser Sitzung schon einmal erfolgreich für die
Aufwertungs-Boost-Schriftrollen gebaut wurde (siehe [[m2manager_build_deploy]]) noch einmal
anwenden - **eine neue, generische "Sonder-Drop"-Funktion** in `item_manager.cpp`
hinzufügen, die statt eines fest verdrahteten Items eine **Konfiguration aus der Datenbank**
liest (z.B. eine neue Tabelle: Flag-Name, Item-VNUM, Anzahl, Chance-Formel-Werte). Danach
ließe sich per Datenbank/Tool **jedes beliebige neue "Item X droppt mit Flag Y an/aus"-Event
anlegen, ohne je wieder Quellcode anzufassen** - genau wie neue Aufwertungs-Schriftrollen
jetzt rein über den Item Editor entstehen. Aufwand: eine einmalige, überschaubare
Quellcode-Änderung (ähnlicher Umfang wie der Schriftrollen-Patch), danach nur noch
Datenbank-Arbeit. Aktuell **nicht gebaut** - wäre ein sinnvoller Folgeauftrag, falls
öfter neue Sonder-Drop-Events gewünscht sind.

**Stufe 3 - nicht realistisch als "ein Werkzeug für alles":** ein Event ist im Quellcode oft
mehr als nur "Item X droppt mit Chance Y" - visuelle Effekte (Schnee, Feuerwerk), neue
NPC-/Quest-Abläufe, Sonder-Regeln (wie das Kindertag-Level-30-Limit) sind jeweils eigene,
strukturell unterschiedliche Mechanismen im Code. Ein wirklich universeller
"Drag-&-Drop-Event-Baukasten", der JEDE denkbare Event-Art ohne Programmierung abdeckt,
würde bedeuten, praktisch jede dieser unterschiedlichen Mechaniken einzeln generisch zu
machen - das ist kein sinnvolles einmaliges Projekt, sondern viele einzelne (mögliche,
aber jeweils eigene) Erweiterungen. Realistischer Ansatz: Stufe 2 für den häufigsten Fall
(Sonder-Drops) bauen, alles andere weiterhin fallweise als eigene kleine
Quellcode-Änderung behandeln - dank des Bauen-&-Einspielen-Werkzeugs jetzt jeweils
schnell umsetzbar, siehe Abschnitt 2.
