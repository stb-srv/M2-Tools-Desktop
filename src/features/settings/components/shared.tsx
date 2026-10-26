import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { toast } from "@/components/ui/toast";
import { CheckCircle2, FolderOpen, X, XCircle } from "lucide-react";
import { cn } from "@/lib/utils";

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

/** Überschrift einer Einstellungs-Seite. */
export function SectionHeader({ title, description }: { title: string; description?: string }) {
  return (
    <div className="space-y-1">
      <h2 className="text-lg font-semibold">{title}</h2>
      {description && <p className="text-sm text-muted-foreground">{description}</p>}
    </div>
  );
}

/** Eine thematische Karte innerhalb einer Einstellungs-Seite. */
export function SettingsCard({
  title,
  description,
  children,
  actions,
}: {
  title: string;
  description?: React.ReactNode;
  children: React.ReactNode;
  actions?: React.ReactNode;
}) {
  return (
    <section className="rounded-lg border border-border bg-card">
      <div className="flex items-start justify-between gap-3 border-b border-border px-4 py-3">
        <div className="space-y-0.5">
          <h3 className="text-sm font-semibold">{title}</h3>
          {description && <p className="text-xs text-muted-foreground">{description}</p>}
        </div>
        {actions}
      </div>
      <div className="divide-y divide-border">{children}</div>
    </section>
  );
}

/** Eine Zeile innerhalb einer SettingsCard: Beschriftung + Hilfe links, Eingabe darunter. */
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
    <div className="space-y-1.5 px-4 py-3">
      <div className="text-sm font-medium">{label}</div>
      {children}
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
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

  useEffect(() => {
    invoke<string | null>("get_setting", { key: def.key })
      .then((v) => {
        const initial = v ?? def.defaultValue;
        setValue(initial);
        setStored(initial);
      })
      .catch(() => {});
  }, [def.key, def.defaultValue]);

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
          "w-full rounded-md border bg-background px-2 py-1.5 font-mono text-sm",
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
      <div className="flex items-center gap-2">
        <div
          className={cn(
            "min-w-0 flex-1 truncate rounded-md border border-border bg-muted/40 px-2 py-1.5 font-mono text-sm",
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
