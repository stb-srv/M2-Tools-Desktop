import "@/i18n";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Sidebar } from "@/components/layout/Sidebar";
import { useNavigationStore } from "@/store/navigation";
import { Dashboard } from "@/features/dashboard/Dashboard";
import { Connections } from "@/features/connections/Connections";
import { ServerControl } from "@/features/server-control/ServerControl";
import { DbExplorer } from "@/features/db-explorer/DbExplorer";
import { ShopEditor } from "@/features/shop-editor/ShopEditor";
import { ModelViewer } from "@/features/model-viewer/ModelViewer";
import { Settings } from "@/features/settings/Settings";
import { SetupWizard } from "@/features/setup/SetupWizard";

function App() {
  const section = useNavigationStore((state) => state.section);
  const [setupChecked, setSetupChecked] = useState(false);
  const [setupCompleted, setSetupCompleted] = useState(true);

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "setup_completed" })
      .then((value) => setSetupCompleted(value === "true"))
      .finally(() => setSetupChecked(true));
  }, []);

  if (!setupChecked) {
    return <div className="h-screen bg-background" />;
  }

  if (!setupCompleted) {
    return <SetupWizard onComplete={() => setSetupCompleted(true)} />;
  }

  return (
    <div className="flex h-screen bg-background text-foreground">
      <Sidebar />
      <main className="flex-1 overflow-y-auto p-6">
        {section === "dashboard" && <Dashboard />}
        {section === "connections" && <Connections />}
        {section === "server-control" && <ServerControl />}
        {section === "db-explorer" && <DbExplorer />}
        {section === "shop-editor" && <ShopEditor />}
        {section === "model-viewer" && <ModelViewer />}
        {section === "settings" && <Settings />}
      </main>
    </div>
  );
}

export default App;
