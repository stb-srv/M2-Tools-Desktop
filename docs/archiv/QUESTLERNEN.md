Inhaltsverzeichnis:
- 1. Begrüßung / Einleitung
- 2. Kapitel 1
- 2.1. Grundgerüst
- 2.2. States
- 2.3. Erste Aufgabe
- 3. Kapitel 2
- 3.1. Abfragen
- 3.2. Bedingungen
- 3.3. Zweite Aufgabe
- 4. Kapitel 3
- 4.1. Questfiles
- 4.2. Eventflags
- 4.3. Dritte Aufgabe
- 5. Kapitel 4
- 5.1. Neue Befehle
- 5.2. Eigene Funktion
- 5.3. Funktionszuteilung
- 5.4. Globale Variablen
- 5.5. Vierte Aufgabe
- 6. Kapitel 5
- 6.1. Schleifen
- 7. Kapitel 6
- 7.1. Native Funktionen
- 7.1.1. os.execute
- 7.1.2. os.date
- 7.1.3. string.gsub
- 8. Kapitel 7
- 8.1. Tabellen
- 9. Kapitel 8
- 9.1. return
- 9.2. Erweiterte Funktionen
- 10. Kapitel 9
- 10.1. Dungeons
- 10.1. Dungeons - Unique
- 11. Kapitel 10
- 11.1. Verschachtelte Funktionen
- 11.2. Abkürzungen



Schönen Tag miteinander!

ich werde euch hier nun Schritt für Schritt erklären, wie ihr am einfachsten lernt, Quests selbst zu schreiben. Es ist im Grunde ganz einfach und vieles wird einfach durch reine Logik klar, aber dazu kommen wir erst alles noch. Ich werde in diesem Thread alle Kapitel nacheinander posten und so ein Tutorial erstellen.

Zu beginn legen wir einmal die Definition der Quests fest:
Quests sind Dialoge oder bestimmte Effekte, die erzielt werden, wenn bestimmte Bedingungen erfüllt werden. Beispielsweise ist eine Quest, die man so in Metin2 schreibt nicht einfach nur eine "jage das oder das"-Quest, sondern es können auch Events damit gemanaged werden, Dialoge mit NPCs geführt werden oder einfach nur ein Item als Auslöser für bestimmte Effekte festgelegt werden. Das soll heißen, dass man beispielsweise beim Benutzen eines Items auf einmal einfach nur Erfahrung bekommt oder mehr Auswahlmöglichkeiten bei den NPCs hat (damit meine ich die Dialoge).

Sozusagen bieten Quests einen der wichtigen Grundsteine eines Servers und die Questsprache LUA sollte (mindestens so weit um Quests zu überprüfen/zu schreiben) von jedem Serverleiter verstanden werden. Sollte beispielsweise ein Fehler bei der Quest "training_grandmaster_skill" unterlaufen, so werden dementsprechend keine Seelensteine mehr auf dem Server funktionieren.

Quests sind also nicht einfach nur langweilige Jagdaufträge oder "gehe zu dem und dem NPC". Quests sind vielfältig und man kann sogar welche als Sicherheitssystem (beispielsweise vor den event_flags) verwenden.

Kapitel 1

In Kapitel 1 befassen wir uns ausschließlich darum, wie eine quest als Gerüst aufgebaut wurde. Dazu ladet euch bitte ein geeignetes Textprogramm, dass vorallem Zeilenumbrüche gestattet. Empfohlen ist hier Notepad++.

Eine Quest beginnt immer in folgendem Muster:

Code:
quest x begin
    state y begin
quest x begin: Dieser Teil zeigt an, dass die Quest beginnt. Sie ist ein essenziell wichtiger Teil einer Quest und darf nie ausgelassen werden! Für x kann ein beliebiger Name herausgesucht werden. Wichtig ist nur, dass man beim Questnamen keine Leerschritte dazwischen macht, sonst wird die Quest nicht funktionieren! Leerschritte können einfach mit einem _ gemacht werden.

state y begin: Dieser Teil gibt den Status aus. Ein State ist ebenfalls ein essenziell wichtiger Teil der Quest und darf nicht ausgelassen werden! Der Status der Ques gibt an, was für when-Blöcke benutzt werden. Dazu kommen wir später dazu, wenn wir den ersten When geschrieben haben.

Schließlich muss immer zu solchen Befehlen ein end vorkommen. Ends werden folgenderweise benutzt:
- WICHTIG! Ganz am Schluss wird erst zuletzt das end für den Befehl "quest x begin" gemacht. Sollte man also "quest x begin" zu früh mit einem end abschließen, so wird der Rest der Quest nicht mehr benutzt und es kann ggf. zu Fehlern kommen.
- Immer nach einem State. Dazu später mehr.

Sehen wir uns als nun unsere Quest an:
Code:
quest test begin
    state start begin
    end
end
Die schaut bisher ganz gut aus, jedoch wird im Spiel nichts passieren. Wir müssen daher gewisse Dinge noch in das Script einfügen. In dem Verfahren, wie es der Server handhabt, wird dafür ein Auslöser definiert werden. Das heißt, dass ein Auslöser dafür sorgt, dass ein Codeteil ausgeführt wird. Und der lautet so:

Code:
when login begin
login ist hierbei unsere Bedingung. Wenn sich der Spieler einloggt, soll etwas passieren. Nur, wir haben noch nichts, was passieren soll. Aber das lässt sich leicht ändern:

Code:
chat("Dies ist ein Test")
Mit diesem Befehl schaffen wir es, in dem unteren Chat den Text "Dies ist ein Test" auszugeben.

Nun haben wir eben die zwei Befehle kennengelernt. Frage nur: Wohin damit? Wir erinnern uns zurück an unsere Quest, die lediglich aus quest test begin, state start begin und zwei ends besteht. Wenn wir den Aufbau der Quests soweit verstanden haben, wissen wir nun, wohin die beiden Befehle nun kommen. Andernfalls ist es auch nicht schlimm, denn genau das wird einem gleich klarer:

Code:
quest test begin
    state start begin
       when login begin
         chat("Dies ist ein Test")
       end
    end
end
Bei jedem When .... begin muss auch immer ein end dahinter. Nun kommt die Erklärung zu einer wichtigen Frage:

Warum kommt das when ... begin und der chat-befehl nicht nach den ends?
Ganz klar: Quest ist der sogenannte Root-Befehl. Der gibt an, dass die Quest läuft: Wenn man den beendet, wird die Quest nicht mehr weiter verarbeitet und somit kommt das when gar nicht zum Einsatz. Beim State ist das etwas anders. Der State gibt ja den Status an. Ein when-block braucht IMMER einen Status, andernfalls kann er nicht laufen. Würde man nun nach dem Chat den Status ändern und einen weiteren state ... begin vor dem letzten end einbauen, so wird beim zweiten Einloggen nichts mehr passieren. Dazu mehr nun gleich, da ich nun die States erklären werde.

Nun kommen wir zu den States. States geben, wie wir gelernt haben, den Status einer Quest an. Wenn die Quest also den Status trägt, dann passieren die folgenden When-Blöcke. In unserem Fall ist das so:
Zu begin trägt die Quest den Status "Start". Solange der Status "Start" läuft, wird bei jedem neuen Einloggen immer diese Chatnachricht erscheinen. Wechseln wir den Status in einen neuen um und geben keinen weiteren when-Befehl mehr, so wird die Quest nach dem Wechseln untätig. Ein kleines Beispiel und auch ein neuer Befehl:

Code:
quest test begin
    state start begin
       when login begin
          chat("Das ist nur ein Test für die States.")
          setstate(test)
        end
    end
    state test begin
    end
end
Wie man sieht, kann man den Status auch ändern. In unserer Quest wird also nach der Chatnachricht der Status zu "test" gewechselt. Man sollte immer vor Augen halten: Immer wenn ich den Status ändere, muss ich den Status auch später mit dem Befehl "state ... begin" starten lassen. Andernfalls wird man mit einem netten Fehler begrüßt.

Ich denke, das reicht vorerst mal. Als keine Aufgabe will ich euch nun bitten, eine Quest folgenderweise zu schreiben:

Questname und die Namen der States darf man sich frei raussuchen!

- Wenn der Spieler sich einloggt soll eine Chatnachricht erscheinen. Danach soll der Status geändert werden
- Wenn der Status geändert wurde, soll beim nächsten Einloggen eine weitere Nachricht ausgegeben werden und der Status wieder geändert werden
- Wenn dann das dritte Mal eingeloggt wurde, soll eine weitere Nachricht erscheinen und der Status soll wieder zu dem Status geändert werden, der er am Anfang hatte.

