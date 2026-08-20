import { Search, X } from "lucide-react";
import { Button } from "@/components/ui/button";

export function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-xs text-muted-foreground">
      {label}
      {children}
    </label>
  );
}

export function VnumInput({
  value,
  onChange,
  onPick,
}: {
  value: string;
  onChange: (value: string) => void;
  onPick: () => void;
}) {
  return (
    <div className="flex gap-1">
      <input
        type="number"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
      />
      <Button variant="outline" size="icon-sm" onClick={onPick} title="Suchen…">
        <Search className="size-3.5" />
      </Button>
    </div>
  );
}

export function Modal({
  children,
  onClose,
  wide,
}: {
  children: React.ReactNode;
  onClose: () => void;
  wide?: boolean;
}) {
  return (
    <div className="fixed inset-0 flex items-center justify-center bg-black/50">
      <div className={`rounded-lg border border-border bg-card p-4 ${wide ? "w-[48rem]" : "w-96"}`}>
        <div className="mb-2 flex justify-end">
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground">
            <X className="size-4" />
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
