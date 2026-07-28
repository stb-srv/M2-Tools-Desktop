import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { FolderOpen } from "lucide-react";
import { Gr2Canvas } from "./Gr2Canvas";
import type { Gr2ModelInfo } from "./types";

export function ModelViewer() {
  const { t } = useTranslation();

  const [dllPath, setDllPath] = useState("");
  const [gr2Path, setGr2Path] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<Gr2ModelInfo | null>(null);

  async function pickDllPath() {
    const selected = await open({
      multiple: false,
      title: "granny2.dll auswählen",
      filters: [{ name: "Granny2 Library", extensions: ["dll"] }],
    });
    if (typeof selected === "string") {
      setDllPath(selected);
    }
  }

  async function pickGr2Path() {
    const selected = await open({
      multiple: false,
      title: ".gr2-Modelldatei auswählen",
      filters: [{ name: "Granny3D Model", extensions: ["gr2"] }],
    });
    if (typeof selected === "string") {
      setGr2Path(selected);
    }
  }

  async function handleLoad() {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<Gr2ModelInfo>("load_gr2_model", {
        grannyDllPath: dllPath,
        gr2Path,
      });
      setInfo(result);
    } catch (e) {
      setError(String(e));
      setInfo(null);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="flex h-full flex-col gap-4">
      <h1 className="text-2xl font-semibold">{t("nav.modelViewer")}</h1>

      <div className="space-y-2 rounded-lg border border-border bg-card p-3">
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={pickDllPath} className="shrink-0">
            <FolderOpen className="size-4" />
            granny2.dll auswählen
          </Button>
          <span className="truncate text-sm text-muted-foreground">
            {dllPath || "Keine Datei ausgewählt"}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={pickGr2Path} className="shrink-0">
            <FolderOpen className="size-4" />
            .gr2-Datei auswählen
          </Button>
          <span className="truncate text-sm text-muted-foreground">
            {gr2Path || "Keine Datei ausgewählt"}
          </span>
        </div>
        <p className="text-xs text-muted-foreground">
          Tipp: <code>granny2.dll</code> liegt normalerweise direkt im
          Hauptordner deines Metin2-Clients (neben der .exe). .gr2-Modelle
          findest du im <code>pack</code>-Ordner des Clients, meist unter{" "}
          <code>ymir work\npc\...</code>, <code>ymir work\monster\...</code>{" "}
          oder <code>ymir work\item\...</code>. Falls diese Ordner nicht
          existieren, liegen die Assets noch gepackt (.epk) vor und müssen
          zuerst entpackt werden.
        </p>
        <Button onClick={handleLoad} disabled={loading || !dllPath || !gr2Path}>
          {loading ? "Lädt…" : "Laden"}
        </Button>
      </div>

      {error && <p className="text-sm text-destructive">{error}</p>}
      {info && (
        <p className="text-sm text-muted-foreground">
          {info.name} — {info.bone_count} Bones, {info.meshes.length} Mesh(es)
          {info.skipped_skinned_meshes > 0 &&
            ` (${info.skipped_skinned_meshes} skinned, noch nicht unterstützt)`}
        </p>
      )}

      <Gr2Canvas
        model={info}
        className="min-h-0 flex-1 overflow-hidden rounded-lg border border-border"
      />
    </div>
  );
}