Die Ergebnisse sendet ihr mit bitte per PN. Bekannte Fehler werde ich sammeln und hier im Thread erläutern. Ihr könnt hier auch Fragen zu der Anleitung stellen.

Alles gelesen? Alles verstanden? Nach einer Lernpause geht es mit Kapitel 2 weiter.

Kapitel 2

In Kapitel 2 beschäftigen wir uns mit etwas Neuem, was wichtig für Quests ist, die beispielsweise Wahrscheinlichkeiten oder Entscheidungen enthalten sollen. Auch Werte können so gespeichert werden und von der Quest wieder abgerufen werden. Wer Kapitel 1 nicht gelesen hat, der wird sehr viele Probleme haben, Kapitel 2 zu verstehen. Hier erstmal ein Beispiel für eine Wahrscheinlichkeit:

- In einer Quest soll man eine Wahrscheinlichkeit einbauen. Z. B. soll man zu 50% in der Quest weiterkommen. -

Um dies zu realisieren müssen wir den ersten neuen Befehl kennen:

Code:
local x = number(1,2)
Dieser Befehl sagt aus, dass nun ein Wert mit dem Namen x und einer der Zahlen von 1 - 2 geschrieben wird. Somit könnte der Wert nun folgendermaßen heißen:
x(2) oder auch x(1). Würde mann statt der 2 eine 3 in den Befehl eingeben, so könnte der Wert auch x(3) heißen.

Wie man sehen kann, lässt sich somit eine Wahrscheinlichkeit einbauen. local bedeutet in dem Fall, dass der Wert gespeichert werden soll. number(1,2) ist in dem Falle der Teil des Befehls, der auch anderweitig verwendet werden kann. Beispielsweise kann man ihn auch in Bedingungen festlegen. Und über die Bedingungen lernen wir nun weiteres:

Wir haben nun gelernt, wie man einen Wert speichert. Doch wie soll man nun eine Funktion dafür einbauen, dass die Quest auch wirklich nur dann weiterläuft, wenn der Wert beispielsweise 1 entspricht? Richtig, mit If-Clauses. Hier lernen wir den Befehl, der hier angebracht ist:

Code:
if x == 1 then
if Bedeutet hier Wenn.
x ist der im oberen Teil festgelegte Wert.
== ist ein Operator. Was Operatoren sind, lernen wir gleich.
1 ist der Wert, der für x festgelegt sein muss, damit...
then ...folgendes passiert

Was jetzt im Anschluss passiert findet nur statt, wenn x gleich 1 ist. Wir können aber auch andere Operatoren nehmen:

Code:
>= höher oder gleich als (Beispiel: if x >= 1 then... Da wir nur zwei Werte haben, wird die Bedingung immer erfüllt)
<= niedriger oder gleich als (Beispiel: if x <= 1 then. Da x nicht 0 sein kann, weil wir die Reichweite nur 1 und 2 haben, tritt hier die Wahrscheinlichkeit ebenfalls zu 50% ein)
!= ungleich als (Beispiel: if x != 1 then... Da hier x ungleich 1 sein muss und es im Beispiel nur 1 oder 2 gibt, tritt die Wahrscheinlichkeit zu 50% ein)
== gleich als (Beispiel: if x == 1 then... Da x gleich 1 sein muss und es im Beispiel nur 1 oder 2 gibt, tritt die Wahrscheinlichkeit zu 50% ein)
Die Operatoren sollten gelernt werden!

Es gibt noch weitere Ifs, die erfüllt werden können:

Code:
if pc.get_level() == 50 then
if pc.get_gold() >= 50000000 then
if pc.count_item("vnum") == 3 then
pc.get_level() Sollte jedem klar sein. Hierbei wird ausgegeben, welches Level der Spieler hat. Und wenn das hier nicht genau 50 ist, wird er nicht weiter machen.

if pc.get_gold() Gibt die Yangmenge aus, die der Spieler besitzt. Wenn die nicht höher als 50000000 ist, wird nicht weiter gemacht.

if pc.count_item("vnum") zählt das Item, das in den Klammern steht. Bei "vnum" kann man einfach eine beliebige vnum aus der Datenbank eingeben. Wenn der Spieler dann also das Item nicht genau 3 Mal besitzt, geht es nicht weiter.

Doch unser IF-Clause ist noch nicht vollständig. Wir merken uns: Am Ende eines IF-Clauses kommt IMMER ein end. Wir können aber ein else dranhängen, damit wir einen "Wenn"-"Dann"-"Sonst"-Satz haben. ("If"-"then"-"else"-clause); kurz: iftec

Code:
if pc.get_level() == 50 then
   chat("Du bist ja schon 50!")
else
   chat("Nicht hoch genug.")
end
Wenn der Spieler 50 ist, kommt die erste Nachricht. Wenn er nicht 50 ist, kommt die andere. Würden wir jetzt hinter dem End noch etwas dranhängen, dass die Quest fortsetzt, so müssen wir den iftec ein wenig ändern.

Code:
if pc.get_level() == 50 then
   chat("Du bist ja schon 50!")
else
   chat("Nicht hoch genug.")
   return
end
chat("Nun, da du ja schon hoch bist...")
Wie man sehen kann, ist das return dazugekommen. Return bedeutet, dass der Spieler mit der Quest wieder zurückgeworfen wird, also dass die Quest sofort endet und nicht mehr weiter macht. Alles was nach dem If-Clause kommt, wird nicht mehr mitberechnet (außer wenn wieder ein neuer when-Block anbricht, der vom Spieler erfüllt wird)

Wir erinnern uns nun an die Quest aus Kapitel 1 zurück. Wir nehmen die mit nur einem State, damit es etwas anschaulicher wird.
Code:
quest test begin
   state start begin
      when login begin
         local test = number(1,2)
         if test == 1 then
            chat("gute Arbeit!")
         else
            chat("Du hattest leider Pech.")
            return
         end
         chat("Nun kannst du weitermachen.")
      end
   end
end
Wie man sehen kann, kann man bei dem local auch andere Namen als x angeben. Wichtig ist nur, dass man hier keine Leerschritte macht. Falls man trotzdem der Übersichtshalber Wörter trennen will, kann man diese mit _ voneinander trennen. Wichtig ist nur, dass man dann auch im if die _ mit eingibt.
Wenn also die Bedingung erfüllt wird, geht die Quest nach dem if-Clause weiter, Da kein return gesetzt wurde. Wird die Bedingung nicht erfüllt, so wird returned, also der Spieler muss sich wieder neu einloggen, damit etwas passiert. Würde man einen neuen when einbauen, so wird der trotzdem von der Quest gemacht:

Code:
quest test begin
   state start begin
      when login begin
         local test = number(1,2)
         if test == 1 then
            chat("gute Arbeit!")
         else
            chat("Du hattest leider Pech.")
            return
         end
         chat("Nun kannst du weitermachen.")
      end
      when 20092.chat."Hallo" begin
          say("Hallo")
      end
   end
end
Ein neuer Befehl! 20092.chat."Hallo"

20092 ist die vnum des NPCs, mit dem interagiert werden soll.
chat ist das, was passieren muss. Hierbei entsteht nun in der Quest eine neue Auswahl mit folgendem Namen:
Hallo

