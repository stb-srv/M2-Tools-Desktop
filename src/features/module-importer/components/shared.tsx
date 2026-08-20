import type { StepStatus } from "../shared";
import { CheckCircle2, AlertTriangle } from "lucide-react";

export function StepRow({ label, status }: { label: string; status?: StepStatus }) {
  return (
    <div className="flex items-center gap-2">
      {status === "done" && <CheckCircle2 className="size-4 text-green-600" />}
      {status === "error" && <AlertTriangle className="size-4 text-destructive" />}
      {status === "running" && (
        <span className="size-4 animate-spin rounded-full border-2 border-muted-foreground border-t-transparent" />
      )}
      {(!status || status === "pending") && <span className="size-4 shrink-0" />}
      <span className={status === "error" ? "text-destructive" : ""}>{label}</span>
    </div>
  );
}

export function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
      {label}
      {children}
    </label>
  );
}
