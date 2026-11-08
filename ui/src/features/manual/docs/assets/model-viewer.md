# 3D-Modell-Viewer

Zeigt `.gr2`-Modelle (Client-Format für 3D-Modelle) direkt im Programm an, inklusive Texturen - ohne den Client zu starten.

## Was du hier tun kannst

- Ein Modell auswählen und im 3D-Raum ansehen/drehen.
- Wird auch **innerhalb** anderer Module genutzt, z.B. für die NPC-Vorschau im Shop-Editor.

## Wichtig zu wissen

Das Laden nutzt einen eigenen, 32-Bit-Hilfsprozess, der die echte `granny2.dll` deines eigenen Clients lädt (dieselbe Bibliothek, die auch der Client selbst zum Rendern nutzt) - dadurch werden sowohl starre als auch animierte (Skinned-Mesh-)Modelle korrekt dargestellt, inklusive NPC-Texturen.
