import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { HelpCircle } from "lucide-react";
import { openManual } from "@/lib/manual";
import { useUpdateStore } from "@/store/updateStore";
import { GeneralTab } from "./components/GeneralTab";
import { UpdatesTab } from "./components/UpdatesTab";
import { ServerTab } from "./components/ServerTab";
import { NotificationsTab } from "./components/NotificationsTab";
import { ClientTab } from "./components/ClientTab";

type SettingsTab = "general" | "updates" | "server" | "notifications" | "client";
const SETTINGS_TABS: { id: SettingsTab; label: string }[] = [
  { id: "general", label: "Allgemein" },
  { id: "updates", label: "Updates" },
  { id: "server", label: "Server" },
  { id: "notifications", label: "Benachrichtigungen" },
  { id: "client", label: "Client" },
];

export function Settings() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<SettingsTab>("general");
  const [saved, setSaved] = useState<string | null>(null);
  const updateAvailable = useUpdateStore((s) => s.available);

  function onSaved(label: string) {
    setSaved(label);
    setTimeout(() => setSaved(null), 2000);
  }

  return (
    <div className="max-w-2xl space-y-6">
      <div className="flex items-center gap-2">
        <h1 className="text-2xl font-semibold">{t("settings.title")}</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("settings")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>

      <div className="flex flex-wrap gap-1 border-b border-border">
        {SETTINGS_TABS.map((tabDef) => (
          <button
            key={tabDef.id}
            onClick={() => setTab(tabDef.id)}
            className={cn(
              "flex items-center gap-1.5 rounded-t-md px-3 py-2 text-sm font-medium transition-colors",
              tab === tabDef.id
                ? "border-b-2 border-primary text-foreground"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            {tabDef.label}
            {tabDef.id === "updates" && updateAvailable && (
              <span className="size-1.5 shrink-0 rounded-full bg-blue-500" />
            )}
          </button>
        ))}
      </div>

      {/* Alle Tabs bleiben durchgehend gemountet (nur per `hidden` ausgeblendet),
          damit z.B. der Update-Check oder ein Verbindungstest nicht bei jedem
          Tab-Wechsel neu läuft. */}
      <div className={cn(tab !== "general" && "hidden")}>
        <GeneralTab />
      </div>
      <div className={cn(tab !== "updates" && "hidden")}>
        <UpdatesTab />
      </div>
      <div className={cn(tab !== "server" && "hidden")}>
        <ServerTab onSaved={onSaved} />
      </div>
      <div className={cn(tab !== "notifications" && "hidden")}>
        <NotificationsTab onSaved={onSaved} />
      </div>
      <div className={cn(tab !== "client" && "hidden")}>
        <ClientTab onSaved={onSaved} />
      </div>

      {saved && <p className="text-xs text-green-600">{saved}</p>}
    </div>
  );
}
