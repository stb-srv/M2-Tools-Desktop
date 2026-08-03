// Core-specific facts learned by reading this server's own source code
// (not generic Metin2 convention) - shown via the "i" info button in the
// Mob Drop Editor. Extend this array as more of the source gets explored.

export interface ServerNote {
  title: string;
  intro?: string;
  formula?: string;
  bullets?: string[];
}

export const SERVER_NOTES: ServerNote[] = [
  {
    title: "Serverfiles von Fliege V3 - Fratello",
    intro:
      "Verifiziert im Server-Quellcode (item_manager.cpp: CreateDropItem/GetDropPct, item_manager_read_tables.cpp: ReadMonsterDropItemGroup) - Formel am 2026-08-03 korrigiert, nachdem ein Nutzer im echten Spiel eine deutlich andere Drop-Häufigkeit beobachtet hatte, als eine frühere Version dieser Anzeige vorhergesagt hat.",
    formula: "Reale Drop-Chance ≈ Prozentwert aus der Datei ÷ 4",
    bullets: [
      "Der Prozentwert wird beim Einlesen intern ×10000 gespeichert (dwPct) und gegen einen Zufallsbereich von 4.000.000 gewürfelt (iRandRange) - als Bruch (0-1) ergibt das Prozentwert ÷ 400, als Prozentzahl (×100) also Prozentwert ÷ 4. Eine frühere Version dieser App hatte das ×100 vergessen und zeigte dadurch einen 100× zu niedrigen Wert an.",
      "Die Level-Differenz zwischen Killer und Mob verändert die Chance zusätzlich (aiPercentByDeltaLev-Tabelle) - großer Levelunterschied verringert sie.",
      "Zufalls-Glücksbonus bei jedem Kill: 1-zu-50.000-Chance auf einen ×10-Boost, 1-zu-10.000-Chance auf ×5.",
      "Server-weite Item-Rate (Standard ×1) kann per Quest-Skript für Events verändert werden (SetMobItemRate) - keine statische Config.",
      "Premium 'Item Drop' oder das Unique-Item 'Double Item' beim Killer verdoppeln die Chance.",
      "Das GM-Privileg PRIV_ITEM_DROP erhöht die Chance zusätzlich.",
    ],
  },
];
