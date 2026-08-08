import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { useNavigationStore } from "@/store/navigation";
import { openManual } from "@/lib/manual";
import {
  CheckCircle2,
  XCircle,
  RefreshCw,
  Terminal,
  Store,
  Box,
  Settings as SettingsIcon,
  Network,
  MemoryStick,
  HardDrive,
  HelpCircle,
} from "lucide-react";

interface DatabaseStats {
  accounts: number;
  players: number;
  items: number;
  shops: number;
  mobs: number;
}

interface ProcessUsage {
  pid: number;
  cpu_percent: number;
  mem_percent: number;
  rss_kb: number;
  command: string;
}

interface MemoryInfo {
  total_bytes: number;
  free_bytes: number;
}

interface DiskInfo {
  total_kb: number;
  used_kb: number;
  avail_kb: number;
  capacity_percent: number;
  mount_point: string;
}

interface ServerOverview {
  ip_address: string | null;
  memory: MemoryInfo | null;
  disk: DiskInfo | null;
}

type Status = "checking" | "ok" | "error";

function formatGiB(bytes: number): string {
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

function formatGiBFromKb(kb: number): string {
  return formatGiB(kb * 1024);
}

export function Dashboard() {
  const { t } = useTranslation();
  const setSection = useNavigationStore((s) => s.setSection);

  const [sshStatus, setSshStatus] = useState<Status>("checking");
  const [sshError, setSshError] = useState<string | null>(null);
  const [mysqlStatus, setMysqlStatus] = useState<Status>("checking");
  const [mysqlError, setMysqlError] = useState<string | null>(null);
  const [stats, setStats] = useState<DatabaseStats | null>(null);
  const [processes, setProcesses] = useState<ProcessUsage[] | null>(null);
  const [processError, setProcessError] = useState<string | null>(null);
  const [overview, setOverview] = useState<ServerOverview | null>(null);
  const [overviewError, setOverviewError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  async function refresh() {
    setRefreshing(true);
    setSshStatus("checking");
    setMysqlStatus("checking");
    setSshError(null);
    setMysqlError(null);

    try {
      await invoke("test_stored_ssh");
      setSshStatus("ok");
      try {
        setProcesses(await invoke<ProcessUsage[]>("get_server_resource_usage"));
        setProcessError(null);
      } catch (e) {
        setProcesses(null);
        setProcessError(String(e));
      }
      try {
        setOverview(await invoke<ServerOverview>("get_server_overview"));
        setOverviewError(null);
      } catch (e) {
        setOverview(null);
        setOverviewError(String(e));
      }
    } catch (e) {
      setSshStatus("error");
      setSshError(String(e));
      setProcesses(null);
      setOverview(null);
    }

    try {
      const connected = await invoke<boolean>("is_mysql_connected");
      if (!connected) {
        setMysqlStatus("error");
        setMysqlError("Nicht verbunden");
        setStats(null);
      } else {
        const result = await invoke<DatabaseStats>("get_database_stats");
        setStats(result);
        setMysqlStatus("ok");
      }
    } catch (e) {
      setMysqlStatus("error");
      setMysqlError(String(e));
      setStats(null);
    } finally {
      setRefreshing(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h1 className="text-2xl font-semibold">{t("dashboard.title")}</h1>
          <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("dashboard")}>
            <HelpCircle className="size-4" />
          </Button>
        </div>
        <Button variant="outline" onClick={refresh} disabled={refreshing}>
          <RefreshCw className={`size-4 ${refreshing ? "animate-spin" : ""}`} />
          Aktualisieren
        </Button>
      </div>

      <section className="space-y-2">
        <h2 className="text-sm font-medium text-muted-foreground">
          {t("dashboard.connectionStatus")}
        </h2>
        <div className="grid gap-3 sm:grid-cols-2">
          <StatusCard
            label={t("dashboard.ssh")}
            status={sshStatus}
            detail={sshError}
            onConfigure={() => setSection("settings")}
          />
          <StatusCard
            label={t("dashboard.mysql")}
            status={mysqlStatus}
            detail={mysqlError}
            onConfigure={() => setSection("settings")}
          />
        </div>
      </section>

      {sshStatus === "ok" && (
        <section className="space-y-2">
          <h2 className="text-sm font-medium text-muted-foreground">Server-Übersicht</h2>
          {overviewError && <p className="text-sm text-destructive">{overviewError}</p>}
          {overview && (
            <div className="grid gap-3 sm:grid-cols-3">
              <div className="rounded-lg border border-border bg-card p-3">
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Network className="size-3.5" />
                  IP-Adresse
                </div>
                <div className="mt-1 font-mono text-sm">
                  {overview.ip_address || "Nicht konfiguriert"}
                </div>
              </div>

              <div className="rounded-lg border border-border bg-card p-3">
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <MemoryStick className="size-3.5" />
                  Arbeitsspeicher (RAM)
                </div>
                {overview.memory ? (
                  <>
                    <div className="mt-1 text-sm">
                      {formatGiB(overview.memory.total_bytes - overview.memory.free_bytes)} von{" "}
                      {formatGiB(overview.memory.total_bytes)} belegt
                    </div>
                    <div className="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-muted">
                      <div
                        className="h-full rounded-full bg-primary"
                        style={{
                          width: `${Math.min(100, (1 - overview.memory.free_bytes / overview.memory.total_bytes) * 100)}%`,
                        }}
                      />
                    </div>
                    <p className="mt-1 text-[10px] text-muted-foreground">
                      Nur echte freie Seiten gezählt, kein wiederverwendbarer Cache - kann von
                      z.B. <code>top</code> abweichen.
                    </p>
                  </>
                ) : (
                  <div className="mt-1 text-sm text-muted-foreground">Nicht verfügbar</div>
                )}
              </div>

              <div className="rounded-lg border border-border bg-card p-3">
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <HardDrive className="size-3.5" />
                  Festplatte {overview.disk && `(${overview.disk.mount_point})`}
                </div>
                {overview.disk ? (
                  <>
                    <div className="mt-1 text-sm">
                      {formatGiBFromKb(overview.disk.used_kb)} von{" "}
                      {formatGiBFromKb(overview.disk.total_kb)} belegt (
                      {overview.disk.capacity_percent}%)
                    </div>
                    <div className="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-muted">
                      <div
                        className="h-full rounded-full bg-primary"
                        style={{ width: `${Math.min(100, overview.disk.capacity_percent)}%` }}
                      />
                    </div>
                  </>
                ) : (
                  <div className="mt-1 text-sm text-muted-foreground">
                    Nicht verfügbar - Pfad unter Einstellungen → Server prüfen.
                  </div>
                )}
              </div>
            </div>
          )}
        </section>
      )}

      {sshStatus === "ok" && (
        <section className="space-y-2">
          <h2 className="text-sm font-medium text-muted-foreground">Server-Ressourcen</h2>
          {processError && <p className="text-sm text-destructive">{processError}</p>}
          {processes && processes.length === 0 && (
            <p className="text-sm text-muted-foreground">
              Keine passenden Prozesse gefunden - Prozessnamen unter Einstellungen → Server prüfen.
            </p>
          )}
          {processes && processes.length > 0 && (
            <div className="overflow-hidden rounded-lg border border-border">
              <table className="w-full text-sm">
                <thead className="bg-muted/40 text-xs text-muted-foreground">
                  <tr>
                    <th className="px-3 py-1.5 text-left">Prozess</th>
                    <th className="px-3 py-1.5 text-left">PID</th>
                    <th className="px-3 py-1.5 text-left">CPU</th>
                    <th className="px-3 py-1.5 text-left">RAM</th>
                    <th className="px-3 py-1.5 text-left">RSS</th>
                  </tr>
                </thead>
                <tbody>
                  {processes.map((p) => (
                    <tr key={p.pid} className="border-t border-border">
                      <td className="px-3 py-1.5 font-mono">{p.command}</td>
                      <td className="px-3 py-1.5 tabular-nums">{p.pid}</td>
                      <td className="px-3 py-1.5 tabular-nums">{p.cpu_percent.toFixed(1)}%</td>
                      <td className="px-3 py-1.5 tabular-nums">{p.mem_percent.toFixed(1)}%</td>
                      <td className="px-3 py-1.5 tabular-nums">
                        {(p.rss_kb / 1024).toFixed(0)} MB
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      )}

      {stats && (
        <section className="space-y-2">
          <h2 className="text-sm font-medium text-muted-foreground">
            Datenbank-Statistik
          </h2>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-5">
            <StatCard label="Accounts" value={stats.accounts} />
            <StatCard label="Spieler" value={stats.players} />
            <StatCard label="Items" value={stats.items} />
            <StatCard label="Shops" value={stats.shops} />
            <StatCard label="Mobs/NPCs" value={stats.mobs} />
          </div>
        </section>
      )}

      <section className="space-y-2">
        <h2 className="text-sm font-medium text-muted-foreground">Schnellzugriff</h2>
        <div className="flex flex-wrap gap-2">
          <QuickAction
            icon={Terminal}
            label={t("nav.serverControl")}
            onClick={() => setSection("server-control")}
          />
          <QuickAction
            icon={Store}
            label={t("nav.shopEditor")}
            onClick={() => setSection("shop-editor")}
          />
          <QuickAction
            icon={Box}
            label={t("nav.modelViewer")}
            onClick={() => setSection("model-viewer")}
          />
          <QuickAction
            icon={SettingsIcon}
            label={t("nav.settings")}
            onClick={() => setSection("settings")}
          />
        </div>
      </section>
    </div>
  );
}

function StatusCard({
  label,
  status,
  detail,
  onConfigure,
}: {
  label: string;
  status: Status;
  detail: string | null;
  onConfigure: () => void;
}) {
  return (
    <div className="rounded-lg border border-border bg-card p-3">
      <div className="flex items-center justify-between">
        <span className="font-medium">{label}</span>
        {status === "checking" && (
          <span className="text-sm text-muted-foreground">Prüfe…</span>
        )}
        {status === "ok" && (
          <span className="flex items-center gap-1 text-sm text-green-600">
            <CheckCircle2 className="size-4" />
            Verbunden
          </span>
        )}
        {status === "error" && (
          <span className="flex items-center gap-1 text-sm text-destructive">
            <XCircle className="size-4" />
            Nicht verbunden
          </span>
        )}
      </div>
      {status === "error" && (
        <div className="mt-2 space-y-2">
          {detail && (
            <p className="line-clamp-2 text-xs text-muted-foreground">{detail}</p>
          )}
          <Button variant="outline" size="sm" onClick={onConfigure}>
            Einrichten
          </Button>
        </div>
      )}
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-lg border border-border bg-card p-3">
      <div className="text-xl font-semibold tabular-nums">
        {value.toLocaleString("de-DE")}
      </div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </div>
  );
}

function QuickAction({
  icon: Icon,
  label,
  onClick,
}: {
  icon: typeof Terminal;
  label: string;
  onClick: () => void;
}) {
  return (
    <Button variant="outline" onClick={onClick}>
      <Icon className="size-4" />
      {label}
    </Button>
  );
}
