import { useTranslation } from "react-i18next";

export function Connections() {
  const { t } = useTranslation();
  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-semibold">{t("connections.title")}</h1>
      <p className="text-sm text-muted-foreground">
        {t("connections.ssh")} / {t("connections.mysql")}
      </p>
    </div>
  );
}
