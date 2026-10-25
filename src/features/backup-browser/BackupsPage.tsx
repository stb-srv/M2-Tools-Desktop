import { SectionTabs } from "@/features/shared/SectionTabs";
import { DbBackups } from "@/features/db-backups/DbBackups";
import { BackupBrowser } from "./BackupBrowser";

// Datei-Backups (die `m2manager_backups`-Ordner der Editoren) und komplette
// Datenbank-Sicherungen unter einem gemeinsamen Sidebar-Punkt "Backups".
export function BackupsPage() {
  return (
    <SectionTabs
      tabs={[
        { id: "files", label: "Datei-Backups", content: <BackupBrowser /> },
        { id: "database", label: "Datenbank-Backups", content: <DbBackups /> },
      ]}
    />
  );
}
