import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { diffLines, type Change } from "diff";
import { runAsyncAction } from "@/lib/asyncAction";
import { logActivity } from "@/lib/logActivity";
import { Button } from "@/components/ui/button";
import {
  Folder,
  FolderArchive,
  FileText,
  ChevronRight,
  Home,
  RefreshCw,
  History,
  Eye,
  AlertTriangle,
  CheckCircle2,
  X,
  HelpCircle,
} from "lucide-react";
import { openManual } from "@/lib/manual";

interface RemoteEntry {
  name: string;
  is_dir: boolean;
}

interface BackupDiff {
  target_path: string;
  backup_content: string;
  current_content: string | null;
  is_binary: boolean;
}

const DEFAULT_ROOT = "/usr/home/game";
const BACKUP_FOLDER_NAME = "m2manager_backups";

// Backup filenames encode when they were made:
//   <name>.<YYYYMMDD_HHMMSS>.bak / .deleted  (see backups.rs)
function parseTimestamp(filename: string): string | null {
  const match = filename.match(/\.(\d{8})_(\d{6})\.(bak|deleted)$/);
  if (!match) return null;
  const [, date, time] = match;
  return `${date.slice(0, 4)}-${date.slice(4, 6)}-${date.slice(6, 8)} ${time.slice(0, 2)}:${time.slice(2, 4)}:${time.slice(4, 6)}`;
}

