import { create } from "zustand";
import { useNavigationStore } from "@/store/navigation";

export type SettingsSection =
  | "general"
  | "connection"
  | "server-paths"
  | "client"
  | "notifications"
  | "updates";

interface SettingsSectionState {
  section: SettingsSection;
  setSection: (section: SettingsSection) => void;
}

// Eigener kleiner Store, damit z.B. die Startseite direkt auf
// "Einstellungen → Verbindung" springen kann statt nur auf die Einstellungen.
export const useSettingsSection = create<SettingsSectionState>((set) => ({
  section: "general",
  setSection: (section) => set({ section }),
}));

export function openSettings(section: SettingsSection) {
  useSettingsSection.getState().setSection(section);
  useNavigationStore.getState().setSection("settings");
}
