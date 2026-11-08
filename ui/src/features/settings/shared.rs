//! Bausteine der Einstellungs-Seiten: Karte, Zeile, Text- und Pfad-Einstellung.

use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon};
use crate::store::connection_profiles::connection_profiles;
use crate::tauri::{dialog_open_one, invoke, invoke_unit, DialogFilter, OpenDialogOptions};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use wasm_bindgen::JsCast;

pub use crate::features::connection::TestState;

pub async fn save_setting(key: &str, value: &str) -> Result<(), String> {
    invoke_unit("set_setting", &json!({ "key": key, "value": value })).await
}

/// Liest eine Einstellung; `Err` nur, wenn der Aufruf selbst scheitert.
pub async fn load_setting(key: &str) -> Result<Option<String>, String> {
    invoke::<Option<String>>("get_setting", &json!({ "key": key })).await
}

/// Speichert und meldet Erfolg/Fehler sichtbar statt still zu scheitern.
pub async fn save_setting_with_feedback(key: &str, value: &str, label: &str) -> bool {
    match save_setting(key, value).await {
        Ok(()) => {
            toast::success(&format!("{label} gespeichert"));
            true
        }
        Err(e) => {
            toast::error(&format!("{label} konnte nicht gespeichert werden: {e}"));
            false
        }
    }
}

#[component]
pub fn ConnStatusIcon(#[prop(into)] state: Signal<TestState>) -> impl IntoView {
    move || match state.get() {
        TestState::Ok => view! { <Icon name="CheckCircle2" class="size-4 text-green-600"/> }.into_any(),
        TestState::Error => view! { <Icon name="XCircle" class="size-4 text-destructive"/> }.into_any(),
        _ => ().into_any(),
    }
}

/// Eine thematische Karte innerhalb einer Einstellungs-Seite.
#[component]
pub fn SettingsCard(
    #[prop(into)] title: Signal<String>,
    #[prop(into, optional)] description: MaybeProp<String>,
    #[prop(optional, into)] icon: Option<&'static str>,
    #[prop(optional)] actions: Option<ChildrenFn>,
    #[prop(into, optional)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <section class=format!("overflow-hidden rounded-xl border border-border bg-card shadow-sm {}", class.unwrap_or_default())>
            <div class="flex flex-wrap items-center justify-between gap-3 border-b border-border bg-muted/50 px-5 py-3.5">
                <div class="flex min-w-0 items-center gap-3">
                    {icon.map(|i| view! {
                        <div class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                            <Icon name=i class="size-[1.125rem]"/>
                        </div>
                    })}
                    <div class="min-w-0">
                        <h3 class="text-base font-semibold leading-tight">{move || title.get()}</h3>
                        {move || description.get().filter(|d| !d.is_empty()).map(|d| view! {
                            <p class="mt-0.5 text-sm text-muted-foreground">{d}</p>
                        })}
                    </div>
                </div>
                {actions.map(|a| a())}
            </div>
            <div class="divide-y divide-border">{children()}</div>
        </section>
    }
}

/// Eine Zeile innerhalb einer SettingsCard. Ab mittlerer Breite steht
/// Beschriftung + Erklärung links und die Eingabe rechts daneben.
/// `hint` ist fest verdrahtetes HTML (für `<code>`-Auszeichnungen).
#[component]
pub fn SettingRow(
    #[prop(into)] label: Signal<String>,
    #[prop(optional_no_strip)] hint: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="grid gap-x-8 gap-y-2 px-5 py-4 md:grid-cols-[minmax(12rem,20rem)_1fr] md:items-start">
            <div class="space-y-1">
                <div class="text-sm font-medium">{move || label.get()}</div>
                {hint.map(|h| view! { <p class="text-sm leading-snug text-muted-foreground" inner_html=h></p> })}
            </div>
            <div class="min-w-0 space-y-2">{children()}</div>
        </div>
    }
}

#[derive(Clone, Copy)]
pub struct TextSettingDef {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: Option<&'static str>,
    pub default_value: &'static str,
    pub placeholder: Option<&'static str>,
    /// Leere Eingabe erlaubt (z.B. "alle Prozesse"). Sonst wird leer ignoriert.
    pub allow_empty: bool,
    pub validate: Option<fn(&str) -> Option<String>>,
    pub class: &'static str,
}

impl TextSettingDef {
    pub const fn new(key: &'static str, label: &'static str, default_value: &'static str) -> Self {
        Self { key, label, hint: None, default_value, placeholder: None, allow_empty: false, validate: None, class: "" }
    }
    pub const fn hint(mut self, h: &'static str) -> Self {
        self.hint = Some(h);
        self
    }
}

/// Ergebnis von [`commit_decision`]: was beim Verlassen des Feldes passiert.
#[derive(Debug, PartialEq)]
pub enum Commit {
    /// Nichts geändert.
    Unchanged,
    /// Leer, aber nicht erlaubt: alten Wert wiederherstellen.
    Revert,
    /// Validierung schlug fehl.
    Invalid(String),
    /// Speichern mit dem (getrimmten) Wert.
    Save(String),
}

pub fn commit_decision(def: &TextSettingDef, value: &str, stored: &str) -> Commit {
    let trimmed = value.trim();
    if trimmed == stored {
        return Commit::Unchanged;
    }
    if trimmed.is_empty() && !def.allow_empty {
        return Commit::Revert;
    }
    if let Some(problem) = def.validate.and_then(|v| v(trimmed)) {
        return Commit::Invalid(problem);
    }
    Commit::Save(trimmed.to_string())
}

