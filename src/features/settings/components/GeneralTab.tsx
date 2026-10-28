import { useTranslation } from "react-i18next";
import { useThemeStore, UI_SCALES, type Theme } from "@/store/theme";
import { Button } from "@/components/ui/button";
import { Moon, Package, Palette, Sun, SunMoon } from "lucide-react";
import { SettingRow, SettingsCard, TextSetting } from "./shared";

const THEMES: { value: Theme; icon: typeof Sun }[] = [
  { value: "light", icon: Sun },
  { value: "dark", icon: Moon },
  { value: "system", icon: SunMoon },
];
const LANGUAGES = [
  { code: "de", label: "Deutsch" },
  { code: "en", label: "English" },
];

export function GeneralTab() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme, scale, setScale } = useThemeStore();

  return (
    <div className="space-y-4">
      <SettingsCard title="Darstellung" icon={Palette}>
        <SettingRow label={t("settings.theme")}>
          <div className="flex flex-wrap gap-2">
            {THEMES.map(({ value, icon: Icon }) => (
              <Button
                key={value}
                variant={theme === value ? "default" : "outline"}
                onClick={() => setTheme(value)}
              >
                <Icon className="size-4" />
                {t(`settings.${value}`)}
              </Button>
            ))}
          </div>
        </SettingRow>
        <SettingRow label={t("settings.language")}>
          <div className="flex flex-wrap gap-2">
            {LANGUAGES.map((lang) => (
              <Button
                key={lang.code}
                variant={i18n.language === lang.code ? "default" : "outline"}
                onClick={() => i18n.changeLanguage(lang.code)}
              >
                {lang.label}
              </Button>
            ))}
          </div>
        </SettingRow>
        <SettingRow
          label="Größe der Oberfläche"
          hint={
            <>
              Vergrößert Schrift, Knöpfe und Abstände in der ganzen App. Geht auch jederzeit mit{" "}
              <kbd className="rounded border border-border bg-muted px-1 text-xs">Strg</kbd> +{" "}
              <kbd className="rounded border border-border bg-muted px-1 text-xs">+</kbd> /{" "}
              <kbd className="rounded border border-border bg-muted px-1 text-xs">-</kbd>, zurück mit{" "}
              <kbd className="rounded border border-border bg-muted px-1 text-xs">Strg</kbd> +{" "}
              <kbd className="rounded border border-border bg-muted px-1 text-xs">0</kbd>.
            </>
          }
        >
          <div className="flex flex-wrap gap-2">
            {UI_SCALES.map((value) => (
              <Button
                key={value}
                variant={scale === value ? "default" : "outline"}
                onClick={() => setScale(value)}
                className="min-w-16 tabular-nums"
              >
                {value} %
              </Button>
            ))}
          </div>
        </SettingRow>
      </SettingsCard>

      <SettingsCard title="Item Editor" icon={Package}>
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
