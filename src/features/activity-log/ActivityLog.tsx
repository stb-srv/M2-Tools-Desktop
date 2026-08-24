import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { NAV_ITEMS, useNavigationStore } from "@/store/navigation";
import { HelpCircle, Search, ChevronLeft, ChevronRight, Undo2, AlertTriangle, ExternalLink } from "lucide-react";
import { openManual } from "@/lib/manual";

interface UndoRef {
  kind: "rollback" | "undo_import";
  id: number;
}

interface ActivityEntry {
  id: string;
  created_at: string;
  module: string;
  action: string;
  summary: string;
  source: "log" | "deploy" | "import";
  undo: UndoRef | null;
}

interface ActivityFeedPage {
  entries: ActivityEntry[];
  total: number;
}

const PAGE_SIZE = 30;

// Module, über die dieses Modul selbst nichts protokolliert (keine echten
// Änderungsquellen) - aus der Filterliste ausgeschlossen, damit dort nur
// Module stehen, die je einen Eintrag erzeugen könnten.
const NON_ACTIVITY_SECTIONS = new Set(["dashboard", "settings", "activity-log", "system-installer"]);

export function ActivityLog() {
  const { t } = useTranslation();
  const setSection = useNavigationStore((s) => s.setSection);

  const [moduleFilter, setModuleFilter] = useState("");
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(0);
  const [refreshKey, setRefreshKey] = useState(0);

  const [entries, setEntries] = useState<ActivityEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [undoingId, setUndoingId] = useState<string | null>(null);
  const [confirmUndo, setConfirmUndo] = useState<ActivityEntry | null>(null);
  const [undoError, setUndoError] = useState<string | null>(null);

  async function load() {
    await runAsyncAction(
      () =>
        invoke<ActivityFeedPage>("list_activity_feed", {
          module: moduleFilter || null,
          search: search.trim() || null,
          offset: page * PAGE_SIZE,
          limit: PAGE_SIZE,
        }),
      {
        onStart: () => {
          setLoading(true);
          setError(null);
        },
        onSuccess: (result) => {
          setEntries(result.entries);
          setTotal(result.total);
        },
        onError: setError,
        onFinally: () => setLoading(false),
      },
    );
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [moduleFilter, page, refreshKey]);

  function runSearch() {
    setPage(0);
    setRefreshKey((k) => k + 1);
  }

  async function confirmImportUndo() {
    if (!confirmUndo?.undo) return;
    const entry = confirmUndo;
    setConfirmUndo(null);
    await runAsyncAction(() => invoke("undo_import_batch", { id: entry.undo!.id }), {
      onStart: () => {
        setUndoingId(entry.id);
        setUndoError(null);
      },
      onSuccess: () => setRefreshKey((k) => k + 1),
      onError: setUndoError,
      onFinally: () => setUndoingId(null),
    });
  }

  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  function moduleLabel(module: string): string {
    const item = NAV_ITEMS.find((n) => n.section === module);
    return item ? t(item.labelKey) : module;
  }

  return (
    <div className="flex h-full flex-col gap-3">
      <div>
        <div className="flex items-center gap-2">
          <h1 className="text-2xl font-semibold">{t("nav.activityLog")}</h1>
          <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("activity-log")}>
            <HelpCircle className="size-4" />
          </Button>
        </div>
        <p className="text-sm text-muted-foreground">
          Gemeinsame Zeitleiste aller Änderungen über die App - Item-/Shop-/Mob-Drop-/Quest-Edits,
          Account-/GM-Aktionen, Server-Events und mehr, zusammen mit den bereits bestehenden
          Verläufen für „Bauen & Einspielen" und den Modul-Importer.
        </p>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <select
          value={moduleFilter}
          onChange={(e) => {
            setModuleFilter(e.target.value);
            setPage(0);
          }}
          className="rounded-md border border-border bg-background px-2 py-1 text-sm"
        >
          <option value="">Alle Module</option>
          {NAV_ITEMS.filter((n) => !NON_ACTIVITY_SECTIONS.has(n.section)).map((n) => (
            <option key={n.section} value={n.section}>
              {t(n.labelKey)}
            </option>
          ))}
        </select>
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && runSearch()}
          placeholder="Volltextsuche über die Zusammenfassung…"
          className="flex-1 min-w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
        />
        <Button variant="outline" onClick={runSearch} disabled={loading}>
          <Search className="size-4" />
        </Button>
      </div>

      {error && <p className="text-sm text-destructive">{error}</p>}
      {undoError && <p className="text-sm text-destructive">{undoError}</p>}

      <div className="flex-1 space-y-1 overflow-y-auto rounded-md border border-border p-1">
        {loading && entries.length === 0 && <p className="p-2 text-sm text-muted-foreground">Lade…</p>}
        {!loading && entries.length === 0 && !error && (
          <p className="p-2 text-sm text-muted-foreground">Keine Einträge.</p>
        )}
        {entries.map((entry) => (
          <div
            key={entry.id}
            className="flex flex-wrap items-center justify-between gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-muted"
          >
            <div className="flex min-w-0 flex-1 items-center gap-2">
              <span className="shrink-0 text-xs text-muted-foreground">
                {new Date(entry.created_at).toLocaleString()}
              </span>
              <span className="shrink-0 rounded-full border border-border px-1.5 py-0.5 text-xs text-muted-foreground">
                {moduleLabel(entry.module)}
              </span>
              <span className="truncate">{entry.summary}</span>
            </div>
            {entry.undo?.kind === "undo_import" && (
              <Button
                size="sm"
                variant="outline"
                disabled={undoingId === entry.id}
                onClick={() => setConfirmUndo(entry)}
              >
                <Undo2 className="size-3.5" />
                {undoingId === entry.id ? "Entferne…" : "Rückgängig machen"}
              </Button>
            )}
            {entry.undo?.kind === "rollback" && (
              <Button size="sm" variant="outline" onClick={() => setSection("build-deploy")}>
                <ExternalLink className="size-3.5" />
                In Bauen & Einspielen öffnen
              </Button>
            )}
          </div>
        ))}
      </div>

      {total > 0 && (
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>
            {total.toLocaleString("de-DE")} Treffer
            {totalPages > 1 && ` · Seite ${page + 1}/${totalPages}`}
          </span>
          {totalPages > 1 && (
            <div className="flex gap-1">
              <Button
                variant="outline"
                size="icon-sm"
                onClick={() => setPage((p) => Math.max(0, p - 1))}
                disabled={page === 0 || loading}
              >
                <ChevronLeft className="size-3.5" />
              </Button>
              <Button
                variant="outline"
                size="icon-sm"
                onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
                disabled={page >= totalPages - 1 || loading}
              >
                <ChevronRight className="size-3.5" />
              </Button>
            </div>
          )}
        </div>
      )}

      {confirmUndo && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>{confirmUndo.summary} - endgültig rückgängig machen?</span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setConfirmUndo(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={confirmImportUndo}>
                Endgültig entfernen
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
