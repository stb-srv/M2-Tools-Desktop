import { useEffect, useMemo, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { listen } from "@tauri-apps/api/event";
import { Search, BookOpen } from "lucide-react";

import introMd from "./docs/intro.md?raw";
import dashboardMd from "./docs/uebersicht/dashboard.md?raw";
import serverControlMd from "./docs/uebersicht/server-control.md?raw";
import buildDeployMd from "./docs/uebersicht/build-deploy.md?raw";
import itemEditorMd from "./docs/items/item-editor.md?raw";
import itemProtoExplorerMd from "./docs/items/item-proto-explorer.md?raw";
import moduleImporterMd from "./docs/items/module-importer.md?raw";
import refineEditorMd from "./docs/items/refine-editor.md?raw";
import boxEditorMd from "./docs/items/box-editor.md?raw";
import cubeEditorMd from "./docs/items/cube-editor.md?raw";
import shopEditorMd from "./docs/shops-monster/shop-editor.md?raw";
import mobProtoEditorMd from "./docs/shops-monster/mob-proto-editor.md?raw";
import mobDropEditorMd from "./docs/shops-monster/mob-drop-editor.md?raw";
import dropGeneratorMd from "./docs/shops-monster/drop-generator.md?raw";
import questBuilderMd from "./docs/quests-welt/quest-builder.md?raw";
import regenEditorMd from "./docs/quests-welt/regen-editor.md?raw";
import localeEditorMd from "./docs/quests-welt/locale-editor.md?raw";
import serverEventsMd from "./docs/server-accounts/server-events.md?raw";
import accountManagerMd from "./docs/server-accounts/account-manager.md?raw";
import gmManagerMd from "./docs/server-accounts/gm-manager.md?raw";
import dbExplorerMd from "./docs/server-accounts/db-explorer.md?raw";
import broadcastSystemMd from "./docs/server-accounts/broadcast-system.md?raw";
import weatherControlMd from "./docs/server-accounts/weather-control.md?raw";
import backupBrowserMd from "./docs/backups/backup-browser.md?raw";
import dbBackupsMd from "./docs/backups/db-backups.md?raw";
import tgaConverterMd from "./docs/assets/tga-converter.md?raw";
import iconBrowserMd from "./docs/assets/icon-browser.md?raw";
import modelViewerMd from "./docs/assets/model-viewer.md?raw";
import systemInstallerMd from "./docs/systeme/system-installer.md?raw";
import settingsMd from "./docs/settings.md?raw";

// Seiten-IDs entsprechen 1:1 den `Section`-Werten aus store/navigation.ts -
// dadurch kann jedes Modul per openManual(section) direkt auf seine eigene
// Seite verlinken, ohne eine separate Zuordnungstabelle pflegen zu müssen.
const PAGES: Record<string, string> = {
  intro: introMd,
  dashboard: dashboardMd,
  "server-control": serverControlMd,
  "build-deploy": buildDeployMd,
  "item-editor": itemEditorMd,
  "item-proto-explorer": itemProtoExplorerMd,
  "module-importer": moduleImporterMd,
  "refine-editor": refineEditorMd,
  "box-editor": boxEditorMd,
  "cube-editor": cubeEditorMd,
  "shop-editor": shopEditorMd,
  "mob-proto-editor": mobProtoEditorMd,
  "mob-drop-editor": mobDropEditorMd,
  "drop-generator": dropGeneratorMd,
  "quest-builder": questBuilderMd,
  "regen-editor": regenEditorMd,
  "locale-editor": localeEditorMd,
  "server-events": serverEventsMd,
  "account-manager": accountManagerMd,
  "gm-manager": gmManagerMd,
  "db-explorer": dbExplorerMd,
  "broadcast-system": broadcastSystemMd,
  "weather-control": weatherControlMd,
  "backup-browser": backupBrowserMd,
  "db-backups": dbBackupsMd,
  "tga-converter": tgaConverterMd,
  "icon-browser": iconBrowserMd,
  "model-viewer": modelViewerMd,
  "system-installer": systemInstallerMd,
  settings: settingsMd,
};

type NavEntry = { id: string; label: string } | { label: string; children: { id: string; label: string }[] };

// Gruppierung + Beschriftung 1:1 aus store/navigation.ts (CATEGORY_ORDER/
// CATEGORY_LABEL_KEYS) und den echten Sidebar-Labels aus i18n/locales/de.json
// übernommen - eigenes Fenster braucht kein i18n (wie beim Quest-Wiki auch),
// die Strings hier sind bewusst die gleichen wie im Hauptfenster.
const NAV: NavEntry[] = [
  { id: "intro", label: "Einführung" },
  {
    label: "Übersicht",
    children: [
      { id: "dashboard", label: "Dashboard" },
      { id: "server-control", label: "Server-Steuerung" },
      { id: "build-deploy", label: "Quellcode Bauen & Einspielen" },
    ],
  },
  {
    label: "Items & Ausrüstung",
    children: [
      { id: "item-editor", label: "Item Editor" },
      { id: "item-proto-explorer", label: "Item-Proto-Explorer" },
      { id: "module-importer", label: "Modul-Importer" },
      { id: "refine-editor", label: "Aufwertungs-Editor" },
      { id: "box-editor", label: "Kisten-Editor" },
      { id: "cube-editor", label: "Cube-Editor" },
    ],
  },
  {
    label: "Shops & Monster",
    children: [
      { id: "shop-editor", label: "Shop-Editor" },
      { id: "mob-proto-editor", label: "Mob-Proto-Editor" },
      { id: "mob-drop-editor", label: "Mob Drop Editor" },
      { id: "drop-generator", label: "Drop-Generator" },
    ],
  },
  {
    label: "Quests & Welt",
    children: [
      { id: "quest-builder", label: "Quest Builder" },
      { id: "regen-editor", label: "Regen-Datei-Editor" },
      { id: "locale-editor", label: "Locale-Verwaltung" },
    ],
  },
  {
    label: "Server & Accounts",
    children: [
      { id: "server-events", label: "Server-Events" },
      { id: "account-manager", label: "Account-Verwaltung" },
      { id: "gm-manager", label: "GM-Verwaltung" },
      { id: "db-explorer", label: "Datenbank-Explorer" },
      { id: "broadcast-system", label: "Broadcast-System" },
      { id: "weather-control", label: "Tag/Nacht & Schnee" },
    ],
  },
  {
    label: "Backups",
    children: [
      { id: "backup-browser", label: "Backup-Browser" },
      { id: "db-backups", label: "Datenbank-Backups" },
    ],
  },
  {
    label: "Client-Assets",
    children: [
      { id: "tga-converter", label: "TGA Konverter" },
      { id: "icon-browser", label: "Icon-Browser" },
      { id: "model-viewer", label: "3D-Modell-Viewer" },
    ],
  },
  {
    label: "Systeme",
    children: [{ id: "system-installer", label: "System-Installer" }],
  },
  { id: "settings", label: "Einstellungen" },
];

function filterNav(nav: NavEntry[], query: string): NavEntry[] {
  if (!query.trim()) return nav;
  const q = query.trim().toLowerCase();
  const matches = (id: string, label: string) =>
    label.toLowerCase().includes(q) || (PAGES[id] ?? "").toLowerCase().includes(q);

  return nav
    .map((entry) => {
      if ("children" in entry) {
        const children = entry.children.filter((c) => matches(c.id, c.label));
        return children.length > 0 ? { ...entry, children } : null;
      }
      return matches(entry.id, entry.label) ? entry : null;
    })
    .filter((e): e is NavEntry => e !== null);
}

// Deep-Link beim ersten Öffnen: openManual(section) übergibt die Ziel-Seite
// als URL-Hash (`manual.html#item-editor`), siehe src/lib/manual.ts.
function initialSectionFromHash(): string {
  const hash = window.location.hash.replace(/^#/, "");
  return hash && PAGES[hash] ? hash : "intro";
}

export function Manual() {
  const [current, setCurrent] = useState(initialSectionFromHash);
  const [query, setQuery] = useState("");

  // Fenster war schon offen, Nutzer klickt in einem anderen Modul erneut auf
  // "Hilfe" - src/lib/manual.ts sendet dafür statt eines Neuöffnens nur ein
  // Event, hier direkt zur passenden Seite springen.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    // listen() can throw synchronously (not just reject) when there's no
    // real Tauri IPC bridge yet - never let that blank out the whole
    // window, deep-link updates just won't arrive in that case.
    try {
      void listen<{ section: string }>("manual-navigate", (event) => {
        const { section } = event.payload;
        if (PAGES[section]) setCurrent(section);
      }).then((f) => {
        unlisten = f;
      });
    } catch {
      // ignore
    }
    return () => unlisten?.();
  }, []);

  const filteredNav = useMemo(() => filterNav(NAV, query), [query]);
  const currentContent = PAGES[current] ?? "";

  return (
    <div className="flex h-screen bg-background text-foreground">
      <aside className="flex w-64 shrink-0 flex-col gap-2 overflow-y-auto border-r border-border p-3">
        <div className="flex items-center gap-2 px-1 pb-1">
          <BookOpen className="size-5 text-muted-foreground" />
          <span className="font-semibold">Handbuch</span>
        </div>
        <div className="relative">
          <Search className="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Durchsuchen…"
            className="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
          />
        </div>
        <nav className="flex-1 space-y-0.5">
          {filteredNav.map((entry) =>
            "children" in entry ? (
              <div key={entry.label} className="pt-2">
                <div className="px-2 pb-1 text-xs font-medium text-muted-foreground">{entry.label}</div>
                {entry.children.map((child) => (
                  <button
                    key={child.id}
                    onClick={() => setCurrent(child.id)}
                    className={`block w-full rounded-md px-2 py-1 text-left text-sm ${
                      current === child.id ? "bg-primary text-primary-foreground" : "hover:bg-muted"
                    }`}
                  >
                    {child.label}
                  </button>
                ))}
              </div>
            ) : (
              <button
                key={entry.id}
                onClick={() => setCurrent(entry.id)}
                className={`block w-full rounded-md px-2 py-1 text-left text-sm ${
                  current === entry.id ? "bg-primary text-primary-foreground" : "hover:bg-muted"
                }`}
              >
                {entry.label}
              </button>
            ),
          )}
          {filteredNav.length === 0 && <p className="px-2 py-1 text-sm text-muted-foreground">Keine Treffer.</p>}
        </nav>
      </aside>

      <main className="min-w-0 flex-1 overflow-y-auto">
        <div className="mx-auto max-w-3xl px-8 py-8">
          <ManualMarkdown content={currentContent} />
        </div>
      </main>
    </div>
  );
}

