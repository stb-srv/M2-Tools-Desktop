import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Server, X } from "lucide-react";
import { toast } from "@/components/ui/toast";
import {
  EMPTY_MYSQL,
  EMPTY_SSH,
  mysqlComplete,
  sshComplete,
  type MysqlValues,
  type SshValues,
} from "@/lib/connection";
import { useConnectionProfiles } from "@/store/connectionProfiles";
import { cn } from "@/lib/utils";
import { MysqlFields, MysqlTestButton, SshFields, SshTestButton, TextField } from "./ConnectionFields";

type Step = "name" | "ssh" | "mysql";
const STEPS: { id: Step; label: string }[] = [
  { id: "name", label: "Name" },
  { id: "ssh", label: "SSH" },
  { id: "mysql", label: "Datenbank" },
];

/**
 * Legt ein neues Server-Profil an: erst nur der Name, dann SSH, dann MySQL.
 * Am Ende wird das Profil gespeichert und sofort zur aktiven Verbindung.
 */
export function NewProfileDialog({ onClose }: { onClose: () => void }) {
  const profiles = useConnectionProfiles((s) => s.profiles);
  const create = useConnectionProfiles((s) => s.create);

  const [stepIndex, setStepIndex] = useState(0);
  const step = STEPS[stepIndex].id;
  const [name, setName] = useState("");
  const [ssh, setSsh] = useState<SshValues>(EMPTY_SSH);
  const [mysql, setMysql] = useState<MysqlValues>(EMPTY_MYSQL);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const trimmedName = name.trim();
  const nameTaken = profiles.some((p) => p.name.toLowerCase() === trimmedName.toLowerCase());

  const canContinue =
    (step === "name" && !!trimmedName && !nameTaken) ||
    (step === "ssh" && sshComplete(ssh)) ||
    (step === "mysql" && mysqlComplete(mysql));

  function next() {
    // Datenbank liegt meistens auf demselben Server - Host als Vorschlag übernehmen.
    if (step === "ssh" && !mysql.host) setMysql({ ...mysql, host: ssh.host });
    setStepIndex((i) => i + 1);
  }

  async function finish() {
    setSaving(true);
    setError(null);
    try {
      await create(trimmedName, ssh, mysql);
      toast.success(`Profil „${trimmedName}“ angelegt und aktiviert`);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
      <div className="flex max-h-[90vh] w-[30rem] flex-col rounded-lg border border-border bg-card">
        <div className="flex items-center justify-between border-b border-border px-4 py-3">
          <div className="flex items-center gap-2">
            <Server className="size-4 text-primary" />
            <h2 className="text-sm font-semibold">Neues Server-Profil</h2>
          </div>
          <Button variant="ghost" size="icon-sm" onClick={onClose} title="Abbrechen">
            <X className="size-4" />
          </Button>
        </div>

        <div className="flex gap-1 px-4 pt-3">
          {STEPS.map((s, i) => (
            <div key={s.id} className="flex-1 space-y-1">
              <div className={cn("h-1 rounded-full", i <= stepIndex ? "bg-primary" : "bg-muted")} />
              <div
                className={cn(
                  "text-[11px]",
                  i === stepIndex ? "font-medium text-foreground" : "text-muted-foreground",
                )}
              >
                {i + 1}. {s.label}
              </div>
            </div>
          ))}
        </div>

        <div className="space-y-3 overflow-y-auto px-4 py-4">
          {step === "name" && (
            <>
              <p className="text-sm text-muted-foreground">
                Wie soll der Server heißen? Unter diesem Namen wählst du ihn später auf der
                Startseite aus.
              </p>
              <TextField
                label="Profilname"
                value={name}
                onChange={setName}
                placeholder="z.B. Live-Server oder Testserver"
                autoFocus
              />
              {nameTaken && (
                <p className="text-sm text-destructive">Ein Profil mit diesem Namen gibt es schon.</p>
              )}
            </>
          )}

          {step === "ssh" && (
            <>
              <p className="text-sm text-muted-foreground">
                SSH-Zugang zum Gameserver - für Server-Steuerung, Logs, Quests und alle Dateien auf
                dem Server.
              </p>
              <SshFields value={ssh} onChange={setSsh} />
              <SshTestButton value={ssh} />
            </>
          )}

          {step === "mysql" && (
            <>
              <p className="text-sm text-muted-foreground">
                MySQL-Zugang - für Item-, Shop- und Mob-Editor, Accounts und die Statistiken auf
                der Startseite.
              </p>
              <MysqlFields value={mysql} onChange={setMysql} />
              <MysqlTestButton value={mysql} />
            </>
          )}

          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>

        <div className="flex justify-between border-t border-border px-4 py-3">
          <Button
            variant="ghost"
            onClick={() => (stepIndex === 0 ? onClose() : setStepIndex((i) => i - 1))}
            disabled={saving}
          >
            {stepIndex === 0 ? "Abbrechen" : "Zurück"}
          </Button>
          {stepIndex < STEPS.length - 1 ? (
            <Button onClick={next} disabled={!canContinue}>
              Weiter
            </Button>
          ) : (
            <Button onClick={finish} disabled={!canContinue || saving}>
              {saving ? "Speichere…" : "Profil anlegen & verbinden"}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
