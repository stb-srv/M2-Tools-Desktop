import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { diffLines, type Change } from "diff";
import { runAsyncAction } from "@/lib/asyncAction";
import { Button } from "@/components/ui/button";
import {
  FolderOpen,
  Search,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  RefreshCw,
  History,
  Sparkles,
  ChevronDown,
  ChevronRight,
} from "lucide-react";

// Spiegeln system_patch.rs/system_installs.rs 1:1 - keine camelCase-
// Umbenennung, dieselbe Konvention wie überall sonst in diesem Projekt
// (siehe z.B. RegenSpawn in regen-editor).

type Placement = "Above" | "Below" | "Inside" | "AtEnd";
type AnchorConfidence = "Exact" | "WhitespaceTolerant" | "CommentStripped" | "Partial";

type PatchOp =
  | { kind: "SearchInsert"; scope: string | null; anchor: string; placement: Placement; code: string }
  | { kind: "FreeformInstruction"; instruction: string; code: string }
  | { kind: "AppendToEnd"; code: string };

interface ParsedSystemFile {
  ops: PatchOp[];
}

type TargetKind = "LiveServer" | "LocalClientSource" | "LocalClientInstall";

interface ScannedFile {
  relative_path: string;
  addon_name: string | null;
  category: TargetKind;
  filename: string;
  parsed: ParsedSystemFile;
  raw_content: string | null;
}

type InsertionResolution =
  | { kind: "Ready"; line: number; confidence: AnchorConfidence }
  | { kind: "NeedsReview"; reason: string };

type FileAction = "Patched" | "Created";

interface InstalledFile {
  target_path: string;
  target_kind: TargetKind;
  backup_path: string | null;
  action: FileAction;
}

interface SystemInstall {
  id: number;
  system_name: string;
  created_at: string;
  files: InstalledFile[];
}

interface ApplyInstallResult {
  install_id: number;
  warnings: string[];
}

const CATEGORY_LABELS: Record<TargetKind, string> = {
  LiveServer: "Server-Quellcode (live)",
  LocalClientSource: "Client-Quellcode (lokal)",
  LocalClientInstall: "Client-Installationsdatei",
};

const CONFIDENCE_LABELS: Record<AnchorConfidence, string> = {
  Exact: "exakt gefunden",
  WhitespaceTolerant: "gefunden (Einrückung ignoriert)",
  CommentStripped: "gefunden (Kommentar-Inhalt ignoriert)",
  Partial: "nur Teiltreffer - bitte prüfen",
};

// Ein Op-Ergebnis, an dem Index in file.parsed.ops ausgerichtet - `null`
// heißt "noch nicht aufgelöst" (Zielinhalt noch nicht geladen).
type OpStatus =
  | { done: false }
  | { done: true; kind: "ready"; resolution: InsertionResolution }
  | { done: true; kind: "freeform" }
  | { done: true; kind: "conflict" }; // Ganzdatei-Kandidat, Ziel existiert bereits

interface FileWorkItem {
  file: ScannedFile;
  targetPath: string;
  targetCandidates: string[];
  targetSearching: boolean;
  targetContent: string | null; // null = noch nicht geladen ODER Datei existiert nicht
  targetLoaded: boolean;
  opStatuses: OpStatus[];
}

function keyFor(file: ScannedFile): string {
  return file.relative_path;
}

function isWholeFileCandidate(file: ScannedFile): boolean {
  return file.parsed.ops.length === 0;
}

function computeFileStatus(item: FileWorkItem): "loading" | "ready" | "review" {
  if (isWholeFileCandidate(item.file)) {
    if (!item.targetLoaded) return "loading";
    return item.targetContent === null ? "ready" : "review"; // review = Ziel existiert schon, Konflikt
  }
  if (item.targetCandidates.length !== 1) return item.targetSearching ? "loading" : "review";
  if (!item.targetLoaded) return "loading";
  if (item.opStatuses.length !== item.file.parsed.ops.length) return "loading";
  if (item.opStatuses.some((s) => !s.done)) return "loading";
  const allReady = item.opStatuses.every((s) => s.done && s.kind === "ready");
  return allReady ? "ready" : "review";
}

function buildAppendOp(code: string): PatchOp {
  return { kind: "AppendToEnd", code };
}

