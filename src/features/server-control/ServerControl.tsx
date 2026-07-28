import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

export function ServerControl() {
  const { t } = useTranslation();
  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-semibold">{t("serverControl.title")}</h1>
      <div className="flex flex-wrap gap-2">
        <Button>{t("serverControl.startServer")}</Button>
        <Button variant="destructive">{t("serverControl.stopServer")}</Button>
        <Button variant="outline">{t("serverControl.reloadQuests")}</Button>
        <Button variant="outline">{t("serverControl.clearLogs")}</Button>
      </div>
    </div>
  );
}
