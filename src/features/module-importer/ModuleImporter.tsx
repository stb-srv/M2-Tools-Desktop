import { useState } from "react";
import { PackageImporter } from "./components/PackageImporter";
import { IconItemImporter } from "./components/IconItemImporter";
import { ImportHistory } from "./components/ImportHistory";
import { QuickRemoveItem } from "./components/QuickRemoveItem";
import { PageHeader } from "@/components/layout/PageHeader";

// Re-exported so ModuleImporter.test.ts (colocated, per the project's test
// convention) can keep importing from "./ModuleImporter" - the actual
// implementation lives in ./shared, alongside the rest of this feature's
// non-JSX types/helpers.
export { fitToByteLimit, fitToByteLimitWithSuffix, ITEM_NAME_MAX_BYTES } from "./shared";

type ImporterMode = "package" | "icon-only";

export function ModuleImporter() {
  const [historyVersion, setHistoryVersion] = useState(0);
  const [mode, setMode] = useState<ImporterMode>("package");

  return (
    <div className="space-y-6 pb-10">
      <PageHeader module="module-importer" title="Modul-Importer" />

      <div className="flex overflow-hidden rounded-md border border-border text-sm w-fit">
        <button
          onClick={() => setMode("package")}
          className={`px-3 py-1 ${mode === "package" ? "bg-primary text-primary-foreground" : ""}`}
        >
          Waffen / Rüstung (mit 3D-Modell)
        </button>
        <button
          onClick={() => setMode("icon-only")}
          className={`px-3 py-1 ${mode === "icon-only" ? "bg-primary text-primary-foreground" : ""}`}
        >
          Zubehör (nur Icon, z. B. Schuhe/Ketten/Schilder)
        </button>
      </div>

      {mode === "package" ? (
        <>
          <p className="text-sm text-muted-foreground">
            Erkennt fertige Ausrüstungs-Pakete (Icons + 3D-Modelle, z. B. von Grafikern gelieferte Waffen-/
            Rüstungs-Sets - auch gemischt in einem Ordner) automatisch und legt daraus Items mit Beispielwerten
            an - inklusive Icon, 3D-Modell und aller nötigen Client-Dateien.
          </p>
          <PackageImporter onImported={() => setHistoryVersion((v) => v + 1)} />
        </>
      ) : (
        <>
          <p className="text-sm text-muted-foreground">
            Für Items ganz ohne 3D-Modell: legt aus einem Ordner voller Icon-Dateien beliebig viele Items an,
            Typ/Subtyp/Werte pro Item einzeln einstellbar.
          </p>
          <IconItemImporter onImported={() => setHistoryVersion((v) => v + 1)} />
        </>
      )}

      <ImportHistory refreshKey={historyVersion} />
      <QuickRemoveItem />
    </div>
  );
}
