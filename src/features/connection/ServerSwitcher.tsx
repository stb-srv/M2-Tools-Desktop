import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { toast } from "@/components/ui/toast";
import { Loader2, Pencil, Plus, Server } from "lucide-react";
import { useConnectionProfiles } from "@/store/connectionProfiles";
import { openSettings } from "@/store/settingsSection";
import { NewProfileDialog } from "./NewProfileDialog";

/**
 * Server-Auswahl ganz oben auf der Startseite: aktives Profil wechseln oder
 * direkt ein neues anlegen, ohne erst in die Einstellungen zu müssen.
 */
export function ServerSwitcher() {
  const { profiles, activeId, loaded, reload, activate } = useConnectionProfiles();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [switching, setSwitching] = useState(false);

  useEffect(() => {
    (async () => {
      await reload().catch(() => {});
      await adoptLegacyConnection();
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function onSelect(id: number) {
    if (id === activeId) return;
    const profile = profiles.find((p) => p.id === id);
    setSwitching(true);
    try {
      await activate(id);
      toast.success(`Verbunden mit „${profile?.name ?? id}“`);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setSwitching(false);
    }
  }

  const active = profiles.find((p) => p.id === activeId);

  return (
    <section className="rounded-lg border border-border bg-card p-4">
      {loaded && profiles.length === 0 ? (
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <Server className="size-5 text-muted-foreground" />
            <div>
              <div className="font-medium">Noch kein Server eingerichtet</div>
              <div className="text-sm text-muted-foreground">
                Leg ein Profil an: erst den Namen, danach die Zugangsdaten.
              </div>
            </div>
          </div>
          <Button onClick={() => setDialogOpen(true)}>
            <Plus className="size-4" />
            Ersten Server anlegen
          </Button>
        </div>
      ) : (
        <div className="flex flex-wrap items-center gap-3">
          <Server className="size-5 shrink-0 text-primary" />
          <div className="min-w-0 flex-1 space-y-1">
            <div className="flex items-center gap-2">
              <select
                value={activeId ?? ""}
                onChange={(e) => onSelect(Number(e.target.value))}
                disabled={switching || !loaded}
                aria-label="Aktiver Server"
                className="min-w-48 rounded-md border border-border bg-background px-2 py-1.5 text-sm font-medium"
              >
                {activeId === null && (
                  <option value="" disabled>
                    Server auswählen…
                  </option>
                )}
                {profiles.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
              {switching && <Loader2 className="size-4 animate-spin text-muted-foreground" />}
            </div>
            <div className="truncate text-xs text-muted-foreground">
              {active
                ? `SSH ${active.ssh_username}@${active.ssh_host}:${active.ssh_port} · MySQL ${active.mysql_username}@${active.mysql_host}:${active.mysql_port}`
                : "Kein Profil aktiv - wähle einen Server aus der Liste."}
            </div>
          </div>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={() => openSettings("connection")}>
              <Pencil className="size-3.5" />
              Bearbeiten
            </Button>
            <Button size="sm" onClick={() => setDialogOpen(true)}>
              <Plus className="size-3.5" />
              Neues Profil
            </Button>
          </div>
        </div>
      )}

      {dialogOpen && <NewProfileDialog onClose={() => setDialogOpen(false)} />}
    </section>
  );
}

// Wer M2Manager schon vor den Profilen benutzt hat, hat eine eingerichtete
// Verbindung, aber kein Profil. Die wird einmalig als erstes Profil übernommen,
// damit die Startseite sofort den gewohnten Server zeigt.
async function adoptLegacyConnection() {
  const { profiles, loaded, reload } = useConnectionProfiles.getState();
  if (!loaded || profiles.length > 0) return;
  const host = await invoke<string | null>("get_setting", { key: "ssh_host" }).catch(() => null);
  if (!host) return;
  await invoke("save_connection_profile", { name: host }).catch(() => {});
  await reload().catch(() => {});
}