export function SystemInstaller() {
  const [importRoot, setImportRoot] = useState<string | null>(null);
  const [systemName, setSystemName] = useState("");
  const [scanning, setScanning] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const [items, setItems] = useState<Record<string, FileWorkItem> | null>(null);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});

  const [applying, setApplying] = useState(false);
  const [applyError, setApplyError] = useState<string | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyInstallResult | null>(null);

  const [historyOpen, setHistoryOpen] = useState(false);
  const [installs, setInstalls] = useState<SystemInstall[] | null>(null);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [undoingId, setUndoingId] = useState<number | null>(null);
  const [undoConfirm, setUndoConfirm] = useState<number | null>(null);

  async function pickAndScan() {
    const selected = await open({ directory: true, title: "System-Ordner wählen" });
    if (!selected || typeof selected !== "string") return;
    setImportRoot(selected);
    const guessedName = selected.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? "System";
    setSystemName(guessedName);
    await scanFolder(selected);
  }

  async function scanFolder(root: string) {
    setScanning(true);
    setScanError(null);
    setItems(null);
    setApplyResult(null);
    setApplyError(null);
    try {
      const files = await invoke<ScannedFile[]>("scan_system_package", { root });
      const map: Record<string, FileWorkItem> = {};
      for (const file of files) {
        map[keyFor(file)] = {
          file,
          targetPath: "",
          targetCandidates: [],
          targetSearching: false,
          targetContent: null,
          targetLoaded: false,
          opStatuses: file.parsed.ops.map(() => ({ done: false }) as OpStatus),
        };
      }
      setItems(map);
      // Zielsuche für jede Datei parallel anstoßen, statt auf Klick zu warten.
      await Promise.all(Object.keys(map).map((key) => searchTarget(key, map)));
    } catch (e) {
      setScanError(String(e));
    } finally {
      setScanning(false);
    }
  }

  async function searchTarget(key: string, snapshot?: Record<string, FileWorkItem>) {
    const current = (snapshot ?? items)?.[key];
    if (!current) return;
    setItems((prev) => (prev ? { ...prev, [key]: { ...prev[key], targetSearching: true } } : prev));
    try {
      const candidates = await invoke<string[]>("find_system_target", {
        category: current.file.category,
        filename: current.file.filename,
      });
      setItems((prev) => {
        if (!prev) return prev;
        return { ...prev, [key]: { ...prev[key], targetCandidates: candidates, targetSearching: false } };
      });
      if (candidates.length === 1) {
        await setTargetPath(key, candidates[0]);
      }
    } catch (e) {
      setItems((prev) =>
        prev ? { ...prev, [key]: { ...prev[key], targetSearching: false, targetCandidates: [] } } : prev,
      );
      void e;
    }
  }

  async function setTargetPath(key: string, path: string) {
    setItems((prev) =>
      prev ? { ...prev, [key]: { ...prev[key], targetPath: path, targetLoaded: false } } : prev,
    );
    const current = items?.[key];
    const category = current?.file.category;
    if (!category) return;
    try {
      const content = await invoke<string | null>("read_system_target_file", { category, path });
      setItems((prev) => {
        if (!prev) return prev;
        return { ...prev, [key]: { ...prev[key], targetContent: content, targetLoaded: true } };
      });
      await resolveOps(key, content);
    } catch {
      setItems((prev) =>
        prev ? { ...prev, [key]: { ...prev[key], targetContent: null, targetLoaded: true } } : prev,
      );
    }
  }

  async function resolveOps(key: string, content: string | null) {
    const current = items?.[key];
    if (!current) return;
    if (isWholeFileCandidate(current.file)) return; // kein Op aufzulösen, nur Konflikt-Check über targetContent

    const statuses = await Promise.all(
      current.file.parsed.ops.map(async (op): Promise<OpStatus> => {
        if (op.kind === "FreeformInstruction") return { done: true, kind: "freeform" };
        if (op.kind === "AppendToEnd") {
          return { done: true, kind: "ready", resolution: { kind: "Ready", line: -1, confidence: "Exact" } };
        }
        const resolution = await invoke<InsertionResolution>("resolve_system_insertion", {
          haystack: content ?? "",
          scope: op.scope,
          anchor: op.anchor,
          placement: op.placement,
        });
        return { done: true, kind: resolution.kind === "Ready" ? "ready" : "review", resolution } as OpStatus;
      }),
    );
    setItems((prev) => (prev ? { ...prev, [key]: { ...prev[key], opStatuses: statuses } } : prev));
  }

  const allItems = items ? Object.values(items) : [];
  const readyItems = allItems.filter((it) => computeFileStatus(it) === "ready");
  const reviewItems = allItems.filter((it) => computeFileStatus(it) === "review");
  const loadingItems = allItems.filter((it) => computeFileStatus(it) === "loading");

  async function applyReady() {
    if (readyItems.length === 0) return;
    setApplying(true);
    setApplyError(null);
    setApplyResult(null);
    try {
      const files = readyItems.map((it) => {
        const ops: PatchOp[] = isWholeFileCandidate(it.file)
          ? [buildAppendOp(it.file.raw_content ?? "")]
          : it.file.parsed.ops;
        return { target_path: it.targetPath, category: it.file.category, ops };
      });
      const result = await invoke<ApplyInstallResult>("apply_system_install", {
        systemName: systemName || "Unbenanntes System",
        files,
      });
      setApplyResult(result);
      // Erfolgreich angewendete Dateien aus der offenen Liste nehmen, damit
      // nicht aus Versehen ein zweites Mal draufgeklickt wird.
      setItems((prev) => {
        if (!prev) return prev;
        const next = { ...prev };
        for (const it of readyItems) delete next[keyFor(it.file)];
        return next;
      });
    } catch (e) {
      setApplyError(String(e));
    } finally {
      setApplying(false);
    }
  }

  async function loadHistory() {
    await runAsyncAction(() => invoke<SystemInstall[]>("list_system_installs"), {
      onStart: () => {
        setHistoryLoading(true);
        setHistoryError(null);
      },
      onSuccess: setInstalls,
      onError: setHistoryError,
      onFinally: () => setHistoryLoading(false),
    });
  }

  async function confirmUndo(id: number) {
    setUndoingId(id);
    setHistoryError(null);
    try {
      await invoke("undo_system_install", { id });
      setUndoConfirm(null);
      await loadHistory();
    } catch (e) {
      setHistoryError(String(e));
    } finally {
      setUndoingId(null);
    }
  }

  // Nach Addon gruppieren, damit ADDONS als eigene Unterebene sichtbar sind.
  const mainFiles = allItems.filter((it) => !it.file.addon_name);
  const addonGroups = new Map<string, FileWorkItem[]>();
  for (const it of allItems) {
    if (!it.file.addon_name) continue;
    const list = addonGroups.get(it.file.addon_name) ?? [];
    list.push(it);
    addonGroups.set(it.file.addon_name, list);
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">System-Installer</h1>
        <Button
          variant="outline"
          onClick={() => {
            setHistoryOpen((v) => !v);
            if (!historyOpen) void loadHistory();
          }}
        >
          <History className="size-4" />
          Verlauf
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Baut fertige Community-"Systeme" (Server-/Client-Erweiterungen wie ResizeWindow oder ein
        Admin-Panel-Modul) automatisiert ein - erkennt die "search/add"-Konvention, sucht die echte
        Zieldatei und fügt den neuen Code an der richtigen Stelle ein. Mustererkennung, kein echtes
        Sprachverständnis - unsichere Stellen werden klar markiert statt geraten, jede Datei wird vor
        dem Schreiben gesichert, jede Installation lässt sich über den Verlauf wieder rückgängig
        machen.
      </p>

      {historyOpen && (
        <div className="space-y-2 rounded-md border border-border p-3">
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">Installierte Systeme</span>
            <Button variant="ghost" size="icon-sm" onClick={loadHistory}>
              <RefreshCw className="size-3.5" />
            </Button>
          </div>
          {historyLoading && <p className="text-sm text-muted-foreground">Lade…</p>}
          {historyError && <p className="text-sm text-destructive">{historyError}</p>}
          {installs && installs.length === 0 && (
            <p className="text-sm text-muted-foreground">Noch keine Installation durchgeführt.</p>
          )}
          {installs?.map((inst) => (
            <div key={inst.id} className="flex items-center justify-between rounded-md border border-border p-2">
              <div>
                <p className="text-sm font-medium">{inst.system_name}</p>
                <p className="text-xs text-muted-foreground">
                  {new Date(inst.created_at).toLocaleString("de-DE")} · {inst.files.length} Datei(en)
                </p>
              </div>
              {undoConfirm === inst.id ? (
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground">Wirklich rückgängig machen?</span>
                  <Button variant="destructive" size="sm" disabled={undoingId === inst.id} onClick={() => confirmUndo(inst.id)}>
                    {undoingId === inst.id ? "…" : "Ja, rückgängig machen"}
                  </Button>
                  <Button variant="ghost" size="sm" onClick={() => setUndoConfirm(null)}>
                    Abbrechen
                  </Button>
                </div>
              ) : (
                <Button variant="outline" size="sm" onClick={() => setUndoConfirm(inst.id)}>
                  Rückgängig machen
                </Button>
              )}
            </div>
          ))}
        </div>
      )}

      <div className="flex items-center gap-2">
        <Button onClick={pickAndScan} disabled={scanning}>
          <FolderOpen className="size-4" />
          {scanning ? "Lese ein…" : "System-Ordner wählen"}
        </Button>
        {importRoot && (
          <>
            <input
              value={systemName}
              onChange={(e) => setSystemName(e.target.value)}
              placeholder="Name für den Verlauf"
              className="w-64 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <span className="truncate text-xs text-muted-foreground">{importRoot}</span>
          </>
        )}
      </div>
      {scanError && <p className="text-sm text-destructive">{scanError}</p>}

      {items && (
        <div className="space-y-4">
          <div className="flex items-center justify-between rounded-md border border-border p-3">
            <div className="text-sm">
              <span className="font-medium text-green-700 dark:text-green-400">{readyItems.length} bereit</span>
              {" · "}
              <span className="font-medium text-amber-700 dark:text-amber-400">{reviewItems.length} Prüfung nötig</span>
              {loadingItems.length > 0 && (
                <>
                  {" · "}
                  <span className="text-muted-foreground">{loadingItems.length} wird geladen…</span>
                </>
              )}
            </div>
            <Button onClick={applyReady} disabled={applying || readyItems.length === 0}>
              <Sparkles className="size-4" />
              {applying ? "Wende an…" : `${readyItems.length} bereite Blöcke anwenden`}
            </Button>
          </div>
          {applyError && (
            <p className="rounded-md border border-destructive/40 bg-destructive/10 p-2 text-sm text-destructive">
              {applyError}
            </p>
          )}
          {applyResult && (
            <div className="space-y-1 rounded-md border border-green-500/40 bg-green-500/10 p-2 text-sm text-green-700 dark:text-green-400">
              <p>
                Eingebaut (Verlauf-Eintrag #{applyResult.install_id}) - über "Verlauf" jederzeit rückgängig zu
                machen.
              </p>
              {applyResult.warnings.map((w, i) => (
                <p key={i} className="text-amber-700 dark:text-amber-400">
                  ⚠️ {w}
                </p>
              ))}
            </div>
          )}

          <FileGroup title="Hauptsystem" items={mainFiles} expanded={expanded} setExpanded={setExpanded}
            onTargetPathChange={setTargetPath} onResearch={(key) => searchTarget(key)} />
          {[...addonGroups.entries()].map(([name, groupItems]) => (
            <FileGroup key={name} title={`ADDON: ${name}`} items={groupItems} expanded={expanded}
              setExpanded={setExpanded} onTargetPathChange={setTargetPath}
              onResearch={(key) => searchTarget(key)} />
          ))}
        </div>
      )}
    </div>
  );
}