export function BackupBrowser() {
  const [root, setRoot] = useState<string | null>(null);
  const [dir, setDir] = useState<string | null>(null);
  const [entries, setEntries] = useState<RemoteEntry[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [restoreConfirm, setRestoreConfirm] = useState<string | null>(null);
  const [restoring, setRestoring] = useState(false);
  const [restoreError, setRestoreError] = useState<string | null>(null);
  const [restoreOk, setRestoreOk] = useState<string | null>(null);

  const [diffPath, setDiffPath] = useState<string | null>(null);
  const [diffData, setDiffData] = useState<BackupDiff | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  const [diffError, setDiffError] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      const workdir =
        (await invoke<string | null>("get_setting", { key: "server_workdir" }).catch(
          () => null,
        )) || DEFAULT_ROOT;
      setRoot(workdir);
      setDir(workdir);
    })();
  }, []);

  useEffect(() => {
    if (dir !== null) loadDir(dir);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dir]);

  async function loadDir(path: string) {
    await runAsyncAction(() => invoke<RemoteEntry[]>("list_remote_dir", { path }), {
      onStart: () => {
        setLoading(true);
        setError(null);
      },
      onSuccess: setEntries,
      onError: setError,
      onFinally: () => setLoading(false),
    });
  }

  function openFolder(name: string) {
    setRestoreOk(null);
    setDir((prev) => `${prev}/${name}`);
  }

  function goUp() {
    setDir((prev) => {
      if (!prev || !root || prev.length <= root.length) return prev;
      const idx = prev.lastIndexOf("/");
      return idx === -1 ? root : prev.slice(0, idx);
    });
  }

  async function openDiff(backupPath: string) {
    setDiffPath(backupPath);
    setDiffData(null);
    await runAsyncAction(() => invoke<BackupDiff>("diff_remote_backup", { backupPath }), {
      onStart: () => {
        setDiffLoading(true);
        setDiffError(null);
      },
      onSuccess: setDiffData,
      onError: setDiffError,
      onFinally: () => setDiffLoading(false),
    });
  }

  async function confirmRestore() {
    if (!restoreConfirm) return;
    const backupPath = restoreConfirm;
    setRestoreConfirm(null);
    await runAsyncAction(
      () => invoke<string>("restore_remote_backup", { backupPath }),
      {
        onStart: () => {
          setRestoring(true);
          setRestoreError(null);
        },
        onSuccess: (target) => {
          setRestoreOk(`Wiederhergestellt nach: ${target}`);
          logActivity("backup-browser", "restore", `Backup wiederhergestellt: '${backupPath}' → '${target}'`, "file", target);
        },
        onError: setRestoreError,
        onFinally: () => setRestoring(false),
      },
    );
  }

  const isInBackupFolder = dir?.split("/").pop() === BACKUP_FOLDER_NAME;
  const breadcrumbs = useMemo(() => {
    if (!dir || !root || dir === root) return [];
    return dir.slice(root.length).replace(/^\//, "").split("/");
  }, [dir, root]);

  return (
    <div className="flex h-full flex-col gap-3">
      <div>
        <div className="flex items-center gap-2">
          <h1 className="text-2xl font-semibold">Backup-Browser</h1>
          <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("backup-browser")}>
            <HelpCircle className="size-4" />
          </Button>
        </div>
        <p className="text-sm text-muted-foreground">
          Jeder Editor (Item Editor, Mob Drop Editor, Quest Builder, Regen-Datei-Editor,
          Locale-Verwaltung) legt vor jedem Überschreiben/Löschen ein Backup direkt neben der
          Originaldatei an, in einem <code>{BACKUP_FOLDER_NAME}</code>-Ordner - hier durchsuchbar
          und wiederherstellbar, ohne manuell per SFTP nachzuschauen.
        </p>
      </div>

      <div className="flex items-center gap-1 text-sm">
        <Button variant="ghost" size="icon-sm" onClick={() => setDir(root)}>
          <Home className="size-3.5" />
        </Button>
        {breadcrumbs.map((part, i) => (
          <span key={i} className="flex items-center gap-1 text-muted-foreground">
            <ChevronRight className="size-3.5" />
            <button
              className={`hover:text-foreground ${part === BACKUP_FOLDER_NAME ? "font-medium text-foreground" : ""}`}
              onClick={() => setDir(`${root}/${breadcrumbs.slice(0, i + 1).join("/")}`)}
            >
              {part}
            </button>
          </span>
        ))}
        <Button
          variant="ghost"
          size="icon-sm"
          className="ml-auto"
          onClick={() => dir && loadDir(dir)}
        >
          <RefreshCw className={`size-3.5 ${loading ? "animate-spin" : ""}`} />
        </Button>
      </div>

      {restoreOk && (
        <span className="flex items-center gap-1 text-sm text-green-600">
          <CheckCircle2 className="size-4" /> {restoreOk}
        </span>
      )}
      {restoreError && <p className="text-sm text-destructive">{restoreError}</p>}
      {error && <p className="text-sm text-destructive">{error}</p>}

      <div className="flex-1 space-y-1 overflow-y-auto rounded-md border border-border p-1">
        {dir && root && dir !== root && (
          <button
            onClick={goUp}
            className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm hover:bg-muted"
          >
            <Folder className="size-4 text-muted-foreground" />
            ..
          </button>
        )}
        {(entries ?? []).map((entry) => {
          if (entry.is_dir) {
            const isBackupFolder = entry.name === BACKUP_FOLDER_NAME;
            return (
              <button
                key={entry.name}
                onClick={() => openFolder(entry.name)}
                className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm hover:bg-muted"
              >
                {isBackupFolder ? (
                  <FolderArchive className="size-4 text-amber-500" />
                ) : (
                  <Folder className="size-4 text-muted-foreground" />
                )}
                <span className={isBackupFolder ? "font-medium" : ""}>{entry.name}</span>
              </button>
            );
          }

          const timestamp = isInBackupFolder ? parseTimestamp(entry.name) : null;
          return (
            <div
              key={entry.name}
              className="flex items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-muted"
            >
              <FileText className="size-4 shrink-0 text-muted-foreground" />
              <span className="flex-1 truncate">{entry.name}</span>
              {timestamp && (
                <span className="shrink-0 text-xs text-muted-foreground">{timestamp}</span>
              )}
              {isInBackupFolder && timestamp && dir && (
                <>
                  <Button
                    size="sm"
                    variant="outline"
                    className="shrink-0"
                    onClick={() => openDiff(`${dir}/${entry.name}`)}
                  >
                    <Eye className="size-3.5" />
                    Diff anzeigen
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    className="shrink-0"
                    onClick={() => setRestoreConfirm(`${dir}/${entry.name}`)}
                  >
                    <History className="size-3.5" />
                    Wiederherstellen
                  </Button>
                </>
              )}
            </div>
          );
        })}
        {entries && entries.length === 0 && (
          <p className="p-2 text-sm text-muted-foreground">Leerer Ordner.</p>
        )}
      </div>

      {diffPath && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="flex max-h-[80vh] w-[52rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">Diff: {diffPath.split("/").pop()}</p>
              <button
                onClick={() => setDiffPath(null)}
                className="text-muted-foreground hover:text-foreground"
              >
                <X className="size-4" />
              </button>
            </div>
            {diffLoading && <p className="text-sm text-muted-foreground">Lade…</p>}
            {diffError && <p className="text-sm text-destructive">{diffError}</p>}
            {diffData && (
              <>
                <p className="text-xs text-muted-foreground">
                  Zielort: <code>{diffData.target_path}</code>
                  {diffData.current_content === null &&
                    " - existiert dort aktuell nicht (würde neu angelegt)."}
                </p>
                {diffData.is_binary ? (
                  <p className="rounded-md border border-dashed border-border p-3 text-sm text-muted-foreground">
                    Binärdatei - kein Text-Diff möglich. Wiederherstellen kopiert den Inhalt
                    trotzdem unverändert (Byte für Byte).
                  </p>
                ) : (
                  <DiffView
                    oldText={diffData.current_content ?? ""}
                    newText={diffData.backup_content}
                  />
                )}
              </>
            )}
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDiffPath(null)}>
                Schließen
              </Button>
              {diffData && (
                <Button
                  onClick={() => {
                    setRestoreConfirm(diffPath);
                    setDiffPath(null);
                  }}
                >
                  Wiederherstellen…
                </Button>
              )}
            </div>
          </div>
        </div>
      )}

      {restoreConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>
                <code>{restoreConfirm.split("/").pop()}</code> wird an ihren ursprünglichen Ort
                zurückgeschrieben. Die aktuell dort liegende Datei wird dabei selbst zuerst
                gesichert, nicht einfach überschrieben.
              </span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setRestoreConfirm(null)}>
                Abbrechen
              </Button>
              <Button onClick={confirmRestore} disabled={restoring}>
                {restoring ? "Stelle wieder her…" : "Wiederherstellen"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// "old" = aktuell auf dem Server, "new" = Inhalt der Backup-Datei (die
// Richtung, in der ein Restore die Datei verändern würde).
function DiffView({ oldText, newText }: { oldText: string; newText: string }) {
  const parts = useMemo(() => diffLines(oldText, newText), [oldText, newText]);
  const hasChanges = parts.some((p) => p.added || p.removed);

  if (!hasChanges) {
    return (
      <p className="rounded-md border border-dashed border-border p-3 text-sm text-muted-foreground">
        Kein Unterschied - Backup und aktuelle Datei sind inhaltlich identisch.
      </p>
    );
  }

  return (
    <pre className="min-h-0 flex-1 overflow-auto rounded-md border border-border bg-muted/20 p-2 font-mono text-xs">
      {parts.map((part: Change, i) => {
        const lines = part.value.replace(/\n$/, "").split("\n");
        const prefix = part.added ? "+" : part.removed ? "-" : " ";
        const cls = part.added
          ? "bg-green-500/15 text-green-700 dark:text-green-400"
          : part.removed
            ? "bg-red-500/15 text-red-700 dark:text-red-400"
            : "text-muted-foreground";
        return (
          <span key={i}>
            {lines.map((line, j) => (
              <div key={j} className={cls}>
                {prefix} {line}
              </div>
            ))}
          </span>
        );
      })}
    </pre>
  );
}
