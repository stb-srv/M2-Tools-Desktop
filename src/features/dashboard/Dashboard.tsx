import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { useNavigationStore } from "@/store/navigation";
import {
  CheckCircle2,
  XCircle,
  RefreshCw,
  Terminal,
  Store,
  Box,
  Plug,
} from "lucide-react";

interface DatabaseStats {
  accounts: number;
  players: number;
  items: number;
  shops: number;
  mobs: number;
}

type Status = "checking" | "ok" | "error";

export function Dashboard() {
  const { t } = useTranslation();
  const setSection = useNavigationStore((s) => s.setSection);

  const [sshStatus, setSshStatus] = useState<Status>("checking");
  const [sshError, setSshError] = useState<string | null>(null);
  const [mysqlStatus, setMysqlStatus] = useState<Status>("checking");
  const [mysqlError, setMysqlError] = useState<string | null>(null);
  const [stats, setStats] = useState<DatabaseStats | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  async function refresh() {
    setRefreshing(true);
    setSshStatus("checking");
    setMysqlStatus("checking");
    setSshError(null);
    setMysqlError(null);

    invoke("test_stored_ssh")
      .then(() => setSshStatus("ok"))
      .catch((e) => {
        setSshStatus("error");
        setSshError(String(e));
      });

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
        <h1 className="text-2xl font-semibold">{t("dashboard.title")}</h1>
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
            onConfigure={() => setSection("connections")}
          />
          <StatusCard
            label={t("dashboard.mysql")}
            status={mysqlStatus}
            detail={mysqlError}
            onConfigure={() => setSection("connections")}
          />
        </div>
      </section>

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
            icon={Plug}
            label={t("nav.connections")}
            onClick={() => setSection("connections")}
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
