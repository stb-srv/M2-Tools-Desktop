import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { FolderOpen, ImagePlus, X, CheckCircle2, AlertTriangle, HelpCircle } from "lucide-react";
import { openManual } from "@/lib/manual";

type RowStatus = "pending" | "converting" | "done" | "error";

interface ConvertRow {
  sourcePath: string;
  destPath: string;
  preview: string | null;
  status: RowStatus;
  error?: string;
}

const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "bmp", "gif", "tga"];

function splitPath(path: string): { dir: string; filename: string } {
  const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return {
    dir: lastSlash >= 0 ? path.slice(0, lastSlash + 1) : "",
    filename: lastSlash >= 0 ? path.slice(lastSlash + 1) : path,
  };
}

function baseNameNoExt(filename: string): string {
  const dot = filename.lastIndexOf(".");
  return dot > 0 ? filename.slice(0, dot) : filename;
}

function defaultDestPath(sourcePath: string): string {
  const { dir, filename } = splitPath(sourcePath);
  return `${dir}${baseNameNoExt(filename)}.tga`;
}

export function TgaConverter() {
  const [rows, setRows] = useState<ConvertRow[]>([]);
  const [converting, setConverting] = useState(false);

  async function pickFiles() {
    const selected = await open({
      multiple: true,
      title: "Bilder auswählen",
      filters: [{ name: "Bild", extensions: IMAGE_EXTENSIONS }],
    });
    const paths = selected == null ? [] : Array.isArray(selected) ? selected : [selected];
    if (paths.length === 0) return;

    const newRows: ConvertRow[] = paths.map((p) => ({
      sourcePath: p,
      destPath: defaultDestPath(p),
      preview: null,
      status: "pending",
    }));
    setRows((prev) => [...prev, ...newRows]);

    newRows.forEach((row) => {
      invoke<string>("preview_image_file", { path: row.sourcePath })
        .then((dataUrl) =>
          setRows((prev) =>
            prev.map((r) => (r.sourcePath === row.sourcePath ? { ...r, preview: dataUrl } : r)),
          ),
        )
        .catch(() => {});
    });
  }

  async function setDestFolder() {
    const folder = await open({ directory: true, title: "Zielordner für alle Dateien" });
    if (typeof folder !== "string") return;
    setRows((prev) =>
      prev.map((r) => ({
        ...r,
        destPath: `${folder}\\${baseNameNoExt(splitPath(r.sourcePath).filename)}.tga`,
      })),
    );
  }

  function updateDest(index: number, value: string) {
    setRows((prev) => prev.map((r, i) => (i === index ? { ...r, destPath: value } : r)));
  }

  function removeRow(index: number) {
    setRows((prev) => prev.filter((_, i) => i !== index));
  }

  function clearDone() {
    setRows((prev) => prev.filter((r) => r.status !== "done"));
  }

  async function convertRow(index: number) {
    const row = rows[index];
    setRows((prev) => prev.map((r, i) => (i === index ? { ...r, status: "converting" } : r)));
    try {
      await invoke("convert_image_to_tga", { sourcePath: row.sourcePath, destPath: row.destPath });
      setRows((prev) => prev.map((r, i) => (i === index ? { ...r, status: "done" } : r)));
    } catch (e) {
      setRows((prev) =>
        prev.map((r, i) => (i === index ? { ...r, status: "error", error: String(e) } : r)),
      );
    }
  }

  async function convertAll() {
    setConverting(true);
    for (let i = 0; i < rows.length; i++) {
      if (rows[i].status === "done") continue;
      setRows((prev) => prev.map((r, ri) => (ri === i ? { ...r, status: "converting" } : r)));
      try {
        await invoke("convert_image_to_tga", {
          sourcePath: rows[i].sourcePath,
          destPath: rows[i].destPath,
        });
        setRows((prev) => prev.map((r, ri) => (ri === i ? { ...r, status: "done" } : r)));
      } catch (e) {
        setRows((prev) =>
          prev.map((r, ri) => (ri === i ? { ...r, status: "error", error: String(e) } : r)),
        );
      }
    }
    setConverting(false);
  }

  return (
    <div className="max-w-3xl space-y-4">
      <div className="flex items-center gap-2">
        <h1 className="text-2xl font-semibold">TGA Konverter</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("tga-converter")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Wandelt PNG/JPG/BMP/GIF in <code>.tga</code> um (dieselbe Konvertierung, die auch beim
        Icon-Schritt im Item Editor automatisch läuft) — praktisch, um mehrere Icons vorab
        vorzubereiten oder eine Datei einzeln zu prüfen.
      </p>

      <div className="flex gap-2">
        <Button onClick={pickFiles}>
          <ImagePlus className="size-4" />
          Bilder auswählen…
        </Button>
        <Button variant="outline" onClick={setDestFolder} disabled={rows.length === 0}>
          <FolderOpen className="size-4" />
          Zielordner für alle setzen
        </Button>
        <Button
          onClick={convertAll}
          disabled={converting || rows.length === 0 || rows.every((r) => r.status === "done")}
        >
          {converting ? "Konvertiere…" : "Alle konvertieren"}
        </Button>
        {rows.some((r) => r.status === "done") && (
          <Button variant="ghost" onClick={clearDone}>
            Erledigte entfernen
          </Button>
        )}
      </div>

      <div className="space-y-2">
        {rows.map((row, index) => (
          <div key={row.sourcePath + index} className="flex items-center gap-3 rounded-md border border-border p-2">
            <div className="flex size-12 shrink-0 items-center justify-center rounded-md border border-border bg-muted/40">
              {row.preview ? (
                <img src={row.preview} alt="" className="max-h-full max-w-full object-contain" />
              ) : (
                <ImagePlus className="size-4 text-muted-foreground" />
              )}
            </div>
            <div className="min-w-0 flex-1 space-y-1">
              <p className="truncate text-xs text-muted-foreground" title={row.sourcePath}>
                {row.sourcePath}
              </p>
              <input
                value={row.destPath}
                onChange={(e) => updateDest(index, e.target.value)}
                className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
              {row.status === "error" && (
                <p className="flex items-center gap-1 text-xs text-destructive">
                  <AlertTriangle className="size-3.5 shrink-0" /> {row.error}
                </p>
              )}
            </div>
            <div className="flex shrink-0 items-center gap-2">
              {row.status === "done" && <CheckCircle2 className="size-5 text-green-600" />}
              {row.status === "converting" && (
                <span className="size-4 animate-spin rounded-full border-2 border-muted-foreground border-t-transparent" />
              )}
              {row.status === "error" && (
                <Button size="sm" variant="outline" onClick={() => convertRow(index)}>
                  Erneut
                </Button>
              )}
              <Button variant="ghost" size="icon-sm" onClick={() => removeRow(index)}>
                <X className="size-4" />
              </Button>
            </div>
          </div>
        ))}
        {rows.length === 0 && (
          <p className="text-sm text-muted-foreground">Noch keine Dateien ausgewählt.</p>
        )}
      </div>
    </div>
  );
}
