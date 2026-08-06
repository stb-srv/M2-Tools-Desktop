import "@/i18n";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { autoConnectMysql } from "@/lib/mysqlConnect";
import { Sidebar } from "@/components/layout/Sidebar";
import { CommandPalette } from "@/components/CommandPalette";
import { CrashWatch } from "@/components/CrashWatch";
import { useNavigationStore } from "@/store/navigation";
import { Dashboard } from "@/features/dashboard/Dashboard";
import { ServerControl } from "@/features/server-control/ServerControl";
import { DbExplorer } from "@/features/db-explorer/DbExplorer";
import { ShopEditor } from "@/features/shop-editor/ShopEditor";
import { ItemEditor } from "@/features/item-editor/ItemEditor";
import { ModuleImporter } from "@/features/module-importer/ModuleImporter";
import { MobProtoEditor } from "@/features/mob-proto-editor/MobProtoEditor";
import { MobDropEditor } from "@/features/mob-drop-editor/MobDropEditor";
import { QuestBuilder } from "@/features/quest-builder/QuestBuilder";
import { RegenEditor } from "@/features/regen-editor/RegenEditor";
import { LocaleEditor } from "@/features/locale-editor/LocaleEditor";
import { BackupBrowser } from "@/features/backup-browser/BackupBrowser";
import { DbBackups } from "@/features/db-backups/DbBackups";
import { TgaConverter } from "@/features/tga-converter/TgaConverter";
import { IconBrowser } from "@/features/icon-browser/IconBrowser";
import { ModelViewer } from "@/features/model-viewer/ModelViewer";
import { AccountManager } from "@/features/account-manager/AccountManager";
import { Settings } from "@/features/settings/Settings";
import { SetupWizard } from "@/features/setup/SetupWizard";

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
        {section === "server-control" && <ServerControl />}
        {section === "db-explorer" && <DbExplorer />}
        {section === "shop-editor" && <ShopEditor />}
        {section === "item-editor" && <ItemEditor />}
        {section === "module-importer" && <ModuleImporter />}
        {section === "mob-proto-editor" && <MobProtoEditor />}
        {section === "mob-drop-editor" && <MobDropEditor />}
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
      </main>
    </div>
  );
}

export default App;
