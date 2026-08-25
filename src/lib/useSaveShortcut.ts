import { useEffect } from "react";
import { useNavigationStore, type Section } from "@/store/navigation";

/**
 * Ctrl+S / Cmd+S triggers `onSave` while `enabled` (there's something unsaved
 * and no save is already in flight) AND `section` is the currently active
 * nav section - `preventDefault` so it doesn't also fire the browser's "Save
 * Page As" dialog inside the WebView2/Chromium shell.
 *
 * The active-section check matters because App.tsx now keeps every visited
 * section mounted-but-hidden instead of unmounting it (see SectionSlot) - a
 * plain `window` keydown listener doesn't know about CSS `display:none`, so
 * without this check a hidden-but-still-mounted editor's Ctrl+S would keep
 * firing (and stealing the keystroke via preventDefault) while the user is
 * actually typing Ctrl+S into a completely different, currently visible
 * section.
 */
export function useSaveShortcut(section: Section, enabled: boolean, onSave: () => void) {
  useEffect(() => {
    if (!enabled) return;
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        if (useNavigationStore.getState().section !== section) return;
        e.preventDefault();
        onSave();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [section, enabled, onSave]);
}
