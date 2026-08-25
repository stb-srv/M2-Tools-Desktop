import "@/i18n";
import { lazy, Suspense, useEffect, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { autoConnectMysql } from "@/lib/mysqlConnect";
import { Sidebar } from "@/components/layout/Sidebar";
import { CommandPalette } from "@/components/CommandPalette";
import { GlobalSearch } from "@/components/GlobalSearch";
import { CrashWatch } from "@/components/CrashWatch";
import { useNavigationStore, type Section } from "@/store/navigation";
import { useUpdateStore } from "@/store/updateStore";
import { Dashboard } from "@/features/dashboard/Dashboard";
import { SetupWizard } from "@/features/setup/SetupWizard";
import { cn } from "@/lib/utils";

// Lazy-loaded: each of these pulls in its own chunk (three.js for the model
// viewer/shop preview, CodeMirror for the quest builder, ...) - eagerly
// importing all 18 into App.tsx's own bundle is what produced the single
// 1.5MB chunk Vite warned about. Only Dashboard (the default section, see
// navigation.ts) stays eager for a fast first paint; everything else loads
// on first visit to that section.
const ServerControl = lazy(() => import("@/features/server-control/ServerControl").then((m) => ({ default: m.ServerControl })));
const BuildDeploy = lazy(() => import("@/features/build-deploy/BuildDeploy").then((m) => ({ default: m.BuildDeploy })));
const ServerEvents = lazy(() => import("@/features/server-events/ServerEvents").then((m) => ({ default: m.ServerEvents })));
const DbExplorer = lazy(() => import("@/features/db-explorer/DbExplorer").then((m) => ({ default: m.DbExplorer })));
const ShopEditor = lazy(() => import("@/features/shop-editor/ShopEditor").then((m) => ({ default: m.ShopEditor })));
const ItemEditor = lazy(() => import("@/features/item-editor/ItemEditor").then((m) => ({ default: m.ItemEditor })));
const ItemProtoExplorer = lazy(() => import("@/features/item-proto-explorer/ItemProtoExplorer").then((m) => ({ default: m.ItemProtoExplorer })));
const ItemViewer = lazy(() => import("@/features/item-viewer/ItemViewer").then((m) => ({ default: m.ItemViewer })));
const ModuleImporter = lazy(() => import("@/features/module-importer/ModuleImporter").then((m) => ({ default: m.ModuleImporter })));
const MobProtoEditor = lazy(() => import("@/features/mob-proto-editor/MobProtoEditor").then((m) => ({ default: m.MobProtoEditor })));
const MobDropEditor = lazy(() => import("@/features/mob-drop-editor/MobDropEditor").then((m) => ({ default: m.MobDropEditor })));
const DropGenerator = lazy(() => import("@/features/drop-generator/DropGenerator").then((m) => ({ default: m.DropGenerator })));
const RefineEditor = lazy(() => import("@/features/refine-editor/RefineEditor").then((m) => ({ default: m.RefineEditor })));
const BoxEditor = lazy(() => import("@/features/box-editor/BoxEditor").then((m) => ({ default: m.BoxEditor })));
const CubeEditor = lazy(() => import("@/features/cube-editor/CubeEditor").then((m) => ({ default: m.CubeEditor })));
const QuestBuilder = lazy(() => import("@/features/quest-builder/QuestBuilder").then((m) => ({ default: m.QuestBuilder })));
const RegenEditor = lazy(() => import("@/features/regen-editor/RegenEditor").then((m) => ({ default: m.RegenEditor })));
const LocaleEditor = lazy(() => import("@/features/locale-editor/LocaleEditor").then((m) => ({ default: m.LocaleEditor })));
const BackupBrowser = lazy(() => import("@/features/backup-browser/BackupBrowser").then((m) => ({ default: m.BackupBrowser })));
const DbBackups = lazy(() => import("@/features/db-backups/DbBackups").then((m) => ({ default: m.DbBackups })));
const ActivityLog = lazy(() => import("@/features/activity-log/ActivityLog").then((m) => ({ default: m.ActivityLog })));
const TgaConverter = lazy(() => import("@/features/tga-converter/TgaConverter").then((m) => ({ default: m.TgaConverter })));
const IconBrowser = lazy(() => import("@/features/icon-browser/IconBrowser").then((m) => ({ default: m.IconBrowser })));
const ModelViewer = lazy(() => import("@/features/model-viewer/ModelViewer").then((m) => ({ default: m.ModelViewer })));
const AccountManager = lazy(() => import("@/features/account-manager/AccountManager").then((m) => ({ default: m.AccountManager })));
const GuildManager = lazy(() => import("@/features/guild-manager/GuildManager").then((m) => ({ default: m.GuildManager })));
const EconomyDashboard = lazy(() => import("@/features/economy-dashboard/EconomyDashboard").then((m) => ({ default: m.EconomyDashboard })));
const GmManager = lazy(() => import("@/features/gm-manager/GmManager").then((m) => ({ default: m.GmManager })));
const SystemInstaller = lazy(() => import("@/features/system-installer/SystemInstaller").then((m) => ({ default: m.SystemInstaller })));
const BroadcastSystem = lazy(() => import("@/features/broadcast/BroadcastSystem").then((m) => ({ default: m.BroadcastSystem })));
const WeatherControl = lazy(() => import("@/features/weather/WeatherControl").then((m) => ({ default: m.WeatherControl })));
const Settings = lazy(() => import("@/features/settings/Settings").then((m) => ({ default: m.Settings })));

function LoadingFallback() {
  return (
    <div className="flex h-full items-center justify-center">
      <span className="size-6 animate-spin rounded-full border-2 border-muted-foreground border-t-transparent" />
    </div>
  );
}

// Once a section has been visited it stays mounted (just hidden via CSS)
// instead of unmounting - previously switching sections destroyed every
// editor's local React state, so any unsaved form was gone the moment you
// left the page. Each slot gets its OWN Suspense boundary (not one shared
// one across all 27 sections) - with a single shared boundary, opening a
// not-yet-loaded lazy chunk would re-suspend the whole boundary and flash
// every already-rendered, merely-hidden section back to the loading spinner.
function SectionSlot({
  section,
  active,
  visited,
  children,
}: {
  section: Section;
  active: Section;
  visited: Set<Section>;
  children: ReactNode;
}) {
  if (!visited.has(section)) return null;
  return (
    <div className={cn("h-full", section !== active && "hidden")}>
      <Suspense fallback={<LoadingFallback />}>{children}</Suspense>
    </div>
  );
}

function App() {
  const section = useNavigationStore((state) => state.section);
  const [visited, setVisited] = useState<Set<Section>>(() => new Set(["dashboard"]));
  useEffect(() => {
    setVisited((prev) => (prev.has(section) ? prev : new Set(prev).add(section)));
  }, [section]);
  const [setupChecked, setSetupChecked] = useState(false);
  const [setupCompleted, setSetupCompleted] = useState(true);
  const [mysqlChecked, setMysqlChecked] = useState(false);

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "setup_completed" })
      .then((value) => setSetupCompleted(value === "true"))
      .finally(() => setSetupChecked(true));
  }, []);

  useEffect(() => {
    if (!setupChecked || !setupCompleted) return;
    autoConnectMysql().finally(() => setMysqlChecked(true));
  }, [setupChecked, setupCompleted]);

  // Leiser Hintergrund-Check beim Start - zeigt bei Erfolg nur den kleinen
  // Punkt am Einstellungen-Eintrag (siehe Sidebar.tsx/updateStore.ts), keine
  // Zwangsinstallation. Schlägt lautlos fehl (privates Repo ohne Token in
  // lokalen Dev-Builds, kein Internet, o.ä.) - siehe build_updater_plugin()
  // in lib.rs für die Token-Einbettung.
  useEffect(() => {
    if (!setupChecked || !setupCompleted) return;
    check()
      .then((update) => {
        if (update) {
          useUpdateStore.getState().setAvailable({ version: update.version, notes: update.body ?? null });
        }
      })
      .catch(() => {});
  }, [setupChecked, setupCompleted]);

  if (!setupChecked) {
    return <div className="h-screen bg-background" />;
  }

  if (!setupCompleted) {
    return <SetupWizard onComplete={() => setSetupCompleted(true)} />;
  }

  if (!mysqlChecked) {
    return <div className="h-screen bg-background" />;
  }

  return (
    <div className="flex h-screen bg-background text-foreground">
      <CommandPalette />
      <GlobalSearch />
      <CrashWatch />
      <Sidebar />
      <main className="flex-1 overflow-y-auto p-6">
        <div className={cn("h-full", section !== "dashboard" && "hidden")}>
          <Dashboard />
        </div>
        <SectionSlot section="server-control" active={section} visited={visited}><ServerControl /></SectionSlot>
        <SectionSlot section="build-deploy" active={section} visited={visited}><BuildDeploy /></SectionSlot>
        <SectionSlot section="server-events" active={section} visited={visited}><ServerEvents /></SectionSlot>
        <SectionSlot section="db-explorer" active={section} visited={visited}><DbExplorer /></SectionSlot>
        <SectionSlot section="shop-editor" active={section} visited={visited}><ShopEditor /></SectionSlot>
        <SectionSlot section="item-editor" active={section} visited={visited}><ItemEditor /></SectionSlot>
        <SectionSlot section="item-proto-explorer" active={section} visited={visited}><ItemProtoExplorer /></SectionSlot>
        <SectionSlot section="item-viewer" active={section} visited={visited}><ItemViewer /></SectionSlot>
        <SectionSlot section="module-importer" active={section} visited={visited}><ModuleImporter /></SectionSlot>
        <SectionSlot section="mob-proto-editor" active={section} visited={visited}><MobProtoEditor /></SectionSlot>
        <SectionSlot section="mob-drop-editor" active={section} visited={visited}><MobDropEditor /></SectionSlot>
        <SectionSlot section="drop-generator" active={section} visited={visited}><DropGenerator /></SectionSlot>
        <SectionSlot section="refine-editor" active={section} visited={visited}><RefineEditor /></SectionSlot>
        <SectionSlot section="box-editor" active={section} visited={visited}><BoxEditor /></SectionSlot>
        <SectionSlot section="cube-editor" active={section} visited={visited}><CubeEditor /></SectionSlot>
        <SectionSlot section="quest-builder" active={section} visited={visited}><QuestBuilder /></SectionSlot>
        <SectionSlot section="regen-editor" active={section} visited={visited}><RegenEditor /></SectionSlot>
        <SectionSlot section="locale-editor" active={section} visited={visited}><LocaleEditor /></SectionSlot>
        <SectionSlot section="backup-browser" active={section} visited={visited}><BackupBrowser /></SectionSlot>
        <SectionSlot section="db-backups" active={section} visited={visited}><DbBackups /></SectionSlot>
        <SectionSlot section="activity-log" active={section} visited={visited}><ActivityLog /></SectionSlot>
        <SectionSlot section="tga-converter" active={section} visited={visited}><TgaConverter /></SectionSlot>
        <SectionSlot section="icon-browser" active={section} visited={visited}><IconBrowser /></SectionSlot>
        <SectionSlot section="model-viewer" active={section} visited={visited}><ModelViewer /></SectionSlot>
        <SectionSlot section="account-manager" active={section} visited={visited}><AccountManager /></SectionSlot>
        <SectionSlot section="guild-manager" active={section} visited={visited}><GuildManager /></SectionSlot>
        <SectionSlot section="economy-dashboard" active={section} visited={visited}><EconomyDashboard /></SectionSlot>
        <SectionSlot section="gm-manager" active={section} visited={visited}><GmManager /></SectionSlot>
        <SectionSlot section="system-installer" active={section} visited={visited}><SystemInstaller /></SectionSlot>
        <SectionSlot section="broadcast-system" active={section} visited={visited}><BroadcastSystem /></SectionSlot>
        <SectionSlot section="weather-control" active={section} visited={visited}><WeatherControl /></SectionSlot>
        <SectionSlot section="settings" active={section} visited={visited}><Settings /></SectionSlot>
      </main>
    </div>
  );
}

export default App;