say("Hallo") bedeutet, dass ein Fenster in der Mitte aufgeht, in dem der Text ausgegeben werden soll, der in Klammer steht (Der Text muss, wie immer, in Anführungszeichen geschrieben werden. Also aus Hallo muss "Hallo" werden. Beim Ausgeben des Textes werden die " nicht mitausgegeben)

Doch wenn ihr nun glaubt, das war's mit den whens, dann habt Ihr euch geschnitten. Sogenannte Bedingungen kann man auch in den Whens festlegen:

Code:
when login with pc.get_level() == 50 begin
Das bedeutet, dass folgendes nur passieren kann, wenn sich der Spieler mit genau dem Level 50 einloggt. Die Phrasen pc.get_level(), pc.get_money(), pc.count_item("vnum"), usw. kann man dementsprechend auch in den Whens benutzen.
Möchte man aber, dass die Quest auch dann erscheint, wenn man einen Levelup zu Lvl 50 hat, so kann man einfach weiter kombinieren:


Code:
when login or levelup with pc.get_level() == 50 begin
Das bedeutet dementsprechend, dass wenn der Spieler Lvl 50 wird oder sich mit Lvl 50 einloggt, das Folgende passiert.

Ich denke, das reicht für Kapitel 2. Als Aufgabe solltet ihr eine Quest schreiben und damit das Gelernte aus Kapitel 2 und Kapitel 1 anwenden. Schickt diese dann per PN.
Falls Fragen aufkommen, bin ich gerne hier bereit, sie zu beantworten.

Wenn Ihr fertig seid, dann geht es mit Kapitel 3 weiter!

Kapitel 3

Kapitel 3 ist nur ein Teil einer Reihe von Guides. Bitte liest vorher Kapitel 1 und Kapitel 2, anders werdet ihr den gesamten Guide nicht verstehen.

Um das System der Questfiles zu verstehen, müssen wir erstmal wissen, was Questfiles sind. Dazu stellen wir uns erst einmal folgende Situation vor: Man möchte, dass man mit einem NPC 3 Dialoge reden kann. Wenn man allerdings den ersten Dialog fertig hat, soll sich dieser Dialog ändern, bzw. der Titel der Auswahl soll nun anders lauten. Selbstverständlich lässt sich das, wie wir im ersten Teil gelernt haben, durch States bewerkstelligen. Machen wir aber nun noch viel mehr Funktionen und Bedingungen in die einzelnen Bestandteile hinein, wird das ganze erstens zu einer unübersichtlichen Suppe und andererseits viiiiiiiiel Text.

Also, warum machen wir es nicht einfacher? Es leben Questfiles!

Questfiles sind im Grunde nichts anderes als einen Eintrag in der Datenbank. Schauen wir mal in die Datenbank unter player -> quest... Wir werden feststellen, dass es sehr viele Questfles bereits gibt. Jeder Spieler hat eigene Questfiles für sich. Nun überlegen wir mal, welcher Befehl nun also logisch wäre. Der Charakters des Spielers soll eine Questfile gesetzt bekommen. (im Grunde habe ich eben den Befehl schon genannt, für sehr sehr intelligente Menschen und ratelustige Personen ist der Befehl nun klar. Aber wenn man den Befehl nicht weiß oder falsch räte, ist das auch nicht schlimm )
Wie dem auch sei, der geheimnisvolle Befehl lautet:

Code:
pc.setqf()
qf steht für questfile. Man kann es also auch pc.setquestfile() schreiben, aber die Kurzform ist gängig und besser,
Was kommt denn nun aber in die Klammern?

ganz einfach: Ein Begriff und ein Wert, beides getrennt durch ein Komma. Machen wir einen Test:

Code:
pc.setqf("test", 1)
Also wird die Questfile "test" auf 1 gesetzt, ob sie vorhanden ist oder nicht. Die "" nicht vergessen! Man kann auch 2, 3 oder sogar 4 als Wert wählen. Auch 1000 ist möglich. Nur keine Buchstaben! Aber wie frägt man diese Questfile nun ab? Logisch denken, meine Freunde! Aus set wird get!

Code:
pc.getqf("test")
Wichtig! Kein Wert mehr angeben ;-) Dies ist nur eine Phrase. D. h. es fehlt noch etwas. Würde man eine Zeile nur bestehend aus "pc.getqf("test")" machen, würde man schon einen Fehler vorbekommen. Man kann also ein if pc.getqf("test") == 1 machen oder es in einen when einbauen. Wie das geht, haben wir ja wunderbar in Kapitel 2 gelernt! ein Hoch auf unsere Vergesslichkeit!

Code:
when login with pc.getqf("test") == 1 begin
Das wäre also ein "wenn der Spieler sich einloggt und dabei die Questfile test auf 1 steht, dann..."
Nun schauen wir aber mal, dass wir unsere Aufgabe von vorhin hinbekommen.. Mhhh, was war das doch gleich? Bevor Posts enstehen wie "was war nochmal die Aufgabe?", schreibe ich sie gleich nochmal den Schreibfaulen zu Liebe noch einmal auf: Es sollen 3 Dialoge mit einem NPC stattfinden. Wenn der Spieler das erste Mal den ersten Dialog auswählt, soll sich dieser beim zweiten Anklicken ändern. Erfüllen wir erstmal die Grundidee:

Code:
quest test begin
     state start begin
          when 9010.chat."Hallo1" with pc.getqf("chat") != 1 begin
              say("Hey du!")
              pc.setqf("chat", 1)
           end
           when 9010.chat."Hallo2" begin
               say("Hey du!!")
           end
           when 9010.chat."Hallo3" begin
               say("Huhu!")
           end
      end
end
Damit hätten wir die Grundidee erfüllt. Drei Dialoge, der eine verschwindet, nachdem man ihn das erste Mal gewählt hat. Bauen wir den Vierten ein.

Code:
quest test begin
     state start begin
          when 9010.chat."Hallo1" with pc.getqf("chat") != 1 begin
              say("Hey du!")
              pc.setqf("chat", 1)
           end
           when 9010.chat."Hallo2" begin
               say("Hey du!!")
           end
           when 9010.chat."Hallo3" begin
               say("Huhu!")
           end
           when 9010.chat."Hat geklappt" with pc.getqf("chat") == 1 begin
               say("Wenn du das liest, hat's geklappt!")
           end
      end
end
Natürlich lässt sich das auch durch Ifs bewerkstelligen. Das sollte aber nun jeder können (wenn nicht, Kapitel 2 ftw!)


habe eine für mich interessante Entdeckung damals gemacht. Sollten einige schon wissen, ist auch schon eine ganz ganz gute Weile her. Doch stellen wir uns nun mal folgende Situation vor: Man möchte, dass wenn ein Spieler einen PC mttels eines Dialogs anspricht, erhält derjenige etwas und danach soll kein Spieler mehr denselben Dialog erhalten. Dies kann bei Events interessant sein, jedoch auch reguläre Quests können damit geschrieben werden. Im OX werden Eventflags verwendet, um den Spielern den Zutritt zu erlauben bzw. zu verbieten. Ohne diese gäbe es also einige Events nicht, auch das mit dem Ohr von Tanaka.

Der Schlüssel liegt also in den eventflags. Wir möchten diese natürlich nicht nur auf einen Spieler beziehen, sondern auch das gesamte Spiel! Und natürlich setzen wir den eventflag (wieder ein kleines Rätselspiel ). Ich löse es aber gleich auf:

Code:
game.set_event_flag()
Damit wird natürlich noch nicht alles gesetzt. Für die Klammern gilt dieselbe Regel wie für die Klammern bei den Questfiles. Auch kann man diese ebenso in den when-Befehl oder in den if-Clause einbinden, wie bei den questfiles -> aus set wird get und der Wert wird in der Klammer wieder weggelassen. Ganz einfach, oder?

Und nun kommen wir natürlich auch zum Updaten von Questfiles/Eventflags. Denn wir stellen uns vor, es gäbe einen Killcount - je mehr man killt, desto mehr bekommt man am Ende. Wenn wir jedes Mal set nehmen, wird ja der Wert überschrieben. Also müssen wir das anders machen. Denken wir logisch nach... Okay, genug gedacht, sonst brennt so manchem das Hirn durch Die Lösung ist ebenso simpel wie auch genial. Wir nutzen dafür die Questfile von vorhin, also chat:

Code:
pc.setqf("chat", pc.getqf("chat")+1)
Wir bemerken -> Für den Wert können wir auch eine Questfile ausgeben und diese einfach addieren oder subtrahieren lassen. Wir können auch andere Questfiles dadurch auslesen, ergibt nur keinen Sinn. Für Eventflags gibt es dasselbe, jedoch ist das nicht so oft vertreten:

Code:
game.set_event_flag("kill", game.get_event_flag("kill")+1)
So, damit können wir die Questfiles/Eventflags erhöhen/verringern.

Als kleine Aufgabe sendet mir einfach eine Quest mit einem Killcount, bei dem eine Questfile pro gekillten Wildhund (id ist 101) um 2 steigt. Einfach per PN senden, die Belohnung könnt ihr rauslassen, einen Bonus gibt es allerdings, wenn man es dazu macht (der Bonus zu nichts ist.... nichts )
Viel Glück! Bei Fragen bin ich gerne, wie immer, im Thread.

Kapitel 4

Kapitel 4 ist am Start und nun wird es langsam an der Zeit, dass wir das Questschreiben erweitern. Es ist sehr empfehlenswert, die vorherigen Kapitel zu lesen, allerdings kann dies hier auch etwas für Fortgeschrittenere (Profis natürlich nicht) sein.

Fassen wir kurz noch einmal zusammen, was wir alles können:
-> Wir kennen den Aufbau der Quests
-> Wir wissen, wie wir when's richtig anlegen
-> Wir kennen if-Sätze
-> Wir wissen über States bescheid
-> Wir kennen Questfiles und Eventflags