// Gleiches Rendering-Muster wie QuestWiki.tsx's WikiMarkdown - kein
// @tailwindcss/typography-Plugin in diesem Projekt, jedes Element von Hand
// im bestehenden Look gestylt.
function ManualMarkdown({ content }: { content: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        h1: (p) => <h1 className="mb-4 text-2xl font-semibold" {...p} />,
        h2: (p) => <h2 className="mb-3 mt-8 text-xl font-semibold" {...p} />,
        h3: (p) => <h3 className="mb-2 mt-6 text-lg font-medium" {...p} />,
        p: (p) => <p className="mb-4 text-sm leading-relaxed" {...p} />,
        ul: (p) => <ul className="mb-4 list-disc space-y-1 pl-6 text-sm" {...p} />,
        ol: (p) => <ol className="mb-4 list-decimal space-y-1 pl-6 text-sm" {...p} />,
        a: (p) => <a className="text-primary underline" {...p} />,
        blockquote: (p) => (
          <blockquote className="mb-4 border-l-2 border-border pl-3 text-sm text-muted-foreground" {...p} />
        ),
        code: ({ className, children, ...rest }) => {
          const isBlock = /language-/.test(className ?? "");
          if (isBlock) {
            return (
              <code className={`font-mono text-xs ${className ?? ""}`} {...rest}>
                {children}
              </code>
            );
          }
          return (
            <code className="rounded bg-muted px-1 py-0.5 font-mono text-xs" {...rest}>
              {children}
            </code>
          );
        },
        pre: (p) => <pre className="mb-4 overflow-x-auto rounded-md border border-border bg-muted/40 p-3" {...p} />,
        table: (p) => (
          <div className="mb-4 overflow-x-auto">
            <table className="w-full border-collapse text-sm" {...p} />
          </div>
        ),
        th: (p) => <th className="border border-border bg-muted/40 px-2 py-1 text-left font-medium" {...p} />,
        td: (p) => <td className="border border-border px-2 py-1" {...p} />,
        hr: (p) => <hr className="my-6 border-border" {...p} />,
      }}
    >
      {content}
    </ReactMarkdown>
  );
}
