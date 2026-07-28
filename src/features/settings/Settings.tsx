import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useThemeStore, type Theme } from "@/store/theme";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { FolderOpen, X } from "lucide-react";

const THEMES: Theme[] = ["light", "dark", "system"];
const LANGUAGES = [
  { code: "de", label: "Deutsch" },
  { code: "en", label: "English" },
];

export function Settings() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useThemeStore();

  const [clientPath, setClientPath] = useState("");
  const [npclistPath, setNpclistPath] = useState("");
  const [saved, setSaved] = useState<string | null>(null);

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "client_path" })
      .then((v) => setClientPath(v ?? ""))
      .catch(() => {});
    invoke<string | null>("get_setting", { key: "npclist_path" })
      .then((v) => setNpclistPath(v ?? ""))
      .catch(() => {});
  }, []);

  async function save(key: string, value: string, label: string) {
    await invoke("set_setting", { key, value });
    setSaved(label);
    setTimeout(() => setSaved(null), 2000);
  }

  async function pickClientPath() {
    const selected = await open({ directory: true, title: "Metin2-Client-Ordner auswählen" });
    if (typeof selected === "string") {
      setClientPath(selected);
      await save("client_path", selected, "Client-Pfad gespeichert");
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

  return (
    <div className="max-w-2xl space-y-8">
      <h1 className="text-2xl font-semibold">{t("settings.title")}</h1>

      <section className="space-y-2">
        <h2 className="text-sm font-medium text-muted-foreground">
          {t("settings.theme")}
        </h2>
        <div className="flex gap-2">
          {THEMES.map((value) => (
            <Button
              key={value}
              variant={theme === value ? "default" : "outline"}
              onClick={() => setTheme(value)}
            >
              {t(`settings.${value}`)}
            </Button>
          ))}
        </div>
      </section>

      <section className="space-y-2">
        <h2 className="text-sm font-medium text-muted-foreground">
          {t("settings.language")}
        </h2>
        <div className="flex gap-2">
          {LANGUAGES.map((lang) => (
            <Button
              key={lang.code}
              variant={i18n.language === lang.code ? "default" : "outline"}
              onClick={() => i18n.changeLanguage(lang.code)}
              className={cn(i18n.language === lang.code && "font-semibold")}
            >
              {lang.label}
            </Button>
          ))}
        </div>
      </section>

      <section className="space-y-3">
        <h2 className="text-sm font-medium text-muted-foreground">
          {t("settings.paths")}
        </h2>

        <div className="space-y-1">
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
            Ordner des Metin2-Clients (enthält <code>granny2.dll</code>) – wird
            für 3D-Modelle und Item-Icons benötigt.
          </p>
        </div>

        <div className="space-y-1">
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
            Ordnet NPC-VNUMs den Modell-Ordnern zu. Normalerweise nicht nötig:
            M2Manager sucht zuerst unter <code>root\npclist.txt</code> und
            durchsucht sonst den kompletten Client-Ordner. Nur setzen, falls die
            Datei umbenannt wurde oder außerhalb des Clients liegt.
          </p>
        </div>

        {saved && <p className="text-xs text-green-600">{saved}</p>}
      </section>
    </div>
  );
}