function StatusIcon({ status }: { status: "loading" | "ready" | "review" }) {
  if (status === "loading") return <RefreshCw className="size-4 shrink-0 animate-spin text-muted-foreground" />;
  if (status === "ready") return <CheckCircle2 className="size-4 shrink-0 text-green-600 dark:text-green-400" />;
  return <AlertTriangle className="size-4 shrink-0 text-amber-600 dark:text-amber-400" />;
}

function FileGroup({
  title,
  items,
  expanded,
  setExpanded,
  onTargetPathChange,
  onResearch,
}: {
  title: string;
  items: FileWorkItem[];
  expanded: Record<string, boolean>;
  setExpanded: React.Dispatch<React.SetStateAction<Record<string, boolean>>>;
  onTargetPathChange: (key: string, path: string) => void;
  onResearch: (key: string) => void;
}) {
  if (items.length === 0) return null;
  return (
    <div className="space-y-1">
      <p className="text-xs font-medium text-muted-foreground">
        {title} ({items.length})
      </p>
      {items.map((item) => {
        const key = keyFor(item.file);
        const status = computeFileStatus(item);
        const isOpen = !!expanded[key];
        return (
          <div key={key} className="rounded-md border border-border">
            <button
              className="flex w-full items-center gap-2 px-2 py-1.5 text-left text-sm hover:bg-muted"
              onClick={() => setExpanded((prev) => ({ ...prev, [key]: !prev[key] }))}
            >
              {isOpen ? <ChevronDown className="size-3.5 shrink-0" /> : <ChevronRight className="size-3.5 shrink-0" />}
              <StatusIcon status={status} />
              <span className="truncate font-mono text-xs">{item.file.relative_path}</span>
              <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                {CATEGORY_LABELS[item.file.category]}
              </span>
            </button>
            {isOpen && (
              <FileDetail item={item} onTargetPathChange={(p) => onTargetPathChange(key, p)} onResearch={() => onResearch(key)} />
            )}
          </div>
        );
      })}
    </div>
  );
}

