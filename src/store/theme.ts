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

// Gesamt-Zoom der Oberfläche in Prozent. Tailwind rechnet in rem, also
// skaliert die Schriftgröße von <html> praktisch alles mit (Text, Abstände,
// Icons) - für große Monitore oder wer schlecht sieht.
export const UI_SCALES = [90, 100, 110, 125, 150] as const;
const SCALE_KEY = "m2manager-ui-scale";

function applyScale(scale: number) {
  document.documentElement.style.fontSize = `${scale}%`;
}

function clampScale(scale: number): number {
  return Math.min(UI_SCALES[UI_SCALES.length - 1], Math.max(UI_SCALES[0], scale));
}

interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  scale: number;
  setScale: (scale: number) => void;
}

const initialTheme = (localStorage.getItem(STORAGE_KEY) as Theme) ?? "system";
applyTheme(initialTheme);

const storedScale = Number(localStorage.getItem(SCALE_KEY));
const initialScale = storedScale ? clampScale(storedScale) : 100;
applyScale(initialScale);

export const useThemeStore = create<ThemeState>((set) => ({
  theme: initialTheme,
  setTheme: (theme) => {
    localStorage.setItem(STORAGE_KEY, theme);
    applyTheme(theme);
    set({ theme });
  },
  scale: initialScale,
  setScale: (scale) => {
    const value = clampScale(scale);
    localStorage.setItem(SCALE_KEY, String(value));
    applyScale(value);
    set({ scale: value });
  },
}));

// Strg + / Strg - / Strg 0 wie im Browser: eine Stufe größer/kleiner bzw. 100%.
window.addEventListener("keydown", (e) => {
  if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
  const { scale, setScale } = useThemeStore.getState();
  const index = UI_SCALES.findIndex((s) => s >= scale);
  if (e.key === "+" || e.key === "=") {
    setScale(UI_SCALES[Math.min(UI_SCALES.length - 1, index + 1)]);
  } else if (e.key === "-") {
    setScale(UI_SCALES[Math.max(0, index - 1)]);
  } else if (e.key === "0") {
    setScale(100);
  } else {
    return;
  }
  e.preventDefault();
});

window
  .matchMedia("(prefers-color-scheme: dark)")
  .addEventListener("change", () => {
    if (useThemeStore.getState().theme === "system") {
      applyTheme("system");
    }
  });
