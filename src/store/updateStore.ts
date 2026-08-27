import { create } from "zustand";

interface AvailableUpdate {
  version: string;
  notes: string | null;
}

interface UpdateStoreState {
  available: AvailableUpdate | null;
  setAvailable: (info: AvailableUpdate | null) => void;
}

// Getrennt vom navigation.ts-Store (dessen dirtySections eine andere
// Bedeutung haben - ungespeicherte Änderungen, nicht "neue Version
// verfügbar") - App.tsx setzt das beim leisen Start-Check, Sidebar.tsx
// zeigt einen kleinen Punkt am Einstellungen-Eintrag, Settings.tsx zeigt
// die volle Update-Sektion.
export const useUpdateStore = create<UpdateStoreState>((set) => ({
  available: null,
  setAvailable: (info) => {
    set({ available: info });
    if (info) {
      // Persisted so WhatsNewDialog.tsx can show these exact notes once,
      // right after the app actually restarts on `info.version` - captured
      // here (the moment ANY update becomes known, whether from the silent
      // startup check or a manual "Nach Updates suchen" click) rather than
      // at the point the user clicks "installieren", so it works no matter
      // which path led to the install and needs no changes to the actual
      // install flow itself.
      PENDING_NOTES_KEY_STORE.setItem(PENDING_NOTES_KEY, JSON.stringify(info));
    }
  },
}));

const PENDING_NOTES_KEY = "m2manager-pending-update-notes";
const PENDING_NOTES_KEY_STORE = window.localStorage;

/** Reads back whatever `setAvailable` last persisted - `WhatsNewDialog`
 * compares `version` against the app's actual running version to decide
 * whether "we just updated to exactly this version" is true. */
export function readPendingUpdateNotes(): AvailableUpdate | null {
  try {
    const raw = PENDING_NOTES_KEY_STORE.getItem(PENDING_NOTES_KEY);
    return raw ? (JSON.parse(raw) as AvailableUpdate) : null;
  } catch {
    return null;
  }
}
