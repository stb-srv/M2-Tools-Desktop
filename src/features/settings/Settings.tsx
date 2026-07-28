import { useTranslation } from "react-i18next";
import { useThemeStore, type Theme } from "@/store/theme";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const THEMES: Theme[] = ["light", "dark", "system"];
const LANGUAGES = [
  { code: "de", label: "Deutsch" },
  { code: "en", label: "English" },
];

export function Settings() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useThemeStore();

  return (
    <div className="space-y-8">
      <h1 className="text-2xl font-semibold">{t("settings.title")}</h1>

      <section className="space-y-2">
        <h2 className="text-sm font-medium text-muted-foreground">
          {t("settings.theme")}
        </h2>
        <div className="flex gap-2">
          {THEMES.map((value) => (
            <Button
              key={value}
              variant={theme === value ? "default" : "outline"}
              onClick={() => setTheme(value)}
            >
              {t(`settings.${value}`)}
            </Button>
          ))}
        </div>
      </section>

      <section className="space-y-2">
        <h2 className="text-sm font-medium text-muted-foreground">
          {t("settings.language")}
        </h2>
        <div className="flex gap-2">
          {LANGUAGES.map((lang) => (
            <Button
              key={lang.code}
              variant={i18n.language === lang.code ? "default" : "outline"}
              onClick={() => i18n.changeLanguage(lang.code)}
              className={cn(i18n.language === lang.code && "font-semibold")}
            >
              {lang.label}
            </Button>
          ))}
        </div>
      </section>
    </div>
  );
}