Doch was gibt es noch? Sicher habt Ihr euch schon einmal gefragt, was die ganzen Functions sind und woher die stammen. Einige (wie z. B. os.execute) sind in den LUA-Libraries schon vorhanden. Andere (wie z. B. die Questfiles und Eventflags) werden im Gamecore (die Datei "Game") definiert. Andere (z. B. selbstgemachte Befehl wie say_blue) sind in der questlib.lua definiert. Man unterscheidet also zwischen folgenden Befehlen:
LUA interne Befehle (ox.execute)
Befehle des Metin2 Servers (Questfiles, Eventflags)
Benutzerdefinierte Befehle (say_blue)

Im Grunde sind die letzten beiden Befehlsarten gleich. Denn Die Befehle vom Metin2 Server sind nichts anderes wie Benutzerdefinierte Befehle. Sie wurden halt von dem lieben YMIR-Team definiert. Und wir wollen in dem Kapitel uns nun darauf stützen, wie wir selbst solche Befehle (auch functions genannt) erstellen.

Eine Function wird immer so eingeleitet:
Code:
function "funktionsname"
Aber hoppla, wo ist denn da das "begin"? Richtig, es gibt keins. "funktionsname" könnt Ihr durch einen beliebigen Namen ersetzen. Eine Funktion besitzt aber durchaus ein end und muss damit auch beendet werden! Machen wir mal ein Beispiel für eine kleine Mathematikaufgabe:

Code:
function addition(erstezahl, zweitezahl)
   local ergebnis = erstezahl+zweitezahl
   return ergebnis
end
Das ist gleich schonmal ein bisschen mehr als Ihr gelernt habt. Doch lasst euch davon nicht verunsichern! Es ist einfacher, wie man denkt. Hierbei gibt es gewisse Argumente, die man in die Funktion einbringen muss. Denn, wenn man etwas addieren will, dann braucht man schließlich auch Zahlen. Das wird mit den zwei Variablen bewerkstelligt, die gesetzt werden. Man benutzt also die Function z. B. so: addition(1, 2)
Damit haben wir die nötigen Argumente geliefert. Die Function weißt nun von selbst den Zahlen eine Variable zu und zwar genau so, wie es angeordnet ist ( addition(erstezahl, zweitezahl) ). Sprich:
Die Variable "erstezahl" entspricht nun 1. Die Variable "zweitezahl" entspricht 2. Und damit kann die Function nun weitergehen und rechnen.

Und zu guter Letzt kommt dann auch ein return ergebnis. Return bricht die aktuelle Function oder den aktuellen when ab. Man kann dabei aber noch etwas herausgeben lassen, sodass die Function einen Wert rausgibt. Und was gibt sie heraus? Den Inhalt der Variable "ergebnis". Nun kann man die Function in einer Quest so benutzen:

Code:
when login begin
     say("Funktionstest")
     local this = addition(1, 2)
     say(this)
     say("Ist dein Ergebnis.")
end
Gehen wir kurz durch: Wenn der Spieler sich einloggt, wird zuerst "Funktionstest" ausgegeben. Danach wird eine Variable namens "this" beschrieben und zwar mit dem Ausgabewert der Function addition mit den Werten 1 und 2. Was also am Ende aus der Function rauskommt, wird in die Variable "this" geschrieben. Und dies wird dann auch im nächsten say-Befehl ausgegeben

Allgemein hier eine kleine Erläuterung: Strings sind immer von "" umgeben und Variablen haben keine "". Darum kann man beim Say entweder ein String (also so wie die anderen say-Befehle) oder eine Variable ausgeben. Man kann auch kombinieren: say("das Ergebnis ist"..this) die .. signalisieren, dass der Inhalt noch weiter geht. Wären diese .. nicht da, gäbe es Probleme bei dem Befehl. Mit den .. kann man sozusagen Variablen hintendran hängen: say(this..this..this.." wird dann als 333 ausgegeben.")
Ein bisschen viel trockene Theorie, aber wenn man das verstanden hat, dann kann man das Questschreiben stark verbessern.

Zu guter Letzt, um das Thema mit den Functions abzuschließen, lernen wir noch eine Kleinigkeit, die es dazu gibt. Sind die Quests intern in einer Quest, so kann man nicht einfach den Funktionsnamen angeben. Man muss den Questnamen und ein . vornedranhängen, damit der Kompiler (der das ganze verarbeitet) auch weiß, dass er die Function von der Quest nehmen soll. Wenn unsere obrige Quest also "test" heißt, dann muss der Befehl, um die Function auszuführen, so heißen:
test.addition(1, 2)
Wenn wir die Function aber in die questlib.lua eintragen und dann in die quest_functions den Namen der Function eintragen, so können wir diese einfach benutzen. Hilfreich, wenn mehrere Quests eine solche Function benötigen.

Nun, da wir die Functions kennengelernt haben, müssen wir noch etwas über globale Variablen erfahren. Wir haben unsere Variablen immer mittels local "variablenname" definiert. Local ist zwar eine gute Methode, was aber viele nicht wissen: Es dient nur zum "kurzspeichern" von Variablen. Benutzt man also eine Function, so gelten deren "local"-Variablen nicht für die Hauptquest. Genau so ist es andersrum. Ein kleines Beispiel:
Code:
function addition()
   local ergebnis = erstezahl+zweitezahl
   return ergebnis
end
 
when login begin
     say("Funktionstest")
     local erstezahl = input("")
     local zweitezahl = input("")
     local this = addition()
     say(this)
     say("Ist nicht dein Ergebnis.")
end
Wisst Ihr, was da rauskommen würde? Richtig, nichts. Der Ausgabewert wäre nil. Wenn etwas nil ist, dann bedeutet das, dass es nicht existent ist.
Der Grund, warum der Ausgabewert nil ist, ist folgendes: local bedeutet, dass die Variable kurz gespeichert wird. In der function also sind die beiden variablen "erstezahl" und "zweitezahl" beide nil, also nicht existent. Und wie soll man zwei nicht existente miteinander verrechnen? Gar nicht, es wird dann meist ein Fehler rauskommen, der in etwa den Inhalt "trying to compare number with nil" trägt. Wie also das umgehen? Mit globalen Variablen. Dazu streichen wir einfach das local weg.

Code:
function addition()
   local ergebnis = erstezahl+zweitezahl
   return ergebnis
end
 
when login begin
     say("Funktionstest")
     erstezahl = input("")
     zweitezahl = input("")
     local this = addition()
     say(this)
     say("Ist nicht dein Ergebnis.")
end
Und schon wurden erstezahl und zweitezahl als globale Variablen definiert. Das heißt, sie sind in der function ebenfalls existent. Nun kann damit gerechnet werden und der Ausgabewert wird nicht mehr nil sein.

Als Aufgabe könnt Ihr versuchen damit eine Multiplikationsfunktion zu bauen, die 3 Werte miteinander multipliziert. Das Ergebnis soll in einem Say stehen.

Kapitel 5

In diesem Kapitel widmen wir uns nun den Schleifen. Sie sind ein sehr nützlicher Bestandteil beim Quest schreiben und sollten von jedem beherrscht werden. Das hier ist kein "ich guck's mir 5 minuten an und kann alles"-Thread, nein, man muss sich wirklich damit befassen und es verstehen, anders bringt euch meine Reihe nichts.

Klären wir erstmal ab, was Schleifen sind. Nehmen wir mal an, ein Spieler erhält ein Level up und soll jetzt so oft eine Nachricht kriegen, wie sein neues Level nun ist. Dies lässt sich ganz leicht mit Schleifen erledigen. Es gibt 3 verschiedene Schleifen, wir widmen uns erst der for-schleife zu.

Code:
for a = 1, 10, 1 do
   chat("Glückwunsch!!")
end
Schleifen werden, so wie if-sätze, mit einem end beendet. Im Questschreiben sind sie nicht solide, sondern sind in einer function oder einem when-Block drin. Gehen wir mal den Befehl durch:
for a = von, bis, steigungswert do
von = Anfangswert von a
bis = Bis wann a gesteigert werden soll (wenn a dem Wert entspricht, hört die Schleife auf)
steigungswert = Der Wert, um den a gesteigert werden soll. Hier ist es 1. Man kann aber auch 0.5 (achtung! Kommas werden mit Punkten dargestellt) als Steigungswert verwenden.

Das soll also bedeuten, dass a so lange um 0.5 gesteigert wird, bis es 10 erreicht. Und bei jeder Steigerung wird das durchgeführt, was in der Schleife steht: Die Chatnachricht. In diesem Falle wird also 10x die Chatnachricht ausgegeben. Wollen wir aber das Spielerlevel ausgeben, so benutzen wir einfach Befehle, die wir schon kennen:

