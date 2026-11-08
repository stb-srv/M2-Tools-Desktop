# Broadcast-System

Legt Ansagetexte an, die automatisch und wiederkehrend im Spiel ausgegeben werden - ohne dass jemand `/n` oder `/notice` von Hand eintippen muss.

## Wie es funktioniert

Jede Nachricht bekommt ihr eigenes Intervall (in Minuten) und wird beim Klick auf **Deploy & Neuladen** in eine einzelne generierte Quest-Datei (`Broadcast/Broadcast_System.lua`) geschrieben und live auf den Server geladen - derselbe Mechanismus wie „Quests reloaden" in der Server-Steuerung, kein Neustart nötig.

Technisch nutzt jede Nachricht einen eigenen, spielerunabhängigen Server-Timer (`server_loop_timer`), der beim ersten Spieler-Login nach einem Deploy einmalig scharfgeschaltet wird und danach im eingestellten Takt eine normale Server-Ansage (`notice_all()`) an alle Spieler auf allen Cores sendet.

## Bewusste Einschränkungen

- **Nur normale Ansagen, kein großes Banner.** Der `/b`-Befehl (großes Infofenster) sendet auf diesem Server tatsächlich nur an den GM selbst, der ihn eingibt - es gibt aktuell keinen funktionierenden globalen "großes Banner"-Broadcast, das würde eine Server-Source-Änderung brauchen.
- **Timer starten erst beim ersten Login nach einem Deploy**, nicht sofort beim Speichern hier - falls direkt nach einem Deploy niemand online ist, startet der Takt erst mit dem nächsten Login (durch wen auch immer).
- **Bearbeiten braucht ein erneutes Deploy.** Jede Änderung an Text oder Intervall bekommt intern eine neue Revisionsnummer, damit ein bereits laufender alter Timer nach dem Neuladen sauber ins Leere läuft statt eine veraltete Nachricht weiterzusenden - dafür muss nach jeder Änderung erneut auf "Deploy & Neuladen" geklickt werden, sonst bleibt der alte Stand aktiv.
- Eine Nachricht deaktivieren entfernt sie beim nächsten Deploy komplett aus der generierten Datei, statt sie nur stumm zu schalten.
