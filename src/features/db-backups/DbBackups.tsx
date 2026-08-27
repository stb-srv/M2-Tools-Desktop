import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { logActivity } from "@/lib/logActivity";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { toast } from "@/components/ui/toast";
import {
  DatabaseBackup,
  RefreshCw,
  AlertTriangle,
  History,
  Trash2,
  HelpCircle,
} from "lucide-react";
import { openManual } from "@/lib/manual";
import { SCHEDULE_HOURS_OPTIONS, nextDueAt } from "./schedule";

interface RemoteEntry {
  name: string;
  is_dir: boolean;
}

// Filenames are always `m2manager_backup_<YYYYMMDD_HHMMSS>.sql` (see
// db_backup.rs::backup_filename), so the timestamp can be parsed back out
// for display without a separate stat call per file.
function parseTimestamp(filename: string): string | null {
  const match = filename.match(/^m2manager_backup_(\d{8})_(\d{6})\.sql$/);
  if (!match) return null;
  const [, date, time] = match;
  return `${date.slice(0, 4)}-${date.slice(4, 6)}-${date.slice(6, 8)} ${time.slice(0, 2)}:${time.slice(2, 4)}:${time.slice(4, 6)}`;
}

const DEFAULT_DIR = "/usr/home/game/m2manager_db_backups";

