import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Bell, Download, FolderTree, HelpCircle, Monitor, Plug, SlidersHorizontal } from "lucide-react";
import { openManual } from "@/lib/manual";
import { useUpdateStore } from "@/store/updateStore";
import { useSettingsSection, type SettingsSection } from "@/store/settingsSection";
import { GeneralTab } from "./components/GeneralTab";
import { UpdatesTab } from "./components/UpdatesTab";
import { ConnectionTab } from "./components/ConnectionTab";
import { ServerPathsTab } from "./components/ServerPathsTab";
import { NotificationsTab } from "./components/NotificationsTab";
import { ClientTab } from "./components/ClientTab";

const SECTIONS: { id: SettingsSection; label: string; icon: typeof Plug }[] = [
  { id: "general", label: "Allgemein", icon: SlidersHorizontal },
  { id: "connection", label: "Verbindung", icon: Plug },
  { id: "server-paths", label: "Server-Pfade", icon: FolderTree },
  { id: "client", label: "Client & Werkzeuge", icon: Monitor },
  { id: "notifications", label: "Benachrichtigungen", icon: Bell },
  { id: "updates", label: "Updates", icon: Download },
];

export function Settings() {
  const { t } = useTranslation();
  const { section, setSection } = useSettingsSection();
  const updateAvailable = useUpdateStore((s) => s.available);

  return (
    <div className="max-w-4xl space-y-6">
      <div className="flex items-center gap-2">
        <h1 className="text-2xl font-semibold">{t("settings.title")}</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("settings")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>

      <div className="flex gap-6">
        <nav className="w-48 shrink-0 space-y-0.5">
          {SECTIONS.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              onClick={() => setSection(id)}
              className={cn(
                "flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors",
                section === id
                  ? "bg-accent font-medium text-accent-foreground"
                  : "text-muted-foreground hover:bg-accent/50 hover:text-foreground",
              )}
            >
              <Icon className="size-4 shrink-0" />
              <span className="flex-1">{label}</span>
              {id === "updates" && updateAvailable && <span className="size-1.5 shrink-0 rounded-full bg-blue-500" />}
            </button>
          ))}
        </nav>

        {/* Alle Bereiche bleiben gemountet (nur per `hidden` ausgeblendet),
            damit z.B. der Update-Check oder ein Verbindungstest nicht bei
            jedem Wechsel neu läuft. */}
        <div className="min-w-0 flex-1">
          <div className={cn(section !== "general" && "hidden")}>
            <GeneralTab />
          </div>
          <div className={cn(section !== "connection" && "hidden")}>
            <ConnectionTab />
          </div>
          <div className={cn(section !== "server-paths" && "hidden")}>
            <ServerPathsTab />
          </div>
          <div className={cn(section !== "client" && "hidden")}>
            <ClientTab />
          </div>
          <div className={cn(section !== "notifications" && "hidden")}>
            <NotificationsTab />
          </div>
          <div className={cn(section !== "updates" && "hidden")}>
            <UpdatesTab />
          </div>
        </div>
      </div>
    </div>
  );
}
