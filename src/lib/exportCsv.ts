import { save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

/**
 * Semicolon-delimited, not comma - German-locale Excel (the realistic
 * consumer here) splits on `;` by default, so a comma-CSV opens as one
 * unsplit column unless the user manually re-imports it.
 */
function toCsvField(value: string | number): string {
  const s = String(value);
  return /[;"\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

/**
 * Builds a CSV string (with a UTF-8 BOM so Excel renders Umlaute correctly)
 * and writes it to a path the user picks via a native save dialog. Returns
 * false if the user cancelled the dialog.
 */
export async function exportRowsAsCsv(
  defaultFileName: string,
  headers: string[],
  rows: (string | number)[][],
): Promise<boolean> {
  const path = await save({
    defaultPath: defaultFileName,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  if (!path) return false;

  const lines = [headers, ...rows].map((row) => row.map(toCsvField).join(";"));
  const csv = "﻿" + lines.join("\r\n") + "\r\n";
  await invoke("export_text_file", { path, content: csv });
  return true;
}
