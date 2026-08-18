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
  setAvailable: (info) => set({ available: info }),
}));
