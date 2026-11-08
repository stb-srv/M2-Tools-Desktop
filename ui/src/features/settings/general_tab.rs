use super::shared::{SettingRow, SettingsCard, TextSetting, TextSettingDef};
use crate::components::{Button, ButtonVariant, Icon};
use crate::i18n::{language, set_language, t};
use crate::store::theme::{theme_store, Theme, UI_SCALES};
use leptos::prelude::*;

const THEMES: [(Theme, &str); 3] = [(Theme::Light, "Sun"), (Theme::Dark, "Moon"), (Theme::System, "SunMoon")];
const LANGUAGES: [(&str, &str); 2] = [("de", "Deutsch"), ("en", "English")];

/// Gültig, wenn nur Ziffern (mindestens eine).
pub fn validate_vnum_start(v: &str) -> Option<String> {
    if !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()) {
        None
    } else {
        Some("Bitte nur eine ganze Zahl eingeben.".into())
    }
}

const SCALE_HINT: &str = "Vergrößert Schrift, Knöpfe und Abstände in der ganzen App. Geht auch jederzeit mit \
<kbd class=\"rounded border border-border bg-muted px-1 text-xs\">Strg</kbd> + \
<kbd class=\"rounded border border-border bg-muted px-1 text-xs\">+</kbd> / \
<kbd class=\"rounded border border-border bg-muted px-1 text-xs\">-</kbd>, zurück mit \
<kbd class=\"rounded border border-border bg-muted px-1 text-xs\">Strg</kbd> + \
<kbd class=\"rounded border border-border bg-muted px-1 text-xs\">0</kbd>.";

fn variant(active: bool) -> ButtonVariant {
    if active {
        ButtonVariant::Default
    } else {
        ButtonVariant::Outline
    }
}

#[component]
pub fn GeneralTab() -> impl IntoView {
    let st = theme_store();
    view! {
        <div class="space-y-4">
            <SettingsCard title="Darstellung".to_string() icon="Palette">
                <SettingRow label=Signal::derive(|| t("settings.theme"))>
                    <div class="flex flex-wrap gap-2">
                        {move || {
                            let current = st.theme.get();
                            THEMES
                                .iter()
                                .map(|(value, icon)| {
                                    let value = *value;
                                    view! {
                                        <Button variant=variant(current == value) on_click=move |_| st.set_theme(value)>
                                            <Icon name=*icon class="size-4"/>
                                            {move || t(&format!("settings.{}", value.id()))}
                                        </Button>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </SettingRow>
                <SettingRow label=Signal::derive(|| t("settings.language"))>
                    <div class="flex flex-wrap gap-2">
                        {move || {
                            let current = language();
                            LANGUAGES
                                .iter()
                                .map(|(code, label)| {
                                    let code = *code;
                                    view! {
                                        <Button variant=variant(current == code) on_click=move |_| set_language(code)>
                                            {*label}
                                        </Button>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </SettingRow>
                <SettingRow label="Größe der Oberfläche".to_string() hint=Some(SCALE_HINT)>
                    <div class="flex flex-wrap gap-2">
                        {move || {
                            let current = st.scale.get();
                            UI_SCALES
                                .iter()
                                .map(|value| {
                                    let value = *value;
                                    view! {
                                        <Button
                                            variant=variant(current == value)
                                            on_click=move |_| st.set_scale(value)
                                            class="min-w-16 tabular-nums"
                                        >
                                            {format!("{value} %")}
                                        </Button>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </SettingRow>
            </SettingsCard>

            <SettingsCard title="Item Editor".to_string() icon="Package">
                <TextSetting def=TextSettingDef {
                    hint: Some("Der Item Editor schlägt ab dieser vnum die nächste freie Nummer vor."),
                    validate: Some(validate_vnum_start),
                    class: "w-40",
                    ..TextSettingDef::new("item_vnum_range_start", "VNUM-Start für neue Items", "500000")
                }/>
            </SettingsCard>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::validate_vnum_start;

    #[test]
    fn vnum_start() {
        assert_eq!(validate_vnum_start("500000"), None);
        assert!(validate_vnum_start("").is_some());
        assert!(validate_vnum_start("-1").is_some());
        assert!(validate_vnum_start("1.5").is_some());
    }
}
