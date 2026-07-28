import { useTranslation } from "react-i18next";

export function DbExplorer() {
  const { t } = useTranslation();
  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-semibold">{t("nav.dbExplorer")}</h1>
      <p className="text-sm text-muted-foreground">
        Schema-Mapping folgt, sobald die Metin2-Core-Tabellenstruktur
        gemeinsam festgelegt ist.
      </p>
    </div>
  );
}
