import { fileNameOf } from "../shared";

export function IconPicker({
  pool,
  value,
  onChange,
}: {
  pool: string[];
  value: string | null;
  onChange: (v: string | null) => void;
}) {
  return (
    <select
      value={value ?? ""}
      onChange={(e) => onChange(e.target.value || null)}
      className="max-w-[12rem] rounded-md border border-border bg-background px-2 py-1 text-xs"
    >
      <option value="">— kein Icon —</option>
      {pool.map((p) => (
        <option key={p} value={p}>
          {fileNameOf(p)}
        </option>
      ))}
    </select>
  );
}
