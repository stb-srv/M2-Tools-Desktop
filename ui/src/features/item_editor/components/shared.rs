use crate::components::Icon;
use crate::features::item_editor::item_flags::FlagOption;
use crate::features::item_editor::types::{num, StepStatus};
use leptos::prelude::*;

/// Beschriftetes Eingabefeld (Label über dem Inhalt).
#[component]
pub fn Field(#[prop(into)] label: Signal<String>, children: Children) -> impl IntoView {
    view! {
        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
            {move || label.get()}
            {children()}
        </label>
    }
}

/// Kontrolliertes `<input type="number">` - Eingaben werden wie
/// `Number(v)` (nicht-endlich/leer = 0) gelesen.
#[component]
pub fn NumInput(
    #[prop(into)] value: Signal<i64>,
    #[prop(into)] on_change: Callback<i64>,
    #[prop(into)] class: String,
    #[prop(optional, into)] min: Option<&'static str>,
    #[prop(optional, into)] max: Option<&'static str>,
) -> impl IntoView {
    view! {
        <input
            type="number"
            min=min
            max=max
            prop:value=move || value.get().to_string()
            on:input=move |e| on_change.run(num(&event_target_value(&e)))
            class=class
        />
    }
}

/// `<select>` über eine Optionsliste, Beschriftung "Label (Wert)".
#[component]
pub fn FlagSelect(
    options: &'static [FlagOption],
    #[prop(into)] value: Signal<i64>,
    #[prop(into)] on_change: Callback<i64>,
    #[prop(into)] class: String,
    /// Zusätzliche erste Option (Wert, Beschriftung), z.B. `(0, "—")`.
    #[prop(optional)] leading: Option<(i64, &'static str)>,
    /// `false` = nur "Label" statt "Label (Wert)".
    #[prop(optional)] plain_labels: bool,
) -> impl IntoView {
    view! {
        <select on:change=move |e| on_change.run(num(&event_target_value(&e))) class=class>
            {leading.map(|(v, l)| view! { <option value=v.to_string() prop:selected=move || value.get() == v>{l}</option> })}
            {options.iter().map(|t| {
                let v = t.value;
                let label = if plain_labels { t.label.to_string() } else { format!("{} ({})", t.label, t.value) };
                view! { <option value=v.to_string() prop:selected=move || value.get() == v>{label}</option> }
            }).collect_view()}
        </select>
    }
}

/// Eine Zeile der Schritt-Anzeige einer Pipeline. `spacer_class` ist der
/// Platzhalter für "noch nicht gestartet" (Item Editor: `size-4`,
/// Modul-Importer: `size-4 shrink-0`).
#[component]
pub fn StepRow(
    #[prop(into)] label: Signal<String>,
    #[prop(into)] status: Signal<Option<StepStatus>>,
    #[prop(optional)] spacer_class: Option<&'static str>,
) -> impl IntoView {
    let spacer = spacer_class.unwrap_or("size-4");
    view! {
        <div class="flex items-center gap-2">
            {move || match status.get() {
                Some(StepStatus::Done) => view! { <Icon name="CheckCircle2" class="size-4 text-green-600"/> }.into_any(),
                Some(StepStatus::Error) => view! { <Icon name="AlertTriangle" class="size-4 text-destructive"/> }.into_any(),
                Some(StepStatus::Running) => view! {
                    <span class="size-4 animate-spin rounded-full border-2 border-muted-foreground border-t-transparent"></span>
                }
                .into_any(),
                None | Some(StepStatus::Pending) => view! { <span class=spacer></span> }.into_any(),
            }}
            <span class=move || if status.get() == Some(StepStatus::Error) { "text-destructive" } else { "" }>
                {move || label.get()}
            </span>
        </div>
    }
}