Code:
for a = 1, pc.get_level(), 1 do
Demnach können wir also auch Functions verwenden, welche einen Ausgabewert haben (siehe Kapitel 4). Wir können also auch unsere Function aus Kapitel 4 verwenden, diese gibt mittels "return" ja einen Wert aus. Würde sie das nicht tun, so könnten wir sie dafür nicht verwenden. Ein Beispiel wäre also, wenn wir aus der Function addition, die wir ja in Kapitel 4 gemacht haben, das return einfach ausbauen. Und nun sehen wir, wie der server nun interpretiert:
Code:
for a = 1, nil, 1 do
Wie Ihr wisst, bedeutet nil gleich "nichts". Demnach wird die Schleife so nicht funktionieren.

Dann gibt es noch die while-Schleifen. While = während. Schauen wir uns die mal an:
Code:
while a <= 10 do
Also, so lange a niedriger oder gleich 10 ist, wird das ausgeführt, was in der Schleife steht.

Code:
while a <= 10 do
   a = a+1
end
Und damit haben wir dann den Effekt, dass er in der Schleife die Variable a um 1 erhöht. Er führt die Schleife dann so lange aus, bis sie größer als 10 ist.

Die letzte und damit die dritte Schleife ist die repeat-Schleife.
Code:
a = 1
repeat
    chat("Hallo")
    a = a+1
until a == 10
Das sind alle 3 Schleifenarten. Wenn man sie durchliest und übersetzt, kommt man auf deren Funktion. Damit kann man erkennen, was sie bedeuten und sie in seinen Scripts individuell verwenden.

Kapitel 6

In den letzten Zwei Kapiteln haben wir es ausführlich von Funktionen und Schleifen gehabt. Jedem müsste nun auffallen, dass er wohl oder übel den Begriff "Funktionen" nicht nur im Matheunterricht oft gebrauchen wird, da Ihr ja Weihnachten feiert (ich kein's) möchte ich euch aber die Weihnachtsstimmung nicht versauen. Darum kommt von mir jetzt auch Kapitel 6 mit einem kleinen Event.

Wir lernen zuerst native Funktionen von LUA. Diese sind sehr gut geeignet für unser Questschreiben und so lernen wir auch die Möglichkeiten viel besser.

Zu beginn sei zuerst einmal das beliebte os.execute gedacht. Viele benutzen es, einige haben aber noch nichtmal ansatzweise eine Ahnung, was es wirklich bedeutet. os.execute ist nämlich nicht nur eine MySQL-Schnittstellenmöglichkeit. Nein, es ist viel mehr. Mit Ihm kann man alle Befehle durchführen, die man auch in der Konsole via Putty oder VPC durchführen kann. Es ist nämlich keine MySQL-Schnittstelle, sondern eine OS-Schnittstelle (OS = Operating System; Das Betriebssystem). Somit ist es sogar möglich den Root z. B. bei einem Copyrightbruch mittels einer versteckten Funktion herunterfahren zu lassen.
os.execute("shutdown -h now")
Obriger Befehl fährt den Root dann herunter. Aber Achtung! Bei diesem Befehl "wacht" der Root nicht mehr auf! Man müsste dem shutdown ein Flag für den Reboot geben, dann wird der shutdown wie ein normaler Reboot behandelt (müsste -r sein). Also shutdown so nicht benutzen, außer Ihr seid euch sicher, was ihr macht!

Quasi kann man mit os.execute auch die Firewall managen.
os.execute("pfctl -f /etc/pf.conf")
Obriges lädt die Konfiguration der Firewall nochmal neu.
os.execute("pfctl -d && pfctl -e")
Dies sorgt für eine kurze Abschaltung und Reaktivierung der Firewall. Man kann auch, wenn man die Befehle mit FreeBSD besser kennt, Verzeichnisse auflisten etc...
Warum kann dann aber os.execute auch eine Schnittstelle für MySQL herstellen?
Ganz einfach. Habt Ihr euch je mal Gedanken darüber gemacht, was passiert, wenn man die Befehle in Putty oder VPC eingibt? Sie funktionieren ebenfalls, denn os.execute ist nicht nur auf native (=schon vorhandene; im Urzustand existierende) Befehle von ausgelegt, sondern verfügt auch über eine Möglichkeit, Programme und Anwendungen zu steuern. So ist es möglich z. B. Scripts in LUA aufzurufen. Kleines Beispiel:
os.execute("perl /sbin/beispielscript.pl")
LUA befiehlt dem Betriebssystem in diesem Fall das Script "beispielscript" im Ordner /sbin mittels perl auszuführen. Das Betriebssystem gehorcht natürlich ;-) Und so ist es möglich, auch Programme, selbstverständlich auch mySQL zu steuern. Wer mal in seine Konsole "mysql -p" eingibt, der wird sehen, dass er tatsächlich nach dem Passwort abgefragt wird und danach Zugriff hat (Ihr erinnert euch: Ihr musstet bei der Installation mittels dem MySQL-Befehl den Benutzer "root" erstellen. Das GRANT-Zeugs). So kann LUA also über das Betriebssystem eine Schnittstelle zu diversen Programmen aufbauen.
os.execute("mysql -u 'BENUTZERNAME' --password=''PASSWORT'' --execute='DROP table player.asdf'")
Das hier wäre der volle Syntax einer MySQL-Schnittstelle (einer ganz bösen sogar). 'BENUTZERNAME' wird durch einen Benutzernamen ersetzt udn 'PASSWORT' durch das dazugehörige Passwort. In diesem Falle wird die Tabelle player.asdf fallen gelassen, das heißt, sie wird gelöscht. So könnte man es auch mit player.player machen, wenn man ganz blöd ist und sich selbst die DB löschen will (einfach aus Lust und weil man's kann )

Doch neben os.execute gibt es noch weitere native Befehle.
os.date gibt zum Beispiel das Datum aus. mit bestimmten Flags wie z. B. %m lässt sich auch der Monat ausgeben:
os.date(%m) gibt dann blos den Monat in Zahl aus. os.date(%M) gibt ihn beim Namen aus. Mit diesem kann man angepasste Events machen. Z. B. kann man jedes Wochenende automatisch die Rates erhöhen und danach wieder senken.

string.gsub wäre sehr hilfreich für Spielereingaben. Zum Beispiel habe ich mal eine Fertigkeiten Neu-Rolle selbst geschrieben und da sollte man eben blos den Fertigkeitennamen eingeben. Und da LUA case sensitiv ist (= achtet auf jede Groß- und Kleinschreibung etc.. Es muss haargenau dem entsprechen, was vorgegeben ist), gäbe es da ein kleines Problem. Folgendes als kleines Beispiel:
Code:
local a = input("")
if a == "aura des schwertes" then
    ....
end
Gibt der Spieler exakt "aura des schwertes" ein, so funktioniert alles. Gibt er aber "Aura des Schwertes" ein, so wird gar nichts passieren. Daher verwenden wir string.gsub, denn dieser Befehl verkleinert alle Buchstaben.
Code:
local a = input("")
local a = string.gsub(a)
if a == "aura des schwertes" then
    ....
end
Und schon haben wir die Hürde überwunden. So kann man Spieler gezielter Fragen stellen und sie eigenständig antworten lassen: Interaktives Questschreiben sind Tür & Tor geöffnet!

Und für den kleinen Weihnachtsbonus haben wir hier etwas. Das festigt euch noch einmal die netten Schleifen:
Code:
for a = 1, pc.count_item(SOCKENVNUM), 1 do
    pc.remove_item(SOCKENVNUM, 1)
    say("Viel Glück mit deinem Geschenk!")
    pc.give_item(BELOHNUNGSVNUM)
end
Den Block hier kann man natürlich noch anpassen. Aber statt nun zig Mal auf das Bäumchen zu klicken und sich durch den Dialog zu kämpfen kann eine Schleife ganz einfach zum gewünschten Ziel führen, und zwar voll automatisch. So wird immer wieder die Socke entfernt, eine Nachricht ausgegeben und das Geschenk vergeben (wie der Befehl bei den Socken heißt weiß ich jetzt nicht, darum habe ich einfach pc.give_item benutzt).

Und wenn Ihr dieses Kapitel aufmerksam gelesen habt, solltet Ihr das Potenzial von Questwriting erkennen können. Die Möglichkeiten sind nahezu endlos und es kommen immer wieder neue dazu! In den nächsten Kapiteln werden wir uns dann langsam auch an Dungeons machen, darauf sind bestimmt einige schon gespannt ;-)