function FileDetail({
  item,
  onTargetPathChange,
  onResearch,
}: {
  item: FileWorkItem;
  onTargetPathChange: (path: string) => void;
  onResearch: () => void;
}) {
  const [manualPath, setManualPath] = useState(item.targetPath);
  const whole = isWholeFileCandidate(item.file);

  return (
    <div className="space-y-3 border-t border-border p-2">
      <div className="flex items-center gap-2">
        <input
          value={manualPath}
          onChange={(e) => setManualPath(e.target.value)}
          placeholder="Zielpfad (z.B. game/src/cmd_gm.cpp)"
          className="flex-1 rounded-md border border-border bg-background px-2 py-1 font-mono text-xs"
        />
        <Button variant="outline" size="sm" onClick={() => onTargetPathChange(manualPath)}>
          Übernehmen
        </Button>
        <Button variant="outline" size="sm" onClick={onResearch} disabled={item.targetSearching}>
          <Search className="size-3.5" />
          Suchen
        </Button>
      </div>
      {item.targetCandidates.length > 1 && (
        <div className="space-y-1 rounded-md border border-amber-500/40 bg-amber-500/10 p-2 text-xs">
          <p className="text-amber-700 dark:text-amber-400">Mehrere Treffer - bitte auswählen:</p>
          {item.targetCandidates.map((c) => (
            <button
              key={c}
              className="block w-full truncate rounded px-1 py-0.5 text-left font-mono hover:bg-muted"
              onClick={() => onTargetPathChange(c)}
            >
              {c}
            </button>
          ))}
        </div>
      )}
      {item.targetCandidates.length === 0 && !item.targetSearching && !item.targetPath && (
        <p className="flex items-center gap-1.5 text-xs text-amber-700 dark:text-amber-400">
          <XCircle className="size-3.5" />
          Zieldatei nicht gefunden - bitte Pfad manuell eintragen oder erneut suchen.
        </p>
      )}

      {whole ? (
        <WholeFilePreview item={item} />
      ) : (
        item.file.parsed.ops.map((op, i) => (
          <OpDetail key={i} op={op} status={item.opStatuses[i]} targetContent={item.targetContent} />
        ))
      )}
    </div>
  );
}

