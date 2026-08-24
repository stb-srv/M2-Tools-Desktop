import { invoke } from "@tauri-apps/api/core";
import type { Section } from "@/store/navigation";

/**
 * Schreibt einen Eintrag ins zentrale Änderungsprotokoll - bewusst
 * fire-and-forget (`.catch(() => {})`), nicht über `runAsyncAction`: ein
 * fehlgeschlagener Log-Eintrag darf niemals die bereits erfolgreich
 * abgeschlossene Nutzeraktion, in deren `onSuccess`-Handler dieser Aufruf
 * sitzt, nachträglich als Fehler erscheinen lassen.
 *
 * Ein Aufruf pro abgeschlossener *logischer* Aktion, nicht pro
 * Tauri-Kommando - bei mehrstufigen Pipelines (z.B. Item Editor: DB-Insert →
 * Beschreibung → Icon → Repack) also erst nach dem letzten Schritt, sonst
 * entstehen mehrere Log-Zeilen für einen einzigen Nutzer-Klick.
 */
export function logActivity(
  module: Section,
  action: string,
  summary: string,
  targetKind?: string,
  targetRef?: string,
) {
  invoke("log_activity", {
    module,
    action,
    targetKind: targetKind ?? null,
    targetRef: targetRef ?? null,
    summary,
  }).catch(() => {});
}