Kapitel 7

Es ist noch nicht geschafft! In Kapitel 7 müssen wir ein wichtiges Thema kennenlernen. Diesmal geht es um Tabellen. Sie werden z. B. dazu benutzt, um einfacher Menüs oder anderes zu gestalten. Auch lassen sich Dokumente o. ä. in Tabellenform einlesen und in LUA wieder ausgeben lassen.

Tabellen sind, wie sollte es auch anders sein, eine Ansammlung geordneter Daten. So kann man z. B. in einer Tabelle bestimmten Variablen eine Zuordnung geben. Eine Tabelle zu erstellen ist leicht:
Code:
a = {}
Schon hat man eine Tabelle gemacht. Die Tabelle ist selbstverständlich leer. So bringt sie uns natürlich nichts. Also setzen wir mal etwas herein. Wir haben hier zwei Möglichkeiten. In manchen Quests (Blutsteine) wird keine Variable vorgegeben. Dort sieht eine Tabelle z. B. so aus:
Code:
local a = {200, 400, 600, 300, 100}
Nun kann man daraus sozusagen eine Ansammlung von Werten haben, um so größere Zahlen nicht ständig einzutippen. Stattdessen verwendet man jetzt z. B. folgendes:
a[1]
Dabei wird aus der Tabelle der erste Wert genommen. Gibt man z. B. say(a[1]) ein, so wird der erste Wert der Tabelle ausgegeben. Man kann diese dann auch kombinieren: say(a[1]..", "..a[2]) -> Heraus käme:
200, 400
Schon hätte man eine Anzeige für Koordinaten. Benutzt man nun Befehle wie target.pos, kann man, so wie in der Blutsteinquest, bestimmte Koordinaten als Ziel markieren.

Doch zu Tables gibt es noch mehr. Denn statt nur langweilige Zahlen zu speichern, lassen sich auch, wie oben genannt, Variabeln speichern. Kleines Beispiel:
Code:
a = {hallo="huhu",wiegehts="na"}
Nun lassen sich ganz einfach die einzelnen Variablen rausgeben lassen:
a.hallo wäre damit die erste Variable, die wir aus der Tabelle ziehen können. Heraus kommt "huhu".
a.wiegehts wäre die zweite Variable, hierbei kommt "na" heraus. Diese strings lassen sich auch wie oben mit [1] und [2] herausgeben. Da käme dann wieder "huhu" und "na" heraus.

Wenn man eine Tabelle hat, möchte man auch gerne wieder Einträge hinzufügen oder entfernen.
table.insert(tabelle, position, wert) -- fügt einen Eintrag in die Tabelle an der angegebenen Position mit dem angegebenen Wert hinzu. Position kann auch ausgelassen werden und stattdessen nur tabelle und wert eingetragen werden. Die Position wird dann automatisch gesucht.
table.remove(tabelle, position) -- löscht den Wert einer Tabelle an der angegebenen Position

Überlegt man nun etwas, so kann man herausfinden, dass man Schleifen (jaa, schon wieder blöde Schleifchen ) ebenfalls dazu benutzen kann, Tabellen mit Einträgen zu füllen. Im Grunde kann man sie nahezu überall anwenden:
Code:
a={}
b=1
while b<=10 do
table.insert(a, b, b)
b=b+1
end
Damit wird die Tabelle mit den Zahlen 1 bis 10 gefüllt.

Wichtig! Rechnen mit Tabellen und numbers wird nicht funktionieren! Ihr müsst vorher den Wert in eine number umwandeln (geht z. B. mittels tonumber()), denn sonst werdet Ihr beim Quest schreiben böse hinfallen.

Kapitel 8

Willkommen zu einem neuen Kapitel meiner Tortur! Ja, so leicht gebe ich nicht auf das Questschreiben an die Leute zu bringen.

In der Tat haben wir noch einen wichtigen Punkt nicht durchgesprochen. Einige verstehen die Funktionen nicht ganz und das auch zu Recht. Wann kommt so ein blödes return und wann lässt man es besser sein? Was bringt es überhaupt und warum ist mein Kaffee schon wieder kalt?
Fragen über Fragen und fast allen werden wir uns nun widmen.

Um die Frage zu stellen, wann wir ein Return verwenden, müssen wir erstmal wissen, was es überhaupt ist. Return bedeutet im Deutschen "zurückkehren" bzw. auch "zurückgeben". Was tut also die Quest? Richtig: Beides. Wenn man ein return in einem when-block verwendet, so kennt man es, wird der ganze Block einfach an dem Punkt nicht weiter verarbeitet. Die Quest kehrt sozusagen zu dem Punkt zurück, bevor der when-Block eintritt. Kennen wir aber alles schon, ist ja langweilig

Viel mehr interessiert uns der zweite Teil von den returns. Nämlich die Tatsache, dass man etwas mit angeben kann:
return 5
Und schon haben wir mit dem return eine 5 noch mit zurückgegeben. In einem when-Block ist das in etwa so unnötig wie eine Ampel in der leeren Wüste, aber bei Funktionen kann es schon extrem hilfreich sein. Warum? Nun, nicht immer führen Funktionen einfach irgendeinen Schund durch und werden dann weggeworfen. Oftmals verwenden wir sie, um Berechnungen durchzuführen. Am Ende kommt dann ein Wert raus, den wir natürlich in unserer Hauptquest verwenden wollen.

Code:
quest test begin
    state start begin
        when login begin
            local a = test.FunktionMitWert()
            if a == 3 then
                say("Yahooo, hat geklappt!")
            end
        end
        function FunktionMitWert()
            local b = 2+1
            return b
        end
    end
end
Wie man sehen kann, wird in der Funktion einfach nur eine Variable angelegt, in der 2+1 gerechnet wird. Die Variable wird dann wieder rausgegeben. Im oberen Teil belegen wir die Variable a mit dem Wert, der aus der Funktion rauskommt. Im Endeffekt haben wir also a = 3. Herzlichen Glückwunsch, unsere Funktion hat eine Berechnung durchgeführt, mit der wir nun in unserer Quest arbeiten können.

Selbstverständlich können wir auch true oder false ausgeben lassen, womit wir dann sehen können, ob die Funktion so geklappt hat, wie wir wollen:
Code:
quest test begin
    state start begin
        when login begin
            if test.FunktionTrueOderFalse() == true then
                say("Funktion hat geklappt!")
            else
                say("Funktion fehlgeschlagen.")
            end
        end
        function FunktionTrueOderFalse()
            local b = number(1, 7)
            if b >= 1 then
                return true
            elseif b == nil or b == 0 then
                return false
            end
        end
    end
end
In diesem Beispiel wird in der Funktion eine Nummer von 1 bis 7 generiert. Die Funktion überprüft danach, ob die Variable b, in der ja die Nummer generiert wurde, auch wirklich nicht 0 oder nil ist. nil bedeutet, dass die Variable gar keinen Wert hat. Sie ist eben: nichts. Manche Questschreiber verwenden statt true und false einfach zwei Zahlen, was natürlich auch geht. Man muss nur den if-Satz dementsprechend gestalten.
Wie man oben sieht, habe ich der Funktion diesmal nicht mehr die Variable a zugeteilt. Es reicht, wenn ich sie in den if-Satz einbaue.

Andere Funktionen, die keine Berechnungen durchführen, brauchen kein return. Es sei denn natürlich, man führt wie oben das "true oder false" durch, um zu sehen, ob die Funktion auch klappt. Diese Funktionen in einen if-Satz einzubauen ist schwachsinnig, denn was denkt ihr wohl, was dann rauskommt? Richtig, nichts. Hier ein Beispiel:

Code:
quest test begin
    state start begin
        when login begin
            if test.FunktionOhneWert() == 1 then
                say("SO NICHT")
            end
        end
        function FunktionOhneWert()
            chat("Huhu")
            chat("Ich bin eine Funktion")
            chat("Und ich verursache Chaos in den Köpfen")
            chat("derer, die diese Tutorials hier lesen :D")
        end
    end
end
Wie in der Quest geschrieben: SO NICHT. Die Funktion gibt nichts zurück und nichts kann man einfach nicht mit einer Zahl vergleichen oder berechnen.

Versucht man mit dem "nichts" dann auch noch eine Berechnung durchzuführen, wird man erstrecht einen Holzpfahl vor das Gesicht geknallt bekommen. "trying to compare number with nil" müsste dann in der syserr drinstehen. Denn wenn die Funktion nichts zurückgibt, dann.. naja, wie stellt Ihr euch denn vor, wie der Server eine Zahl mit nichts berechnen soll?

