import { create } from "zustand";

export type Theme = "light" | "dark" | "system";

const STORAGE_KEY = "m2manager-theme";

function applyTheme(theme: Theme) {
  const isDark =
    theme === "dark" ||
    (theme === "system" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", isDark);
}

interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
}

const initialTheme = (localStorage.getItem(STORAGE_KEY) as Theme) ?? "system";
applyTheme(initialTheme);

export const useThemeStore = create<ThemeState>((set) => ({
  theme: initialTheme,
  setTheme: (theme) => {
    localStorage.setItem(STORAGE_KEY, theme);
    applyTheme(theme);
    set({ theme });
  },
}));

window
  .matchMedia("(prefers-color-scheme: dark)")
  .addEventListener("change", () => {
    if (useThemeStore.getState().theme === "system") {
      applyTheme("system");
    }
  });
