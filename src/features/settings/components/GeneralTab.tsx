import { useTranslation } from "react-i18next";
import { useThemeStore, type Theme } from "@/store/theme";
import { Button } from "@/components/ui/button";
import { SectionHeader, SettingRow, SettingsCard, TextSetting } from "./shared";

const THEMES: Theme[] = ["light", "dark", "system"];
const LANGUAGES = [
  { code: "de", label: "Deutsch" },
  { code: "en", label: "English" },
];

export function GeneralTab() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useThemeStore();

  return (
    <div className="space-y-4">
      <SectionHeader title="Allgemein" description="Aussehen, Sprache und Voreinstellungen für die Editoren." />

      <SettingsCard title="Darstellung">
        <SettingRow label={t("settings.theme")}>
          <div className="flex gap-2">
            {THEMES.map((value) => (
              <Button
                key={value}
                size="sm"
                variant={theme === value ? "default" : "outline"}
                onClick={() => setTheme(value)}
              >
                {t(`settings.${value}`)}
              </Button>
            ))}
          </div>
        </SettingRow>
        <SettingRow label={t("settings.language")}>
          <div className="flex gap-2">
            {LANGUAGES.map((lang) => (
              <Button
                key={lang.code}
                size="sm"
                variant={i18n.language === lang.code ? "default" : "outline"}
                onClick={() => i18n.changeLanguage(lang.code)}
              >
                {lang.label}
              </Button>
            ))}
          </div>
        </SettingRow>
      </SettingsCard>

      <SettingsCard title="Item Editor">
        <TextSetting
          def={{
            key: "item_vnum_range_start",
            label: "VNUM-Start für neue Items",
            hint: "Der Item Editor schlägt ab dieser vnum die nächste freie Nummer vor.",
            defaultValue: "500000",
            validate: (v) => (/^\d+$/.test(v) ? null : "Bitte nur eine ganze Zahl eingeben."),
            className: "w-40",
          }}
        />
      </SettingsCard>
    </div>
  );
}
