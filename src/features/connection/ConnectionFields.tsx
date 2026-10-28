import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { PasswordInput } from "@/components/ui/password-input";
import { CheckCircle2, FolderOpen, XCircle } from "lucide-react";
import {
  mysqlComplete,
  sshComplete,
  testMysql,
  testSsh,
  type MysqlValues,
  type SshValues,
} from "@/lib/connection";
import { cn } from "@/lib/utils";

// Gemeinsame Eingabefelder für SSH/MySQL - genutzt vom "Neues Profil"-Dialog,
// den Einstellungen (Verbindung) und dem Setup-Assistenten.

export function TextField({
  label,
  value,
  onChange,
  placeholder,
  className,
  autoFocus,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
  autoFocus?: boolean;
}) {
  return (
    <label className={cn("block space-y-1", className)}>
      <span className="text-xs font-medium text-muted-foreground">{label}</span>
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        autoFocus={autoFocus}
        className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
      />
    </label>
  );
}

export function SshFields({
  value,
  onChange,
}: {
  value: SshValues;
  onChange: (value: SshValues) => void;
}) {
  const set = <K extends keyof SshValues>(key: K, v: SshValues[K]) => onChange({ ...value, [key]: v });

  async function pickKey() {
    const selected = await open({ multiple: false, title: "Privaten SSH-Schlüssel auswählen" });
    if (typeof selected === "string") set("keyPath", selected);
  }

  return (
    <div className="space-y-2">
      <div className="grid grid-cols-3 gap-2">
        <TextField
          label="Host / IP"
          value={value.host}
          onChange={(v) => set("host", v)}
          placeholder="z.B. 192.168.0.10"
          className="col-span-2"
        />
        <TextField label="Port" value={value.port} onChange={(v) => set("port", v)} placeholder="22" />
      </div>
      <TextField label="Benutzername" value={value.username} onChange={(v) => set("username", v)} />

      <div className="space-y-1">
        <span className="text-xs font-medium text-muted-foreground">Anmeldung</span>
        <div className="flex overflow-hidden rounded-md border border-border text-sm">
          {(["password", "key"] as const).map((mode) => (
            <button
              key={mode}
              type="button"
              onClick={() => set("authMode", mode)}
              className={cn(
                "flex-1 px-3 py-1",
                value.authMode === mode && "bg-primary text-primary-foreground",
              )}
            >
              {mode === "password" ? "Passwort" : "SSH-Schlüssel"}
            </button>
          ))}
        </div>
      </div>

      {value.authMode === "password" ? (
        <PasswordInput value={value.password} onChange={(v) => set("password", v)} placeholder="Passwort" />
      ) : (
        <>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={pickKey} className="shrink-0">
              <FolderOpen className="size-4" />
              Schlüsseldatei
            </Button>
            <span className="truncate text-sm text-muted-foreground">
              {value.keyPath || "Keine Datei ausgewählt"}
            </span>
          </div>
          <PasswordInput
            value={value.keyPassphrase}
            onChange={(v) => set("keyPassphrase", v)}
            placeholder="Passphrase (falls vorhanden)"
          />
        </>
      )}
    </div>
  );
}

export function MysqlFields({
  value,
  onChange,
}: {
  value: MysqlValues;
  onChange: (value: MysqlValues) => void;
}) {
  const set = <K extends keyof MysqlValues>(key: K, v: MysqlValues[K]) => onChange({ ...value, [key]: v });

  return (
    <div className="space-y-2">
      <div className="grid grid-cols-3 gap-2">
        <TextField
          label="Host / IP"
          value={value.host}
          onChange={(v) => set("host", v)}
          placeholder="meist wie SSH-Host"
          className="col-span-2"
        />
        <TextField label="Port" value={value.port} onChange={(v) => set("port", v)} placeholder="3306" />
      </div>
      <TextField label="Benutzername" value={value.username} onChange={(v) => set("username", v)} />
      <PasswordInput value={value.password} onChange={(v) => set("password", v)} placeholder="Passwort" />
    </div>
  );
}

export type TestState = "idle" | "testing" | "ok" | "error";

/** Testet die eingegebenen Werte, ohne etwas zu speichern. */
export function useConnectionTest(run: () => Promise<void>, values: unknown) {
  const [state, setState] = useState<TestState>("idle");
  const [error, setError] = useState<string | null>(null);

  // Ein altes "klappt" darf nach einer Änderung an den Feldern nicht stehen bleiben.
  const fingerprint = JSON.stringify(values);
  useEffect(() => {
    setState("idle");
    setError(null);
  }, [fingerprint]);

  async function test() {
    setState("testing");
    setError(null);
    try {
      await run();
      setState("ok");
    } catch (e) {
      setState("error");
      setError(String(e));
    }
  }

  return { state, error, test };
}

export function TestResult({ state, error }: { state: TestState; error: string | null }) {
  if (state === "ok") {
    return (
      <span className="flex items-center gap-1 text-sm text-green-600">
        <CheckCircle2 className="size-4" />
        Verbindung klappt
      </span>
    );
  }
  if (state === "error") {
    return (
      <span className="flex min-w-0 items-start gap-1 text-sm text-destructive">
        <XCircle className="mt-0.5 size-4 shrink-0" />
        <span className="line-clamp-3">{error}</span>
      </span>
    );
  }
  return null;
}

export function SshTestButton({ value }: { value: SshValues }) {
  const t = useConnectionTest(() => testSsh(value), value);
  return (
    <div className="flex items-start gap-3">
      <Button
        variant="outline"
        size="sm"
        onClick={t.test}
        disabled={!sshComplete(value) || t.state === "testing"}
        className="shrink-0"
      >
        {t.state === "testing" ? "Teste…" : "Verbindung testen"}
      </Button>
      <TestResult state={t.state} error={t.error} />
    </div>
  );
}

export function MysqlTestButton({ value }: { value: MysqlValues }) {
  const t = useConnectionTest(() => testMysql(value), value);
  return (
    <div className="flex items-start gap-3">
      <Button
        variant="outline"
        size="sm"
        onClick={t.test}
        disabled={!mysqlComplete(value) || t.state === "testing"}
        className="shrink-0"
      >
        {t.state === "testing" ? "Teste…" : "Verbindung testen"}
      </Button>
      <TestResult state={t.state} error={t.error} />
    </div>
  );
}
