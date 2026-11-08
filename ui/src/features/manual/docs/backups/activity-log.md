# Änderungsprotokoll

Eine gemeinsame, durchsuchbare Zeitleiste aller relevanten Änderungen über die ganze App - vorher hatte fast jedes Modul entweder gar kein Protokoll oder ein eigenes, isoliertes (Backup-Browser, Bauen & Einspielen, Modul-Importer).

## Was du hier siehst

- Ein Eintrag pro abgeschlossener Nutzeraktion (nicht pro internem Rust-Kommando) - z.B. "Item 3219 aktualisiert" statt einer Zeile pro Zwischenschritt der Speicher-Pipeline.
- Nach Modul filterbar (Dropdown) und per Volltextsuche über die Zusammenfassung durchsuchbar.
- Einträge aus **Bauen & Einspielen** und dem **Modul-Importer** erscheinen hier automatisch mit - sie kommen weiterhin aus ihren eigenen, spezialisierten Verläufen und werden nur zusätzlich hier eingeblendet.

## Rückgängig machen

- **Modul-Importer**-Einträge lassen sich direkt hier rückgängig machen (derselbe Vorgang wie im Modul-Importer selbst).
- **Bauen & Einspielen**-Einträge verlinken stattdessen auf die eigentliche Seite - das Zurückrollen einer Server-Programmdatei ist die folgenreichste Aktion im ganzen Tool (kein Testserver, betrifft sofort den Live-Server) und bleibt bewusst hinter der dortigen Tippen-zum-Bestätigen-Sicherung, statt sie hier zu duplizieren.

## Wichtig zu wissen

Nicht jede Aktion in der App erzeugt einen Eintrag - reine App-Einstellungen, Zugangsdaten und Datei-Exporte/-Konvertierungen sind bewusst ausgenommen (kein Server-/Spieldaten-Effekt). Ein fehlgeschlagener Protokoll-Eintrag lässt niemals eine ansonsten erfolgreiche Aktion als fehlgeschlagen erscheinen - im Zweifel fehlt dann nur der Log-Eintrag, nicht die eigentliche Änderung.
