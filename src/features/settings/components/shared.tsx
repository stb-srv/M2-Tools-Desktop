import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { toast } from "@/components/ui/toast";
import { CheckCircle2, FolderOpen, X, XCircle, type LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import { useConnectionProfiles } from "@/store/connectionProfiles";

export type TestState = "idle" | "testing" | "ok" | "error";

export async function saveSetting(key: string, value: string) {
  await invoke("set_setting", { key, value });
}

/** Speichert und meldet Erfolg/Fehler sichtbar statt still zu scheitern. */
export async function saveSettingWithFeedback(key: string, value: string, label: string): Promise<boolean> {
  try {
    await saveSetting(key, value);
    toast.success(`${label} gespeichert`);
    return true;
  } catch (e) {
    toast.error(`${label} konnte nicht gespeichert werden: ${String(e)}`);
    return false;
  }
}

export function ConnStatusIcon({ state }: { state: TestState }) {
  if (state === "ok") return <CheckCircle2 className="size-4 text-green-600" />;
  if (state === "error") return <XCircle className="size-4 text-destructive" />;
  return null;
}

/** Eine thematische Karte innerhalb einer Einstellungs-Seite. */
export function SettingsCard({
  title,
  description,
  icon: Icon,
  children,
  actions,
  className,
}: {
  title: string;
  description?: React.ReactNode;
  icon?: LucideIcon;
  children: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
}) {
  return (
    <section className={cn("overflow-hidden rounded-xl border border-border bg-card shadow-sm", className)}>
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border bg-muted/50 px-5 py-3.5">
        <div className="flex min-w-0 items-center gap-3">
          {Icon && (
            <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
              <Icon className="size-[1.125rem]" />
            </div>
          )}
          <div className="min-w-0">
            <h3 className="text-base font-semibold leading-tight">{title}</h3>
            {description && <p className="mt-0.5 text-sm text-muted-foreground">{description}</p>}
          </div>
        </div>
        {actions}
      </div>
      <div className="divide-y divide-border">{children}</div>
    </section>
  );
}

/**
 * Eine Zeile innerhalb einer SettingsCard. Ab mittlerer Breite steht
 * Beschriftung + Erklärung links und die Eingabe rechts daneben, schmal
 * untereinander.
 */
export function SettingRow({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="grid gap-x-8 gap-y-2 px-5 py-4 md:grid-cols-[minmax(12rem,20rem)_1fr] md:items-start">
      <div className="space-y-1">
        <div className="text-sm font-medium">{label}</div>
        {hint && <p className="text-sm leading-snug text-muted-foreground">{hint}</p>}
      </div>
      <div className="min-w-0 space-y-2">{children}</div>
    </div>
  );
}

export interface TextSettingDef {
  key: string;
  label: string;
  hint?: React.ReactNode;
  defaultValue: string;
  placeholder?: string;
  /** Leere Eingabe erlaubt (z.B. "alle Prozesse"). Sonst wird leer ignoriert. */
  allowEmpty?: boolean;
  validate?: (value: string) => string | null;
  className?: string;
}

/** Textfeld, das beim Verlassen speichert (nur wenn sich etwas geändert hat). */
export function TextSetting({ def }: { def: TextSettingDef }) {
  const [value, setValue] = useState(def.defaultValue);
  const [stored, setStored] = useState(def.defaultValue);
  const [error, setError] = useState<string | null>(null);

  // Server-Pfade gehören zum aktiven Profil -> nach einem Wechsel neu laden.
  const profileRevision = useConnectionProfiles((s) => s.revision);

  useEffect(() => {
    invoke<string | null>("get_setting", { key: def.key })
      .then((v) => {
        const initial = v ?? def.defaultValue;
        setValue(initial);
        setStored(initial);
        setError(null);
      })
      .catch(() => {});
  }, [def.key, def.defaultValue, profileRevision]);

  async function commit() {
    const trimmed = value.trim();
    if (trimmed === stored) return;
    if (!trimmed && !def.allowEmpty) {
      setValue(stored);
      return;
    }
    const problem = def.validate?.(trimmed) ?? null;
    setError(problem);
    if (problem) return;
    if (await saveSettingWithFeedback(def.key, trimmed, def.label)) setStored(trimmed);
  }

  return (
    <SettingRow label={def.label} hint={def.hint}>
      <input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
        placeholder={def.placeholder}
        className={cn(
          "w-full max-w-2xl rounded-md border bg-background px-3 py-2 font-mono text-sm",
          error ? "border-destructive" : "border-border",
          def.className,
        )}
      />
      {error && <p className="text-xs text-destructive">{error}</p>}
    </SettingRow>
  );
}

export interface PathSettingDef {
  key: string;
  label: string;
  hint?: React.ReactNode;
  directory?: boolean;
  dialogTitle: string;
  filters?: { name: string; extensions: string[] }[];
  emptyText?: string;
  clearable?: boolean;
}

/** Lokaler Datei-/Ordnerpfad mit Auswahldialog. */
export function PathSetting({ def }: { def: PathSettingDef }) {
  const [value, setValue] = useState("");

  useEffect(() => {
    invoke<string | null>("get_setting", { key: def.key })
      .then((v) => setValue(v ?? ""))
      .catch(() => {});
  }, [def.key]);

  async function pick() {
    const selected = await open({
      directory: def.directory,
      multiple: false,
      title: def.dialogTitle,
      filters: def.filters,
    });
    if (typeof selected === "string" && (await saveSettingWithFeedback(def.key, selected, def.label))) {
      setValue(selected);
    }
  }

  async function clear() {
    if (await saveSettingWithFeedback(def.key, "", def.label)) setValue("");
  }

  return (
    <SettingRow label={def.label} hint={def.hint}>
      <div className="flex max-w-2xl items-center gap-2">
        <div
          className={cn(
            "min-w-0 flex-1 truncate rounded-md border border-border bg-muted/40 px-3 py-2 font-mono text-sm",
            !value && "font-sans text-muted-foreground",
          )}
          title={value}
        >
          {value || def.emptyText || "Nicht gesetzt"}
        </div>
        {def.clearable && value && (
          <Button variant="ghost" size="icon-sm" onClick={clear} title="Zurücksetzen">
            <X className="size-3.5" />
          </Button>
        )}
        <Button variant="outline" size="sm" onClick={pick} className="shrink-0">
          <FolderOpen className="size-4" />
          Auswählen
        </Button>
      </div>
    </SettingRow>
  );
}
