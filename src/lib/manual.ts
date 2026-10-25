import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { emit } from "@tauri-apps/api/event";
import type { ModuleId } from "@/store/navigation";

// Zentraler Helfer für den "Hilfe"-Knopf auf jeder Modul-Seite - Muster
// 1:1 aus QuestBuilder.tsx's openWiki() übernommen (eigenes Fenster bleibt
// parallel zur Arbeit offen, `getByLabel` fokussiert ein bereits offenes
// Fenster erneut statt ein zweites zu öffnen), hier einmal zentral statt in
// jeder Modul-Seite dupliziert. `section` entspricht 1:1 einem `ModuleId`-
// Wert aus navigation.ts (Sidebar-Bereich oder eingebettetes Modul) - das
// ist zugleich die Seiten-ID im Handbuch, also kein zusätzliches Mapping nötig.
export async function openManual(section?: ModuleId): Promise<void> {
  const existing = await WebviewWindow.getByLabel("manual");
  if (existing) {
    await existing.setFocus();
    if (section) {
      // Fenster ist schon offen (ggf. auf einer anderen Seite) - per
      // Tauri-Event zur richtigen Seite springen, das Handbuch-Fenster
      // hört darauf (Manual.tsx).
      await emit("manual-navigate", { section });
    }
    return;
  }
  new WebviewWindow("manual", {
    url: section ? `manual.html#${section}` : "manual.html",
    title: "Handbuch - M2Manager",
    width: 1100,
    height: 800,
  });
}