function WholeFilePreview({ item }: { item: FileWorkItem }) {
  if (!item.targetLoaded) {
    return <p className="text-xs text-muted-foreground">Prüfe, ob die Zieldatei schon existiert…</p>;
  }
  if (item.targetContent !== null) {
    return (
      <p className="flex items-center gap-1.5 text-xs text-amber-700 dark:text-amber-400">
        <AlertTriangle className="size-3.5" />
        Zieldatei existiert bereits - wird NICHT automatisch überschrieben. Bitte manuell prüfen/mergen.
      </p>
    );
  }
  return (
    <div className="space-y-1">
      <p className="flex items-center gap-1.5 text-xs text-green-700 dark:text-green-400">
        <CheckCircle2 className="size-3.5" />
        Neue Datei - wird 1:1 angelegt.
      </p>
      <pre className="max-h-48 overflow-auto rounded-md border border-border bg-muted/20 p-2 font-mono text-xs">
        {item.file.raw_content}
      </pre>
    </div>
  );
}

function OpDetail({
  op,
  status,
  targetContent,
}: {
  op: PatchOp;
  status: OpStatus;
  targetContent: string | null;
}) {
  if (op.kind === "FreeformInstruction") {
    return (
      <div className="space-y-1 rounded-md border border-amber-500/40 bg-amber-500/10 p-2 text-xs">
        <p className="flex items-center gap-1.5 text-amber-700 dark:text-amber-400">
          <AlertTriangle className="size-3.5" />
          Freitext-Anweisung, kein fester Suchtext - nicht automatisch anwendbar:
        </p>
        <p className="font-mono">{op.instruction}</p>
        <pre className="max-h-32 overflow-auto rounded bg-muted/40 p-1">{op.code}</pre>
      </div>
    );
  }

  const code = op.code;
  const label = op.kind === "AppendToEnd" ? "Am Dateiende anhängen" : `Einfügen (${op.placement})`;

  if (!status.done) {
    return <p className="text-xs text-muted-foreground">{label} - wird geprüft…</p>;
  }

  if (status.kind === "freeform") return null; // sollte hier nie vorkommen
  if (status.kind === "conflict") return null;

  const resolution = status.resolution;
  const ready = resolution.kind === "Ready";

  return (
    <div
      className={`space-y-1 rounded-md border p-2 text-xs ${
        ready ? "border-green-500/40 bg-green-500/10" : "border-amber-500/40 bg-amber-500/10"
      }`}
    >
      <p
        className={
          ready ? "flex items-center gap-1.5 text-green-700 dark:text-green-400" : "flex items-center gap-1.5 text-amber-700 dark:text-amber-400"
        }
      >
        {ready ? <CheckCircle2 className="size-3.5" /> : <AlertTriangle className="size-3.5" />}
        {label} - {ready ? CONFIDENCE_LABELS[resolution.confidence] : resolution.reason}
      </p>
      {op.kind === "SearchInsert" && (
        <details>
          <summary className="cursor-pointer text-muted-foreground">Suchtext anzeigen</summary>
          <pre className="mt-1 max-h-32 overflow-auto rounded bg-muted/40 p-1">{op.anchor || "(kein eigener Suchtext)"}</pre>
        </details>
      )}
      <details>
        <summary className="cursor-pointer text-muted-foreground">Einzufügender Code</summary>
        <pre className="mt-1 max-h-32 overflow-auto rounded bg-muted/40 p-1">{code}</pre>
      </details>
      {ready && targetContent !== null && op.kind === "SearchInsert" && (
        <details>
          <summary className="cursor-pointer text-muted-foreground">Vorschau (Diff)</summary>
          <PreviewDiff before={targetContent} line={resolution.line} code={code} />
        </details>
      )}
    </div>
  );
}

function PreviewDiff({ before, line, code }: { before: string; line: number; code: string }) {
  const lines = before.split("\n");
  const insertLines = code.split("\n");
  const after = [...lines.slice(0, line), ...insertLines, ...lines.slice(line)].join("\n");
  const parts = diffLines(before, after);
  return (
    <pre className="mt-1 max-h-48 overflow-auto rounded bg-muted/20 p-1 font-mono">
      {parts.map((part: Change, i) => {
        if (!part.added && !part.removed) return null; // nur die Änderung zeigen, nicht die ganze Datei
        const partLines = part.value.replace(/\n$/, "").split("\n");
        const cls = part.added
          ? "bg-green-500/15 text-green-700 dark:text-green-400"
          : "bg-red-500/15 text-red-700 dark:text-red-400";
        return (
          <span key={i}>
            {partLines.map((l, j) => (
              <div key={j} className={cls}>
                {part.added ? "+" : "-"} {l}
              </div>
            ))}
          </span>
        );
      })}
    </pre>
  );
}
