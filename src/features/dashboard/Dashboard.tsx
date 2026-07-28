import { useTranslation } from "react-i18next";

export function Dashboard() {
  const { t } = useTranslation();

  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-semibold">{t("dashboard.title")}</h1>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div className="rounded-lg border border-border bg-card p-4">
          <h2 className="text-sm font-medium text-muted-foreground">
            {t("dashboard.connectionStatus")}
          </h2>
          <div className="mt-3 space-y-2">
            <div className="flex items-center justify-between text-sm">
              <span>{t("dashboard.ssh")}</span>
              <span className="text-muted-foreground">
                {t("dashboard.disconnected")}
              </span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span>{t("dashboard.mysql")}</span>
              <span className="text-muted-foreground">
                {t("dashboard.disconnected")}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