export function DbBackups() {
  const [dir, setDir] = useState(DEFAULT_DIR);
  const [entries, setEntries] = useState<RemoteEntry[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const [creating, setCreating] = useState(false);
  const [createConfirm, setCreateConfirm] = useState(false);

  const [restoreTarget, setRestoreTarget] = useState<string | null>(null);
  const [restoring, setRestoring] = useState(false);

  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);

  const [scheduleHours, setScheduleHours] = useState(0);
  const [lastAutoAt, setLastAutoAt] = useState<string | null>(null);

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "db_backup_dir" })
      .then((v) => setDir(v || DEFAULT_DIR))
      .catch(() => {})
      .finally(() => load(undefined));
    invoke<string | null>("get_setting", { key: "db_backup_schedule_hours" })
      .then((v) => setScheduleHours(Number(v ?? 0)))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "db_backup_last_auto_at" })
      .then(setLastAutoAt)
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function changeSchedule(hours: number) {
    setScheduleHours(hours);
    await invoke("set_setting", { key: "db_backup_schedule_hours", value: String(hours) }).catch(() => {});
  }

  async function load(overrideDir: string | undefined) {
    const path = overrideDir ?? dir;
    await runAsyncAction(() => invoke<RemoteEntry[]>("list_backup_dir", { path }), {
      onStart: () => {
        setLoading(true);
        setLoadError(null);
      },
      onSuccess: setEntries,
      onError: setLoadError,
      onFinally: () => setLoading(false),
    });
  }

  async function runCreate() {
    setCreateConfirm(false);
    await runAsyncAction(() => invoke<string>("create_database_backup"), {
      onStart: () => setCreating(true),
      onSuccess: (path) => {
        toast.success(`Backup erstellt: ${path}`);
        logActivity("db-backups", "create", `Datenbank-Backup erstellt: '${path}'`, "backup", path);
        load(undefined);
      },
      onError: (e) => toast.error(e),
      onFinally: () => setCreating(false),
    });
  }

  async function runRestore() {
    if (!restoreTarget) return;
    const path = restoreTarget;
    setRestoreTarget(null);
    await runAsyncAction(() => invoke("restore_database_backup", { backupPath: path }), {
      onStart: () => setRestoring(true),
      onSuccess: () => {
        toast.success(`Wiederhergestellt aus: ${path}`);
        logActivity("db-backups", "restore", `Datenbank-Backup wiederhergestellt: '${path}' (überschreibt Live-DB)`, "backup", path);
      },
      onError: (e) => toast.error(e),
      onFinally: () => setRestoring(false),
    });
  }

  async function runDelete() {
    if (!deleteTarget) return;
    const path = deleteTarget;
    setDeleteTarget(null);
    await runAsyncAction(() => invoke("delete_database_backup", { backupPath: path }), {
      onStart: () => setDeleting(true),
      onSuccess: () => {
        logActivity("db-backups", "delete", `Datenbank-Backup gelöscht: '${path}'`, "backup", path);
        load(undefined);
      },
      onError: (e) => toast.error(e),
      onFinally: () => setDeleting(false),
    });
  }

  const files = (entries ?? []).filter((e) => !e.is_dir);

  return (
    <div className="max-w-3xl space-y-4">
      <div className="flex items-center gap-2">
        <DatabaseBackup className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Datenbank-Backups</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("db-backups")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Löst <code>mysqldump</code> auf dem Server über die bestehende SSH-Verbindung aus, mit
        denselben MySQL-Zugangsdaten wie die eigene DB-Verbindung dieser App (setzt voraus, dass
        diese auch vom Server aus funktionieren - typischerweise der Fall, wenn Datenbank und
        Server auf derselben Maschine laufen). Andere Editoren sichern nur einzelne Dateien - das
        hier ist eine vollständige Momentaufnahme der Datenbanken, konfigurierbar unter
        Einstellungen → Server.
      </p>

      <div className="flex items-center gap-2">
        <Button onClick={() => setCreateConfirm(true)} disabled={creating}>
          <DatabaseBackup className="size-4" />
          {creating ? "Erstelle Backup…" : "Backup jetzt erstellen"}
        </Button>
        <Button variant="outline" onClick={() => load(undefined)} disabled={loading}>
          <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
          Neu laden
        </Button>
      </div>

      <div className="space-y-1.5 rounded-md border border-border p-3">
        <div className="flex items-center gap-2">
          <label htmlFor="db-backup-schedule" className="text-sm font-medium">
            Automatisches Backup
          </label>
          <select
            id="db-backup-schedule"
            value={scheduleHours}
            onChange={(e) => changeSchedule(Number(e.target.value))}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm"
          >
            {SCHEDULE_HOURS_OPTIONS.map((h) => (
              <option key={h} value={h}>
                {h === 0 ? "Aus" : `Alle ${h} Stunden`}
              </option>
            ))}
          </select>
        </div>
        <p className="text-xs text-muted-foreground">
          Läuft nur, solange diese App geöffnet ist (kein echter Cron-Job auf dem Server) - wird
          beim nächsten periodischen Check nachgeholt, sobald die App wieder läuft.
          {scheduleHours > 0 && (
            <>
              {" "}
              Nächstes automatisches Backup fällig: {nextDueAt(lastAutoAt, scheduleHours)?.toLocaleString("de-DE")}.
            </>
          )}
          {lastAutoAt && <> Letztes automatisches Backup: {new Date(lastAutoAt).toLocaleString("de-DE")}.</>}
        </p>
      </div>

      {loadError && <p className="text-sm text-destructive">{loadError}</p>}

      <div className="space-y-1 rounded-md border border-border p-1">
        {files.map((f) => {
          const timestamp = parseTimestamp(f.name);
          return (
            <div
              key={f.name}
              className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-muted"
            >
              <span className="flex-1 truncate font-mono text-xs">{f.name}</span>
              {timestamp && (
                <span className="shrink-0 text-xs text-muted-foreground">{timestamp}</span>
              )}
              <Button
                size="sm"
                variant="outline"
                onClick={() => setRestoreTarget(`${dir}/${f.name}`)}
                disabled={restoring}
              >
                <History className="size-3.5" />
                Wiederherstellen
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setDeleteTarget(`${dir}/${f.name}`)}
                disabled={deleting}
              >
                <Trash2 className="size-3.5" />
              </Button>
            </div>
          );
        })}
        {entries && files.length === 0 && (
          <p className="p-2 text-sm text-muted-foreground">Noch keine Backups.</p>
        )}
        {!entries && !loadError && (
          <div className="space-y-1 p-1">
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-3/4" />
          </div>
        )}
      </div>

      {createConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>
                Erstellt einen vollständigen <code>mysqldump</code> aller konfigurierten
                Datenbanken auf dem Server. Kann je nach Datenmenge etwas dauern.
              </span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setCreateConfirm(false)}>
                Abbrechen
              </Button>
              <Button onClick={runCreate}>Erstellen</Button>
            </div>
          </div>
        </div>
      )}

      {restoreTarget && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>
                <strong>Alle Daten in den gesicherten Datenbanken werden durch den Stand dieses
                Backups ersetzt.</strong> Das betrifft die komplette Live-Datenbank, nicht nur
                einzelne Zeilen, und kann nicht rückgängig gemacht werden außer durch ein neueres
                Backup. Datei: <code>{restoreTarget}</code>
              </span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setRestoreTarget(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={runRestore} disabled={restoring}>
                {restoring ? "Stelle wieder her…" : "Wirklich wiederherstellen"}
              </Button>
            </div>
          </div>
        </div>
      )}

      {deleteTarget && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="flex items-start gap-2 text-sm">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <span>
                <code>{deleteTarget}</code> wird endgültig gelöscht (kein weiteres Backup davon).
              </span>
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDeleteTarget(null)}>
                Abbrechen
              </Button>
              <Button variant="destructive" onClick={runDelete} disabled={deleting}>
                Löschen
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
