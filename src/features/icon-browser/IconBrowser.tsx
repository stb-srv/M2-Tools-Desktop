import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import { Search, Images, ChevronLeft, ChevronRight } from "lucide-react";

interface IconFile {
  absolute_path: string;
  file_name: string;
}

const PAGE_SIZE = 120;

/**
 * Shared grid: lists every .tga under the client's item-icon folder
 * (`list_item_icon_files`, backend walks recursively) and lazy-loads a
 * thumbnail per visible cell only (`load_icon_file`) - with icon counts
 * potentially in the thousands, decoding everything up front would be both
 * slow and a large one-shot IPC payload.
 */
function IconGrid({ onPick }: { onPick?: (absolutePath: string) => void }) {
  const [files, setFiles] = useState<IconFile[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(0);
  const [thumbs, setThumbs] = useState<Record<string, string | null>>({});

  useEffect(() => {
    runAsyncAction(() => invoke<IconFile[]>("list_item_icon_files"), {
      onSuccess: setFiles,
      onError: setError,
    });
  }, []);

  const filtered = useMemo(() => {
    if (!files) return [];
    const q = query.trim().toLowerCase();
    return q ? files.filter((f) => f.file_name.toLowerCase().includes(q)) : files;
  }, [files, query]);

  const pageCount = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const clampedPage = Math.min(page, pageCount - 1);
  const visible = filtered.slice(clampedPage * PAGE_SIZE, (clampedPage + 1) * PAGE_SIZE);

  useEffect(() => {
    setPage(0);
  }, [query]);

  useEffect(() => {
    for (const f of visible) {
      if (f.absolute_path in thumbs) continue;
      invoke<string>("load_icon_file", { absolutePath: f.absolute_path })
        .then((dataUrl) => setThumbs((prev) => ({ ...prev, [f.absolute_path]: dataUrl })))
        .catch(() => setThumbs((prev) => ({ ...prev, [f.absolute_path]: null })));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);

  if (error) return <p className="text-sm text-destructive">{error}</p>;
  if (!files) return <p className="text-sm text-muted-foreground">Lade Icons…</p>;

  return (
    <div className="flex h-full flex-col gap-2">
      <div className="flex items-center gap-2">
        <div className="relative flex-1">
          <Search className="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Dateiname filtern…"
            className="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
          />
        </div>
        <span className="shrink-0 text-xs text-muted-foreground">
          {filtered.length.toLocaleString("de-DE")} Icons
        </span>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-8 gap-1 overflow-y-auto sm:grid-cols-10">
        {visible.map((f) => (
          <button
            key={f.absolute_path}
            onClick={() => onPick?.(f.absolute_path)}
            title={f.file_name}
            className="flex flex-col items-center gap-0.5 rounded-md border border-border p-1 hover:bg-muted"
          >
            <div className="flex size-9 items-center justify-center bg-muted/40">
              {thumbs[f.absolute_path] ? (
                <img
                  src={thumbs[f.absolute_path]!}
                  alt=""
                  className="max-h-full max-w-full object-contain [image-rendering:pixelated]"
                />
              ) : (
                <Images className="size-4 text-muted-foreground" />
              )}
            </div>
            <span className="w-full truncate text-[9px] text-muted-foreground">{f.file_name}</span>
          </button>
        ))}
        {visible.length === 0 && (
          <p className="col-span-full p-4 text-center text-sm text-muted-foreground">
            Keine Icons gefunden.
          </p>
        )}
      </div>

      {pageCount > 1 && (
        <div className="flex items-center justify-center gap-2 text-sm">
          <Button
            variant="outline"
            size="icon-sm"
            onClick={() => setPage((p) => Math.max(0, p - 1))}
            disabled={clampedPage === 0}
          >
            <ChevronLeft className="size-3.5" />
          </Button>
          <span className="text-xs text-muted-foreground">
            Seite {clampedPage + 1} / {pageCount}
          </span>
          <Button
            variant="outline"
            size="icon-sm"
            onClick={() => setPage((p) => Math.min(pageCount - 1, p + 1))}
            disabled={clampedPage >= pageCount - 1}
          >
            <ChevronRight className="size-3.5" />
          </Button>
        </div>
      )}
    </div>
  );
}

/** Standalone nav page - pure browsing, no selection action. */
export function IconBrowser() {
  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center gap-2">
        <Images className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Icon-Browser</h1>
      </div>
      <p className="text-sm text-muted-foreground">
        Durchblättert alle Item-Icons (<code>pack/icon/icon/item/**.tga</code>) im konfigurierten
        Client-Ordner - nützlich um ein Icon nach Aussehen zu finden, statt die vnum-Zuordnung
        schon zu kennen. Zum tatsächlichen Auswählen eines Icons beim Anlegen eines Items siehe
        Item Editor.
      </p>
      <div className="min-h-0 flex-1">
        <IconGrid />
      </div>
    </div>
  );
}

/** Embedded picker modal - used by the Item Editor's icon step. */
export function IconBrowserModal({
  onPick,
  onClose,
}: {
  onPick: (absolutePath: string) => void;
  onClose: () => void;
}) {
  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50" onClick={onClose}>
      <div
        className="flex h-[70vh] w-[48rem] flex-col gap-2 rounded-lg border border-border bg-card p-4"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between">
          <p className="text-sm font-medium">Icon auswählen</p>
          <Button variant="ghost" size="sm" onClick={onClose}>
            Schließen
          </Button>
        </div>
        <div className="min-h-0 flex-1">
          <IconGrid
            onPick={(path) => {
              onPick(path);
              onClose();
            }}
          />
        </div>
      </div>
    </div>
  );
}
