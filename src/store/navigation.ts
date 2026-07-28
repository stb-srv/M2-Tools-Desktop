import { create } from "zustand";

export type Section =
  | "dashboard"
  | "connections"
  | "server-control"
  | "db-explorer"
  | "shop-editor"
  | "model-viewer"
  | "settings";

interface NavigationState {
  section: Section;
  setSection: (section: Section) => void;
}

export const useNavigationStore = create<NavigationState>((set) => ({
  section: "dashboard",
  setSection: (section) => set({ section }),
}));