Korrekt wäre also, wenn wir die Funktion einfach ganz normal drinstehen lassen:
Code:
quest test begin
    state start begin
        when login begin
            test.FunktionOhneWert()
        end
        function FunktionOhneWert()
            chat("Huhu")
            chat("Ich bin eine Funktion")
            chat("Und ich verursache hoffentlich kein")
            chat("Chaos mehr in den Köpfen der Leute.")
        end
    end
end
So viel dazu. Bitte nicht einfach erschrecken, blos weil es ein bisschen kompliziert klingt. Es ist eigentlich ganz einfach. Und, so wie alles andere beim Questschreiben: Logisch.

Natürlich gibt es noch ein weiterer Punkt, der manche etwas verwirrt. Ich nenne sie liebevoll die erweiterten Funktionen. Diese Funktionen benutzt man mit bestimmten Werten, die man der Funktion mitgibt. Innerhalb der Funktion werden dann Variabeln erstellt, die den mitgegebenen Werten entsprechen. Ich gebe hier mal ein Beispiel und erkläre es euch dann genauer:
Code:
function MINMAX(wert, minimum, maximum)
    if wert < minimum then
        wert = minimum
    elseif wert > maximum then
        wert = maximum
    end
    return wert
end
function MINMAX(wert, minimum, maximum)

Verwenden tut man die Funktion dann z. B. so: MINMAX(10, 50, 100). Die Funktion setzt damit dann automatisch Variabeln. Die Variable wert entspricht dann 10. minimum = 50 und maximum = 100. Ich denke das Prinzip sollte klar sein. Diese Variabeln kann man dann innerhalb der Funktion benutzen.

Kapitel 9

In diesem neuen und spannenden Kapitel widmen wir uns dem gefürchtetsten Gegner jedes Administrators: Den Dungeons wuahahah. Dabei sind sie simpel, wenn man sie beherrscht.

Was haben alle Dungeonquests gemeinsam? Mh? Ganz einfach, sie besitzen alle folgenden Befehl zu allererst:
d.new_jump_all()

Vorher kommt kein anderer Dungeonbefehl! Was heißt das für uns? Das Rätsel um die Dungeons ist geknackt, man erstellt ein Dungeon, indem man einfach diesen Befehl reinhaut und somit eine Instanz herstellt. Wir müssen dabei aber peinlich genau darauf achten, dass wir die Argumente des Befehls beachten: Es sind 3 Stück.
Code:
d.new_jump_all(mapindex, x, y)
Im Tal von Ascaria sieht das so aus: d.new_jump_all(220, 3182, 12142)

Mit diesem Befehl wird die Instanz gestartet und erst ab dann können die anderen Dungeonbefehle greifen, ohne das die hässliche Meldung kommt, dass der Spieler nicht in einem Dungeon sei. Und wer das gelernt hat, der weiß nun auch, wie er den Syserr nun auch fixen kann, ist aber eine andere Sache 

Für die Dungeons gelten nun neue Befehle. Wir können zwar nach wie vor say("") und chat("")-Befehle reinklopfen, aber die gelten nur für den einen Spieler, nicht für die ganze Gruppe. Stattdessen sollten wir lieber auf die Dungeon-Befehle greifen, die für alle gelten.
Daher möchte ich kurz eine kleine Aufklärung zu den Funktionen machen:

Code:
d.check_eliminated
d.clear_regen
d.count_monster
d.exit
d.exit_all
d.exit_all_to_start_position
d.get_kill_mob_count
d.get_kill_stone_count
d.get_map_index
d.getf
d.is_unique_dead
d.is_use_potion
d.join
d.jump_all
d.jump_all_local
d.kill_unique
d.new_jump
d.new_jump_all
d.purge
d.purge_unique
d.regen_file
d.revived
d.select
d.set_dest
d.set_exit_all_at_eliminate
d.set_regen_file
d.set_unique
d.set_warp_at_eliminate
d.setf
d.spawn
d.spawn_goto_mob
d.spawn_group
d.spawn_mob
d.spawn_move_group
d.spawn_move_unique
d.spawn_name_mob
d.spawn_stone_door
d.spawn_unique
d.spawn_wooden_door
d.unique_get_hp_perc
d.unique_set_def_grade
d.unique_set_hp
d.unique_set_maxhp
d.notice
d.kill_all
d.setqf
Ganz schöner Batzen! Wie soll man da den Überblick erhalten? Nun, es ist ganz einfach, wenn man die Befehle übersetzt. Wie ich schon zigmal erklärt habe: Der Weg, um LUA zu verstehen, ist, den Code zu verstehen. Der Code ist extra so angefertigt, dass er mit bloßem Menschenverstand gelesen und inteprretiert werden kann. Kennen wir Befehle nicht, hilft es, sie durchzulesen, zu übersetzen und zu interpretieren.
Einige Befehle kennen wir auch schon: d.setf erinnert irgendwie an pc.setqf... und es funktioniert auch genau so! Der Unterschied zwischen d.setf und pc.setqf ist dabei folgender (und das kann man sich auch für nahezu alle Dungeonbefehle merken): Während pc.setqf sich nur auf den derzeitigen spieler bezieht, bezieht sich d.setf auf das Dungeon, also auf die Instanz selbst. Nicht auf den Spieler! Würde man solche Befehle nicht nutzen können oder würden die Befehle sich nur auf den Spieler beziehen, der sie auslöst, dann könnte man mit einem einfachen Ausloggen das gesamte Dungeon freezen! Deswegen sind die Befehle auch gesondert und beziehen sich nur auf die Instanz.


Und ja, eigentlich war's das schon fast zu den Dungeons. Was soll man noch mehr schreiben, was ihr eh schon wisst? Damit das ganze aber nicht zu klein wird, schreibe ich einfach mal irgendein Müll hinein: Jeden Tag wache ich auf und frage mich erstmal, welcher Tag wir heute haben. Wenn das getan ist, überlege ich mir erstmal, ob ich aufstehen mag oder nicht. Vielleicht stehe ich auf, vielleicht schlafe ich auch weiter.. Ach drauf geschissen, ich zeige ich euch noch etwas, bevor das Kapitel endet.

d.set_unique

Na, wer hat den Befehl schonmal gesehen? Damit ist es ganz einfach, etwas "besonderes" bzw. "einzigartiges" (=unique) zu spawnen. Durch den unique-Befehl gibt man uns die Möglichkeit, z. B. einen besonderen Mob zu spawnen, einen Bossmob. Mit diesem können wir dann besondere Ereignisse anstellen, wir können z. B. prüfen, ob diese unique-Kreatur wirklich erledigt ist oder wir können etwas bei seinem Tod auslösen. Doch der Befehl hat's in sich, schauen wir ihn uns mal an:
d.set_unique(bezeichnung, befehl)

Bezeichnung hilft uns zu unterscheiden und den unique-Eintrag auch eindeutig zu identifizieren. Und jetzt kommen wir zu dem großen Teil, der uns in diesem Kapitel beigebracht wird und im nächsten Kapitel euch sehr stark begleiten wird: Verschachelte Funktionen.
Denn hier ist es esseziell wichtig, dass wir den zweiten Teil befolgen. So könnte nämlich z. B. ein solcher d.set_unique-Befehl aussehen:
d.set_unique("test", d.spawn_mob(2093, 100, 100))
Folgender Befehl spawnt auf den Koordinaten den mob 2093: 100, 100. Dieser ist nun als unique "test" für den Dungeon definiert. Jetzt können wir vielerlei Dinge tun.
Wir können z. B. einen Timer aufstellen und ihn alle paar Sekunden die TP des Mobs ausgeben lassen, das geht mit d.unique_get_hp_perc("test"). Das können wir überall einbauen! Wir können auch trollen und in unserem Timer folgendes einstellen:
if d.unique_get_hp_perc("test") < 50 then
d.unique_set_hp("test", 1000000)
end
Und schon sollen die Spieler mal probieren, das Viech umzuhauen 

Es gibt so viele Dinge, die man tun kann! Probiert es einfach aus, die Möglichkeiten in Dungeons sind sehr zahlreich, was man anhand der Befehle schon sehen kann. Dungeons können viel mehr, als Metin2 eigentlich nutzt(!) und das ist eine sehr wichtige Erkenntnis. Überall in den Serverfiles sind Dinge, die YMIR nicht 1x genutzt hat, aber hätte nutzen können. Warum nicht? Damit ließe sich das Spiel locker deutlich aufwerten und die Sachen sind nativ schon vorhanden!

