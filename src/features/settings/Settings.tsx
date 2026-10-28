import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  Bell,
  ChevronRight,
  Download,
  FolderTree,
  HelpCircle,
  Monitor,
  Plug,
  SlidersHorizontal,
  type LucideIcon,
} from "lucide-react";
import { openManual } from "@/lib/manual";
import { useUpdateStore } from "@/store/updateStore";
import { useSettingsSection, type SettingsSection } from "@/store/settingsSection";
import { GeneralTab } from "./components/GeneralTab";
import { UpdatesTab } from "./components/UpdatesTab";
import { ConnectionTab } from "./components/ConnectionTab";
import { ServerPathsTab } from "./components/ServerPathsTab";
import { NotificationsTab } from "./components/NotificationsTab";
import { ClientTab } from "./components/ClientTab";

const SECTIONS: {
  id: SettingsSection;
  label: string;
  description: string;
  icon: LucideIcon;
  Component: () => React.ReactNode;
}[] = [
  {
    id: "general",
    label: "Allgemein",
    description: "Design, Sprache, Größe der Oberfläche und Voreinstellungen für die Editoren.",
    icon: SlidersHorizontal,
    Component: GeneralTab,
  },
  {
    id: "connection",
    label: "Verbindung",
    description: "Server-Profile mit SSH- und MySQL-Zugang. Den aktiven Server wechselst du auch auf der Startseite.",
    icon: Plug,
    Component: ConnectionTab,
  },
  {
    id: "server-paths",
    label: "Server-Pfade",
    description: "Wo die Dateien auf dem Gameserver liegen. Gespeichert wird beim Verlassen eines Feldes oder mit Enter.",
    icon: FolderTree,
    Component: ServerPathsTab,
  },
  {
    id: "client",
    label: "Client & Werkzeuge",
    description: "Lokale Ordner und Programme auf diesem PC.",
    icon: Monitor,
    Component: ClientTab,
  },
  {
    id: "notifications",
    label: "Benachrichtigungen",
    description: "Meldungen an Discord o.ä., wenn etwas schiefgeht.",
    icon: Bell,
    Component: NotificationsTab,
  },
  {
    id: "updates",
    label: "Updates",
    description: "M2Manager aktualisiert sich selbst über GitHub-Releases.",
    icon: Download,
    Component: UpdatesTab,
  },
];

export function Settings() {
  const { t } = useTranslation();
  const { section, setSection } = useSettingsSection();
  const updateAvailable = useUpdateStore((s) => s.available);
  const current = SECTIONS.find((s) => s.id === section) ?? SECTIONS[0];

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-2">
        <h1 className="text-2xl font-semibold">{t("settings.title")}</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("settings")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>

      <div className="grid gap-6 lg:grid-cols-[16rem_minmax(0,1fr)] xl:gap-8">
        {/* Schmal: Bereiche als waagerechte Leiste; ab lg links als feste Spalte. */}
        <nav className="-mx-1 flex gap-1 overflow-x-auto px-1 pb-1 lg:sticky lg:top-0 lg:mx-0 lg:flex-col lg:self-start lg:overflow-visible lg:rounded-xl lg:border lg:border-border lg:bg-card lg:p-2 lg:shadow-sm">
          {SECTIONS.map(({ id, label, icon: Icon }) => {
            const active = section === id;
            return (
              <button
                key={id}
                onClick={() => setSection(id)}
                aria-current={active ? "page" : undefined}
                className={cn(
                  "flex shrink-0 items-center gap-2.5 rounded-lg px-3 py-2.5 text-left text-sm transition-colors lg:w-full",
                  active
                    ? "bg-primary font-semibold text-primary-foreground shadow-sm"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                <Icon className="size-[1.125rem] shrink-0" />
                <span className="flex-1 whitespace-nowrap">{label}</span>
                {id === "updates" && updateAvailable && (
                  <span
                    className={cn("size-2 shrink-0 rounded-full", active ? "bg-primary-foreground" : "bg-blue-500")}
                  />
                )}
                {active && <ChevronRight className="hidden size-4 shrink-0 lg:block" />}
              </button>
            );
          })}
        </nav>

        <div className="min-w-0 space-y-5">
          <header className="flex items-start gap-4 rounded-xl border border-border bg-card px-5 py-4 shadow-sm">
            <div className="flex size-11 shrink-0 items-center justify-center rounded-xl bg-primary text-primary-foreground">
              <current.icon className="size-5" />
            </div>
            <div className="min-w-0">
              <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Einstellungen
              </div>
              <h2 className="text-xl font-semibold leading-tight">{current.label}</h2>
              <p className="mt-1 text-sm text-muted-foreground">{current.description}</p>
            </div>
          </header>

          {/* Alle Bereiche bleiben gemountet (nur per `hidden` ausgeblendet),
              damit z.B. der Update-Check oder ein Verbindungstest nicht bei
              jedem Wechsel neu läuft. */}
          {SECTIONS.map(({ id, Component }) => (
            <div key={id} className={cn(section !== id && "hidden")}>
              <Component />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
