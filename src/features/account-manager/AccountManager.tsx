import { Button } from "@/components/ui/button";
import { Users, HelpCircle } from "lucide-react";
import { openManual } from "@/lib/manual";
import { PLAYER_TABLE } from "./shared";
import { AccountSection } from "./components/AccountSection";
import { TableSearchSection } from "./components/TableSearchSection";
import { GiveItemSection } from "./components/GiveItemSection";

export function AccountManager() {
  return (
    <div className="max-w-4xl space-y-8 pb-10">
      <div className="flex items-center gap-2">
        <Users className="size-6 text-muted-foreground" />
        <h1 className="text-2xl font-semibold">Account-/Spieler-Verwaltung</h1>
        <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("account-manager")}>
          <HelpCircle className="size-4" />
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
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
