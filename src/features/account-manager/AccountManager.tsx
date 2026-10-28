import { PLAYER_TABLE } from "./shared";
import { AccountSection } from "./components/AccountSection";
import { TableSearchSection } from "./components/TableSearchSection";
import { GiveItemSection } from "./components/GiveItemSection";
import { SectionTabs } from "@/features/shared/SectionTabs";
import { GmManager } from "@/features/gm-manager/GmManager";
import { PageHeader } from "@/components/layout/PageHeader";

// GM-Rechte (`common.gmlist`) hängen an Account + Charakter und leben deshalb
// als zweiter Tab hier statt als eigener Sidebar-Punkt.
export function AccountManager() {
  return (
    <SectionTabs
      tabs={[
        { id: "accounts", label: "Accounts & Spieler", content: <AccountsAndPlayers /> },
        { id: "gm", label: "GM-Verwaltung", content: <GmManager /> },
      ]}
    />
  );
}

function AccountsAndPlayers() {
  return (
    <div className="space-y-8 pb-10">
      <PageHeader module="account-manager" title="Account-/Spieler-Verwaltung" />
      <p className="max-w-4xl text-sm text-muted-foreground">
        Es gibt keinen Kanal zu einem laufenden Spielprozess - Änderungen an bereits online
        befindlichen Spielern wirken daher unter Umständen erst nach Neuanmeldung, nicht sofort
        live.
      </p>

      <AccountSection />
      <TableSearchSection
        title="Spieler"
        target={PLAYER_TABLE}
        defaultColumn="name"
        placeholder="Charaktername suchen…"
        playerTools
      />
      <GiveItemSection />
    </div>
  );
}