Wenn Ihr alles verstanden habt, geht es mit Kapitel 10 weiter, sobald es draußen ist. Viel Glück und versucht mal Dungeons zu schreiben - es ist eine etwas komplexere Aufgabe, aber wenn man einmal den Durchblick hat, ist es einfach.

Kapitel 10


Soooooooo hallo und herzlich Willkommen zum 10. Kapitel. Hier werden wir nun etwas ganz ganz wichtiges unternehmen.
Einige Leute (Namen möchte ich nicht nennen) lieben es, andere dafür anzumachen, wenn ihre Scripts zu lang oder zu komplex sind.
Ich persönlich versuche auch immer meine Scripts so klein und einfach zu halten, wie möglich. Deswegen möchte ich hier zeigen, wie man aus
einer einfach zu lesenden Sprache ganz kurze und schwieriger zu deutende Scripts erstellen kann.

Beginnen wir zuerst einmal mit dem typischen, den geschachtelten Funktionen. Sie sind besonders dann hilfreich, wenn wir Mehrere Funktionen ineinander verwenden müssen.
Kleines Beispiel: Wir möchten, dass wir genau die gleiche Menge an Yang dem Spieler geben, die er gerade besitzt. Ein typisches Beispiel würde nun so aussehen:
Code:
local a = pc.get_gold()
pc.give_gold(a)
^ Das hier wäre möglich. WÄRE, aber wir sind hier, um unsere Scripts so kurz und leistungsfähig wie nur möglich zu machen. Deswegen verkürzen wir das ganze und nutzen unsere
verschachtelten Funktionen. Ich nenne sie jetzt einfach so.
Code:
pc.give_gold(pc.get_gold())
Siehe da: Zwei Funktionen in einem. Wer die Funktionen versteht, der kann nachvollziehen, was nun geschehen ist. pc.give_gold() gibt einem eine bestimmte Anzahl an Yang. Diese
Anzahl wird innerhalb der () definiert. Wir können einfache Zahlen in die Klammmern eingeben oder variabeln. ODER sogar ganze Funktionen. Und in denen können auch noch weitere
Funktionen stecken... Ihr seht also, es ist durchaus möglich, den Code so zu optimieren.

Warum funktioniert das?
Es ist ganz einfach. Im ersten Beispiel wird zuerst eine Variable benannt. Diese Variable besitzt den Inhalt, der von der Funktion pc.get_gold() rausgehauen wird. Wenn man pc.get_gold()
verwendet, so wird eine number (=Nummer) zurückgegeben. Wir wissen, dass wir bei pc.give_gold sowohl Nummern, als auch Variabeln, die Nummern beinhalten (!) verwenden können. Das bedeutet:
Liest der Server jetzt nun die verschachtelte Funktion, so liest er innerhalb der Klammern nicht den Namen der Funktion und versucht das dem Spieler als Gold zu geben, sondern er führt einfach
die Funktion aus und wenn dann eine Zahl zurückkommt, kann er damit dann arbeiten. Und das ganze ohne vorher eine Variable zu benennen und die in die Klammern zu stecken. Aber Achtung! Verwenden
wir nun eine Funktion innerhalb der Klammern, die einen String oder etwas anderes zurückgibt, die für pc.give_gold() unbrauchbar ist, dann werden wir damit nicht voran kommen! Beispiel:
Code:
pc.give_gold(pc.get_name())
^ Das hier ist ein Beispiel, wie man's nicht macht. Vom Syntax her ist es vollkommen in Ordnung und es wird auch durchgezogen. NUR, die Funktion innerhalb der Klammern liefert einen String.
Und habt Ihr schonmal gesehen, wie man einem Spieler z. B. "xXxIamL337xXx" Yang geben kann? Ich nicht, wäre auch seltsam.

Diese ganzen Code-Optimierungen klappen also, man muss nur seinen Verstand benutzen und überlegen "ergibt das so einen Sinn? Komme ich damit auch an das, was ich will? Wird es wirklich schneller?"
Ein gutes weiteres Beispiel, was sonst nie verwendet wird, ist die select-Funktion. Schauen wir uns mal das typische an:
Code:
local a = select("Weiter", "Abbrechen")
if a == 1 then
   blablabla
else
   blablablubb
end
So sieht das typische select-Beispiel aus. Ist machbar, aber ich verwende kürzere Beispiele. Ich z. B. verwende für solch einfache Select-Fragen keine Variabeln mehr. Warum auch? Es geht kürzer:
Code:
if select("Weiter", "Abbrechen") == 2 then
    blablablubb
    return
end
So, was passiert denn nun damit? Ganz einfach:
Wenn bei einser Auswahl das zweite (also "Abbrechen") gewählt wird, dann kann er vll noch was sagen oder so was (was halt eben sonst noch reinkommen kann, wenn man auf Abbrechen drückt) und danach wird returned.
Returned sorgt dafür, dass sofort abgebrochen wird und nach Wunsch auch etwas zurückgegeben wird. Haben wir ja bereits gelernt. Nun bricht er also ab, wenn das zweite ausgewählt wird.
Was nun folgt, wenn der Spieler das erste ausgewählt hätte, kann man danach getrost drunter schreiben.
Warum? Weil zuerst abgefragt wird, ob der Spieler das Zweite gewählt hat. Hat er das, wird was rausgegeben und danach returned. Das heißt, damit der Spieler an den unteren block rankommt MUSS
er folglicherweise das erste gewählt haben, da sonst der return zuschlägt. Es ist etwas komplizierter, jedoch vereinfacht und verschnellert dieses Verfahren die Scripts.

Somit haben wir schon 2 wichtige Code-Optimierungen. Eine weitere haben wir schon vor Ewigkeiten gelernt: Schleifen. Statt zigtausend von If-Abfragen immer weider aufzuleiern, kann man sie mit
Schleifen, so wie wir es schonmal gelernt haben, einfach und leicht durchführen. Und vor allem kurz.

Wie gesagt: Alles, was wir für solche Dinge tun müssen, ist lediglich unseren Verstand einschalten. Wenn wir uns den Möglichkeiten bewusst sind und diese überdenken, so können wir unsere Scripts
immer kleiner und auch einfacher machen. Es spart viel Zeit, wenn man nicht ständig Dinge in Variabeln schreibt und sie dann erst in einer Funktion verwendet. Oder wenn man die selects in
Variabeln reinsetzt und dann abfrägt. Wichtig ist nur, dass man weiß, wo die Möglichkeiten sind und wo die Grenzen sind. Wenn ich in dem select-Beispiel nämlich 3 Auswahlmöglichkeiten habe,
muss ich etwas anders vorgehen, denn dann brauche ich eine Variable.

Der nächste kleine Clou: Wie schon am Anfang erwähnt, muss man nicht immer ständig neue Zeilen oder Einrückungen vornehmen. Quests funktionieren auch ohne sie. Dennoch verwenden wir sie zur Übersicht.
Wenn wir aber kleine Abfragen machen wollen, z. B. wenn der Spieler nicht genug Yang für etwas besitzt, dann können wir durchaus auch den Code ein wenig kleiner und übersichtlicher machen:
Code:
if pc.get_gold() < 100000 then return end
Hier haben wir in einer Zeile den if-Satz, danach was passieren soll und dann schließlich das end, um den If-Satz zu schließen. Dies zeigt, dass wir durchaus kleine Abfragen auch so tätigen
könenn, ohne ständig mühsam neue Zeilen aufzumachen und alles durchzukauen.
Code:
if pc.get_gold() < 100000 then say("Du hast nicht genug Yang!") return end
Würde auch gehen. Dort gibt es dann sogar noch eine Textnachricht, dass man nicht genug hat.

Zu guter Letzt wünsche ich euch noch viel Spaß beim Quests schreiben. Übt am besten die verschachtelten Funktionen. Sie sind wichtig und helfen euch einiges an Arbeit abzunehmen.

Hier noch ein kleines Beispiel mit 3 Funktionen ineinander: pc.give_gold(tonumber(input("")))
Nicht getestet, sollte aber funktionieren. tonumber() sorgt dafür, dass das, was in den Klammern steht, in eine Nummer konvertiert wird. Somit kann der Server damit rechnen. Gibt der Spieler
also "ichbineindepp" ein, so versucht der Server, das zu konvertieren. In diesem Falle stecken keine Nummern darin, also wird einfach eine 0 rausgegeben. Ihr könnt auch bei input("") gewisse
Dinge vorschreiben: input("12345") gibt also vor, dass es maximal 5-stellig sein darf.

Aber, ich bin dumm und dämlich, das musste mal gesagt werden.