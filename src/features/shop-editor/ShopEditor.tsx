import { useTranslation } from "react-i18next";

export function ShopEditor() {
  const { t } = useTranslation();
  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-semibold">{t("nav.shopEditor")}</h1>
      <p className="text-sm text-muted-foreground">
        Benötigt das DB-Schema-Mapping aus dem Datenbank-Explorer.
      </p>
    </div>
  );
}
