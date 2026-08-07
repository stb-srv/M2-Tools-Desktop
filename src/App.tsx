import "@/i18n";
import { lazy, Suspense, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { autoConnectMysql } from "@/lib/mysqlConnect";
import { Sidebar } from "@/components/layout/Sidebar";
import { CommandPalette } from "@/components/CommandPalette";
import { CrashWatch } from "@/components/CrashWatch";
import { useNavigationStore } from "@/store/navigation";
import { Dashboard } from "@/features/dashboard/Dashboard";
import { SetupWizard } from "@/features/setup/SetupWizard";

// Lazy-loaded: each of these pulls in its own chunk (three.js for the model
// viewer/shop preview, CodeMirror for the quest builder, ...) - eagerly
// importing all 18 into App.tsx's own bundle is what produced the single
// 1.5MB chunk Vite warned about. Only Dashboard (the default section, see
// navigation.ts) stays eager for a fast first paint; everything else loads
// on first visit to that section.
const ServerControl = lazy(() => import("@/features/server-control/ServerControl").then((m) => ({ default: m.ServerControl })));
const BuildDeploy = lazy(() => import("@/features/build-deploy/BuildDeploy").then((m) => ({ default: m.BuildDeploy })));
const DbExplorer = lazy(() => import("@/features/db-explorer/DbExplorer").then((m) => ({ default: m.DbExplorer })));
const ShopEditor = lazy(() => import("@/features/shop-editor/ShopEditor").then((m) => ({ default: m.ShopEditor })));
const ItemEditor = lazy(() => import("@/features/item-editor/ItemEditor").then((m) => ({ default: m.ItemEditor })));
const ModuleImporter = lazy(() => import("@/features/module-importer/ModuleImporter").then((m) => ({ default: m.ModuleImporter })));
const MobProtoEditor = lazy(() => import("@/features/mob-proto-editor/MobProtoEditor").then((m) => ({ default: m.MobProtoEditor })));
const MobDropEditor = lazy(() => import("@/features/mob-drop-editor/MobDropEditor").then((m) => ({ default: m.MobDropEditor })));
const RefineEditor = lazy(() => import("@/features/refine-editor/RefineEditor").then((m) => ({ default: m.RefineEditor })));
const BoxEditor = lazy(() => import("@/features/box-editor/BoxEditor").then((m) => ({ default: m.BoxEditor })));
const QuestBuilder = lazy(() => import("@/features/quest-builder/QuestBuilder").then((m) => ({ default: m.QuestBuilder })));
const RegenEditor = lazy(() => import("@/features/regen-editor/RegenEditor").then((m) => ({ default: m.RegenEditor })));
const LocaleEditor = lazy(() => import("@/features/locale-editor/LocaleEditor").then((m) => ({ default: m.LocaleEditor })));
const BackupBrowser = lazy(() => import("@/features/backup-browser/BackupBrowser").then((m) => ({ default: m.BackupBrowser })));
const DbBackups = lazy(() => import("@/features/db-backups/DbBackups").then((m) => ({ default: m.DbBackups })));
const TgaConverter = lazy(() => import("@/features/tga-converter/TgaConverter").then((m) => ({ default: m.TgaConverter })));
const IconBrowser = lazy(() => import("@/features/icon-browser/IconBrowser").then((m) => ({ default: m.IconBrowser })));
const ModelViewer = lazy(() => import("@/features/model-viewer/ModelViewer").then((m) => ({ default: m.ModelViewer })));
const AccountManager = lazy(() => import("@/features/account-manager/AccountManager").then((m) => ({ default: m.AccountManager })));
const Settings = lazy(() => import("@/features/settings/Settings").then((m) => ({ default: m.Settings })));

function LoadingFallback() {
  return (
    <div className="flex h-full items-center justify-center">
      <span className="size-6 animate-spin rounded-full border-2 border-muted-foreground border-t-transparent" />
    </div>
  );
}

function App() {
  const section = useNavigationStore((state) => state.section);
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
      <CrashWatch />
      <Sidebar />
      <main className="flex-1 overflow-y-auto p-6">
        {section === "dashboard" && <Dashboard />}
        <Suspense fallback={<LoadingFallback />}>
          {section === "server-control" && <ServerControl />}
          {section === "build-deploy" && <BuildDeploy />}
          {section === "db-explorer" && <DbExplorer />}
          {section === "shop-editor" && <ShopEditor />}
          {section === "item-editor" && <ItemEditor />}
          {section === "module-importer" && <ModuleImporter />}
          {section === "mob-proto-editor" && <MobProtoEditor />}
          {section === "mob-drop-editor" && <MobDropEditor />}
          {section === "refine-editor" && <RefineEditor />}
          {section === "box-editor" && <BoxEditor />}
          {section === "quest-builder" && <QuestBuilder />}
          {section === "regen-editor" && <RegenEditor />}
          {section === "locale-editor" && <LocaleEditor />}
          {section === "backup-browser" && <BackupBrowser />}
          {section === "db-backups" && <DbBackups />}
          {section === "tga-converter" && <TgaConverter />}
          {section === "icon-browser" && <IconBrowser />}
          {section === "model-viewer" && <ModelViewer />}
          {section === "account-manager" && <AccountManager />}
          {section === "settings" && <Settings />}
        </Suspense>
      </main>
    </div>
  );
}

export default App;