/// Textfeld, das beim Verlassen speichert (nur wenn sich etwas geändert hat).
#[component]
pub fn TextSetting(def: TextSettingDef) -> impl IntoView {
    let value = RwSignal::new(def.default_value.to_string());
    let stored = RwSignal::new(def.default_value.to_string());
    let error = RwSignal::new(None::<String>);

    // Server-Pfade gehören zum aktiven Profil -> nach einem Wechsel neu laden.
    let revision = connection_profiles().revision;
    Effect::new(move |_| {
        revision.track();
        spawn_local(async move {
            if let Ok(v) = load_setting(def.key).await {
                let initial = v.unwrap_or_else(|| def.default_value.to_string());
                value.set(initial.clone());
                stored.set(initial);
                error.set(None);
            }
        });
    });

    let commit = move || {
        let cur_stored = stored.get_untracked();
        match commit_decision(&def, &value.get_untracked(), &cur_stored) {
            Commit::Unchanged => {}
            Commit::Revert => value.set(cur_stored),
            Commit::Invalid(problem) => error.set(Some(problem)),
            Commit::Save(trimmed) => {
                error.set(None);
                spawn_local(async move {
                    if save_setting_with_feedback(def.key, &trimmed, def.label).await {
                        stored.set(trimmed);
                    }
                });
            }
        }
    };

    view! {
        <SettingRow label=def.label.to_string() hint=def.hint>
            <input
                prop:value=move || value.get()
                on:input=move |e| value.set(event_target_value(&e))
                on:blur=move |_| commit()
                on:keydown=move |e| {
                    if e.key() == "Enter" {
                        if let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) {
                            let _ = el.blur();
                        }
                    }
                }
                placeholder=def.placeholder
                // Wie tailwind-merge im alten `cn()`: eine eigene Breite ersetzt `w-full`.
                class=move || format!(
                    "{} max-w-2xl rounded-md border bg-background px-3 py-2 font-mono text-sm {} {}",
                    if def.class.split_whitespace().any(|c| c.starts_with("w-")) { "" } else { "w-full" },
                    if error.with(|e| e.is_some()) { "border-destructive" } else { "border-border" },
                    def.class,
                )
            />
            {move || error.get().map(|e| view! { <p class="text-xs text-destructive">{e}</p> })}
        </SettingRow>
    }
}

#[derive(Clone)]
pub struct PathSettingDef {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: Option<&'static str>,
    pub directory: bool,
    pub dialog_title: &'static str,
    pub filters: Vec<DialogFilter>,
    pub empty_text: Option<&'static str>,
    pub clearable: bool,
}

/// Lokaler Datei-/Ordnerpfad mit Auswahldialog.
#[component]
pub fn PathSetting(def: PathSettingDef) -> impl IntoView {
    let value = RwSignal::new(String::new());
    let key = def.key;
    let label = def.label;

    spawn_local(async move {
        if let Ok(v) = load_setting(key).await {
            value.set(v.unwrap_or_default());
        }
    });

    let opts = OpenDialogOptions {
        title: Some(def.dialog_title.to_string()),
        directory: def.directory,
        multiple: false,
        filters: def.filters.clone(),
        ..Default::default()
    };
    let pick = move |_| {
        let opts = opts.clone();
        spawn_local(async move {
            if let Ok(Some(selected)) = dialog_open_one(&opts).await {
                if save_setting_with_feedback(key, &selected, label).await {
                    value.set(selected);
                }
            }
        });
    };
    let clear = move |_| {
        spawn_local(async move {
            if save_setting_with_feedback(key, "", label).await {
                value.set(String::new());
            }
        });
    };
    let empty_text = def.empty_text.unwrap_or("Nicht gesetzt");
    let clearable = def.clearable;

    view! {
        <SettingRow label=label.to_string() hint=def.hint>
            <div class="flex max-w-2xl items-center gap-2">
                <div
                    class=move || {
                        if value.with(|v| v.is_empty()) {
                            "min-w-0 flex-1 truncate rounded-md border border-border bg-muted/40 px-3 py-2 font-mono text-sm font-sans text-muted-foreground"
                        } else {
                            "min-w-0 flex-1 truncate rounded-md border border-border bg-muted/40 px-3 py-2 font-mono text-sm"
                        }
                    }
                    title=move || value.get()
                >
                    {move || {
                        let v = value.get();
                        if v.is_empty() { empty_text.to_string() } else { v }
                    }}
                </div>
                <Show when=move || clearable && value.with(|v| !v.is_empty())>
                    <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm on_click=clear title="Zurücksetzen">
                        <Icon name="X" class="size-3.5"/>
                    </Button>
                </Show>
                <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=pick class="shrink-0">
                    <Icon name="FolderOpen" class="size-4"/>
                    "Auswählen"
                </Button>
            </div>
        </SettingRow>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digits(v: &str) -> Option<String> {
        if !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()) {
            None
        } else {
            Some("Bitte nur eine ganze Zahl eingeben.".into())
        }
    }

    #[test]
    fn commit_rules() {
        let mut def = TextSettingDef::new("k", "L", "d");
        assert_eq!(commit_decision(&def, " a ", "a"), Commit::Unchanged);
        assert_eq!(commit_decision(&def, "  ", "a"), Commit::Revert);
        assert_eq!(commit_decision(&def, " b ", "a"), Commit::Save("b".into()));
        def.allow_empty = true;
        assert_eq!(commit_decision(&def, "", "a"), Commit::Save(String::new()));
        def.validate = Some(digits);
        assert_eq!(commit_decision(&def, "12x", "1"), Commit::Invalid("Bitte nur eine ganze Zahl eingeben.".into()));
        assert_eq!(commit_decision(&def, "12", "1"), Commit::Save("12".into()));
    }
}
