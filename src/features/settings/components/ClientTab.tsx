import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { FolderOpen, X } from "lucide-react";
import { saveSetting } from "./shared";

export function ClientTab({ onSaved }: { onSaved: (label: string) => void }) {
  const { t } = useTranslation();

  const [clientPath, setClientPath] = useState("");
  const [binarySrcPath, setBinarySrcPath] = useState("");
  const [npclistPath, setNpclistPath] = useState("");
  const [eterpackPath, setEterpackPath] = useState("");

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "client_path" })
      .then((v) => setClientPath(v ?? ""))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "binary_src_path" })
      .then((v) => setBinarySrcPath(v ?? ""))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "npclist_path" })
      .then((v) => setNpclistPath(v ?? ""))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "eterpack_tool_path" })
      .then((v) => setEterpackPath(v ?? ""))
      .catch(() => {});
  }, []);

  async function save(key: string, value: string, label: string) {
    await saveSetting(key, value);
    onSaved(label);
  }

  async function pickClientPath() {
    const selected = await open({ directory: true, title: "Metin2-Client-Ordner auswählen" });
    if (typeof selected === "string") {
      setClientPath(selected);
      await save("client_path", selected, "Client-Pfad gespeichert");
    }
  }

  async function pickBinarySrcPath() {
    const selected = await open({ directory: true, title: "Lokalen Client-Quellcode-Ordner auswählen" });
    if (typeof selected === "string") {
      setBinarySrcPath(selected);
      await save("binary_src_path", selected, "Client-Quellcode-Pfad gespeichert");
    }
  }

  async function pickNpclist() {
    const selected = await open({
      multiple: false,
      title: "npclist.txt auswählen",
      filters: [{ name: "Textdatei", extensions: ["txt"] }],
    });
    if (typeof selected === "string") {
      setNpclistPath(selected);
      await save("npclist_path", selected, "NPC-Liste gespeichert");
    }
  }

  async function clearNpclist() {
    setNpclistPath("");
    await save("npclist_path", "", "NPC-Liste zurückgesetzt");
  }

  async function pickEterpack() {
    const selected = await open({
      multiple: false,
      title: "EterPackConsoleLz4.exe auswählen",
      filters: [{ name: "Programm", extensions: ["exe"] }],
    });
    if (typeof selected === "string") {
      setEterpackPath(selected);
      await save("eterpack_tool_path", selected, "EterPackConsoleLz4-Pfad gespeichert");
    }
  }

  return (
    <div className="space-y-4">
      <h2 className="text-base font-semibold">Client</h2>

      <section className="space-y-1">
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={pickClientPath} className="shrink-0">
            <FolderOpen className="size-4" />
            {t("settings.clientPath")}
          </Button>
          <span className="truncate text-sm text-muted-foreground">
            {clientPath || "Nicht gesetzt"}
          </span>
        </div>
        <p className="text-xs text-muted-foreground">
          Ordner des Metin2-Clients (enthält <code>granny2.dll</code>) – wird für 3D-Modelle und
          Item-Icons benötigt.
        </p>
      </section>

      <section className="space-y-1">
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={pickBinarySrcPath} className="shrink-0">
            <FolderOpen className="size-4" />
            Client-Quellcode-Ordner
          </Button>
          <span className="truncate text-sm text-muted-foreground">
            {binarySrcPath || "Nicht gesetzt"}
          </span>
        </div>
        <p className="text-xs text-muted-foreground">
          Lokaler Checkout des Client-Quellcodes (z.B. <code>binary-src</code>) – nur für den
          System-Installer nötig, wenn ein System Client-C++-Dateien patcht. Das Kompilieren des
          Clients bleibt danach manuell (kein Client-Build-Werkzeug in M2Manager).
        </p>
      </section>

      <section className="space-y-1">
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={pickNpclist} className="shrink-0">
            <FolderOpen className="size-4" />
            NPC-Liste (npclist.txt)
          </Button>
          <span className="truncate text-sm text-muted-foreground">
            {npclistPath || "Automatisch suchen"}
          </span>
          {npclistPath && (
            <Button variant="ghost" size="icon-sm" onClick={clearNpclist}>
              <X className="size-3.5" />
            </Button>
          )}
        </div>
        <p className="text-xs text-muted-foreground">
          Ordnet NPC-VNUMs den Modell-Ordnern zu. Normalerweise nicht nötig: M2Manager sucht
          zuerst unter <code>root\npclist.txt</code> und durchsucht sonst den kompletten
          Client-Ordner. Nur setzen, falls die Datei umbenannt wurde oder außerhalb des Clients
          liegt.
        </p>
      </section>

      <section className="space-y-1">
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={pickEterpack} className="shrink-0">
            <FolderOpen className="size-4" />
            EterPackConsoleLz4.exe
          </Button>
          <span className="truncate text-sm text-muted-foreground">
            {eterpackPath || "Nicht gesetzt"}
          </span>
        </div>
        <p className="text-xs text-muted-foreground">
          Packt <code>icon.epk</code> neu, nachdem ein Item-Icon hinzugefügt wurde. Liegt
          normalerweise unter <code>Client\pack\EterPackConsoleLz4.exe</code>.
        </p>
      </section>
    </div>
  );
}
