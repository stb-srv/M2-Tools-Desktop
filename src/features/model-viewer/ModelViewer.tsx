import { useTranslation } from "react-i18next";

export function ModelViewer() {
  const { t } = useTranslation();
  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-semibold">{t("nav.modelViewer")}</h1>
      <p className="text-sm text-muted-foreground">
        three.js-Viewer folgt nach dem GR2-Parser-Research-Spike.
      </p>
    </div>
  );
}
