import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { toast } from "@/components/ui/toast";
import { CheckCircle2, Database, Plus, Server, Terminal, Trash2, type LucideIcon } from "lucide-react";
import {
  EMPTY_MYSQL,
  EMPTY_SSH,
  connectMysql,
  loadStoredConnection,
  mysqlComplete,
  saveMysql,
  saveSsh,
  sshComplete,
  testMysql,
  testSsh,
  type MysqlValues,
  type SshValues,
} from "@/lib/connection";
import { useConnectionProfiles, type ConnectionProfile } from "@/store/connectionProfiles";
import { MysqlFields, SshFields, TestResult, type TestState } from "@/features/connection/ConnectionFields";
import { NewProfileDialog } from "@/features/connection/NewProfileDialog";
import { SettingsCard } from "./shared";

export function ConnectionTab() {
  const { profiles, activeId, revision, reload, activate, remove, syncActive } = useConnectionProfiles();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<number | null>(null);

  const [ssh, setSsh] = useState<SshValues>(EMPTY_SSH);
  const [mysql, setMysql] = useState<MysqlValues>(EMPTY_MYSQL);

  useEffect(() => {
    reload().catch(() => {});
  }, [reload]);

  // Nach jedem Server-Wechsel (auch von der Startseite aus) die Felder neu befüllen.
  useEffect(() => {
    loadStoredConnection()
      .then(({ ssh, mysql }) => {
        setSsh(ssh);
        setMysql(mysql);
      })
      .catch(() => {});
  }, [revision]);

  async function onActivate(profile: ConnectionProfile) {
    setBusyId(profile.id);
    try {
      await activate(profile.id);
      toast.success(`Verbunden mit „${profile.name}“`);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusyId(null);
    }
  }

  async function onDelete(profile: ConnectionProfile) {
    setBusyId(profile.id);
    try {
      await remove(profile.id);
      toast.success(`Profil „${profile.name}“ gelöscht`);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusyId(null);
      setConfirmDeleteId(null);
    }
  }

  const active = profiles.find((p) => p.id === activeId);

  return (
    <div className="space-y-4">

      <SettingsCard
        title="Server-Profile"
        icon={Server}
        description="Jedes Profil merkt sich SSH- und MySQL-Zugang inkl. Passwörter (Windows Anmeldeinformationsverwaltung)."
        actions={
          <Button size="sm" onClick={() => setDialogOpen(true)}>
            <Plus className="size-3.5" />
            Neues Profil
          </Button>
        }
      >
        {profiles.length === 0 && (
          <p className="px-5 py-4 text-sm text-muted-foreground">Noch keine Profile angelegt.</p>
        )}
        {profiles.map((profile) => {
          const isActive = profile.id === activeId;
          return (
            <div
              key={profile.id}
              className={`flex flex-wrap items-center gap-3 border-l-4 px-5 py-3 text-sm ${isActive ? "border-l-green-600 bg-green-600/5" : "border-l-transparent"}`}
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2 text-base font-medium">
                  {profile.name}
                  {isActive && (
                    <span className="flex items-center gap-1 rounded-full bg-green-600/15 px-2 py-0.5 text-[11px] font-medium text-green-600">
                      <CheckCircle2 className="size-3" />
                      Aktiv
                    </span>
                  )}
                </div>
                <div className="truncate text-xs text-muted-foreground">
                  SSH {profile.ssh_host || "-"} · MySQL {profile.mysql_host || "-"}
                </div>
              </div>
              {confirmDeleteId === profile.id ? (
                <>
                  <span className="text-xs text-muted-foreground">Wirklich löschen?</span>
                  <Button size="sm" variant="destructive" onClick={() => onDelete(profile)} disabled={busyId !== null}>
                    Löschen
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => setConfirmDeleteId(null)}>
                    Abbrechen
                  </Button>
                </>
              ) : (
                <>
                  {!isActive && (
                    <Button size="sm" variant="outline" onClick={() => onActivate(profile)} disabled={busyId !== null}>
                      {busyId === profile.id ? "Verbinde…" : "Aktivieren"}
                    </Button>
                  )}
                  <Button
                    size="icon-sm"
                    variant="ghost"
                    onClick={() => setConfirmDeleteId(profile.id)}
                    disabled={busyId !== null}
                    title="Profil löschen"
                  >
                    <Trash2 className="size-3.5" />
                  </Button>
                </>
              )}
            </div>
          );
        })}
      </SettingsCard>

      <div className="grid gap-4 2xl:grid-cols-2">
        <ConnectionEditor
          title="SSH-Zugang"
          icon={Terminal}
          description={
            active
              ? `Gehört zum aktiven Profil „${active.name}“. Für Server-Steuerung, Logs, Quests und alle Server-Dateien.`
              : "Für Server-Steuerung, Logs, Quests und alle Server-Dateien."
          }
          canSave={sshComplete(ssh)}
          values={ssh}
          onSave={async () => {
            await testSsh(ssh);
            await saveSsh(ssh);
            await syncActive();
          }}
        >
          <SshFields value={ssh} onChange={setSsh} />
        </ConnectionEditor>

        <ConnectionEditor
          title="MySQL-Zugang"
          icon={Database}
          description={
            active
              ? `Gehört zum aktiven Profil „${active.name}“. Für alle Datenbank-Editoren und die Statistiken.`
              : "Für alle Datenbank-Editoren und die Statistiken."
          }
          canSave={mysqlComplete(mysql)}
          values={mysql}
          onSave={async () => {
            await testMysql(mysql);
            await connectMysql(mysql);
            await saveMysql(mysql);
            await syncActive();
          }}
        >
          <MysqlFields value={mysql} onChange={setMysql} />
        </ConnectionEditor>
      </div>

      {dialogOpen && <NewProfileDialog onClose={() => setDialogOpen(false)} />}
    </div>
  );
}

function ConnectionEditor({
  title,
  icon,
  description,
  canSave,
  values,
  onSave,
  children,
}: {
  title: string;
  icon: LucideIcon;
  description: string;
  canSave: boolean;
  values: unknown;
  onSave: () => Promise<void>;
  children: React.ReactNode;
}) {
  const [state, setState] = useState<TestState>("idle");
  const [error, setError] = useState<string | null>(null);

  const fingerprint = JSON.stringify(values);
  useEffect(() => {
    setState("idle");
    setError(null);
  }, [fingerprint]);

  async function save() {
    setState("testing");
    setError(null);
    try {
      await onSave();
      setState("ok");
    } catch (e) {
      setState("error");
      setError(String(e));
    }
  }

  return (
    <SettingsCard title={title} description={description} icon={icon} className="h-fit">
      <div className="max-w-3xl space-y-4 px-5 py-4">
        {children}
        <div className="flex items-start gap-3">
          <Button onClick={save} disabled={!canSave || state === "testing"} className="shrink-0">
            {state === "testing" ? "Teste…" : "Testen & speichern"}
          </Button>
          <TestResult state={state} error={error} />
        </div>
      </div>
    </SettingsCard>
  );
}
