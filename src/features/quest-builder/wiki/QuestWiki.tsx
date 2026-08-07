import { useMemo, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkFrontmatter from "remark-frontmatter";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Search, BookOpen, ExternalLink } from "lucide-react";

import introMd from "./docs/intro.md?raw";
import gettingStartedMd from "./docs/getting-started.md?raw";
import coreConceptsMd from "./docs/core-concepts.md?raw";
import userInterfaceMd from "./docs/user-interface.md?raw";
import advancedTopicsMd from "./docs/advanced-topics.md?raw";
import bestPracticesMd from "./docs/best-practices.md?raw";
import referenceMd from "./docs/reference.md?raw";
import playerApiMd from "./docs/api-reference/player-api.md?raw";
import npcApiMd from "./docs/api-reference/npc-api.md?raw";
import itemApiMd from "./docs/api-reference/item-api.md?raw";
import guildApiMd from "./docs/api-reference/guild-api.md?raw";
import gameApiMd from "./docs/api-reference/game-api.md?raw";
import partyApiMd from "./docs/api-reference/party-api.md?raw";
import dungeonApiMd from "./docs/api-reference/dungeon-api.md?raw";
import otherApisMd from "./docs/api-reference/other-apis.md?raw";

// Inhalt 1:1 übernommen aus https://github.com/wielandino/metin2-quest-docs
// (gerendert unter https://wielandino.github.io/metin2-quest-docs/) - siehe
// Quellenangabe am Ende jeder Seite. Das Repo hat keine LIZENZ-Datei; diese
// Kopie liegt nur in diesem privaten Projekt zum eigenen Nachschlagen, mit
// klarer Quellenangabe, nicht zur Weiterverbreitung gedacht.
const PAGES: Record<string, string> = {
  intro: introMd,
  "getting-started": gettingStartedMd,
  "core-concepts": coreConceptsMd,
  "api-reference/player-api": playerApiMd,
  "api-reference/npc-api": npcApiMd,
  "api-reference/item-api": itemApiMd,
  "api-reference/guild-api": guildApiMd,
  "api-reference/game-api": gameApiMd,
  "api-reference/party-api": partyApiMd,
  "api-reference/dungeon-api": dungeonApiMd,
  "api-reference/other-apis": otherApisMd,
  "user-interface": userInterfaceMd,
  "advanced-topics": advancedTopicsMd,
  "best-practices": bestPracticesMd,
  reference: referenceMd,
};

type NavEntry = { id: string; label: string } | { label: string; children: { id: string; label: string }[] };

// Reihenfolge/Gruppierung 1:1 aus der echten sidebars.js des Original-Repos
// übernommen, nur die Beschriftungen sind eigene deutsche Kurztitel.
const NAV: NavEntry[] = [
  { id: "intro", label: "Einführung" },
  { id: "getting-started", label: "Erste Schritte" },
  { id: "core-concepts", label: "Grundkonzepte" },
  {
    label: "API-Referenz",
    children: [
      { id: "api-reference/player-api", label: "Player API (pc.*)" },
      { id: "api-reference/npc-api", label: "NPC API" },
      { id: "api-reference/item-api", label: "Item API" },
      { id: "api-reference/guild-api", label: "Guild API" },
      { id: "api-reference/game-api", label: "Game API" },
      { id: "api-reference/party-api", label: "Party API" },
      { id: "api-reference/dungeon-api", label: "Dungeon API" },
      { id: "api-reference/other-apis", label: "Weitere APIs" },
    ],
  },
  { id: "user-interface", label: "Benutzeroberfläche" },
  { id: "advanced-topics", label: "Fortgeschrittene Themen" },
  { id: "best-practices", label: "Best Practices" },
  { id: "reference", label: "Kurzreferenz" },
];

function flattenNav(nav: NavEntry[]): { id: string; label: string }[] {
  return nav.flatMap((entry) => ("children" in entry ? entry.children : [entry]));
}

const ALL_PAGES = flattenNav(NAV);

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

export function QuestWiki() {
  const [current, setCurrent] = useState("intro");
  const [query, setQuery] = useState("");

  const filteredNav = useMemo(() => filterNav(NAV, query), [query]);
  const currentLabel = ALL_PAGES.find((p) => p.id === current)?.label ?? current;
  const currentContent = PAGES[current] ?? "";

  return (
    <div className="flex h-screen bg-background text-foreground">
      <aside className="flex w-64 shrink-0 flex-col gap-2 overflow-y-auto border-r border-border p-3">
        <div className="flex items-center gap-2 px-1 pb-1">
          <BookOpen className="size-5 text-muted-foreground" />
          <span className="font-semibold">Quest-Wiki</span>
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
          <WikiMarkdown content={currentContent} />

          <footer className="mt-12 border-t border-border pt-4 text-xs text-muted-foreground">
            <p>
              Inhalt ("{currentLabel}") 1:1 übernommen aus dem{" "}
              <button
                className="underline"
                onClick={() => openUrl("https://github.com/wielandino/metin2-quest-docs")}
              >
                Metin2 Quest Docs-Repository von wielandino
                <ExternalLink className="ml-0.5 inline size-3" />
              </button>{" "}
              (gerendert unter{" "}
              <button
                className="underline"
                onClick={() => openUrl("https://wielandino.github.io/metin2-quest-docs/intro/")}
              >
                wielandino.github.io/metin2-quest-docs
                <ExternalLink className="ml-0.5 inline size-3" />
              </button>
              ). Alle Rechte am Originaltext liegen beim Autor - diese Kopie dient nur dem eigenen Nachschlagen
              innerhalb dieses Projekts.
            </p>
          </footer>
        </div>
      </main>
    </div>
  );
}

// Strips the Docusaurus YAML-frontmatter block (--- sidebar_position: N ---)
// via remark-frontmatter (parses it out of the AST instead of a fragile
// regex) and renders the rest with the app's own look instead of an
// unstyled default - no @tailwindcss/typography plugin in this project,
// same "hand-style each element" convention already used everywhere else.
function WikiMarkdown({ content }: { content: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkFrontmatter, remarkGfm]}
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
        pre: (p) => (
          <pre className="mb-4 overflow-x-auto rounded-md border border-border bg-muted/40 p-3" {...p} />
        ),
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
