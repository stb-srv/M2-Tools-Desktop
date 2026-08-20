import { invoke } from "@tauri-apps/api/core";
import { CheckCircle2, XCircle } from "lucide-react";

export type TestState = "idle" | "testing" | "ok" | "error";

export async function saveSetting(key: string, value: string) {
  await invoke("set_setting", { key, value });
}

export function ConnField({
  label,
  value,
  onChange,
  className,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  className?: string;
}) {
  return (
    <div className={`space-y-1 ${className ?? ""}`}>
      <label className="text-xs font-medium text-muted-foreground">{label}</label>
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
      />
    </div>
  );
}

export function ConnStatusIcon({ state }: { state: TestState }) {
  if (state === "ok") return <CheckCircle2 className="size-4 text-green-600" />;
  if (state === "error") return <XCircle className="size-4 text-destructive" />;
  return null;
}
