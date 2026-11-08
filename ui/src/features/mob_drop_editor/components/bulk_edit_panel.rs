use super::super::types::{BulkMode, BulkScope};
use super::shared::{js_number, js_number_or, num_sig, Field, NumInput, MAX_PERCENT_ATTR};
use crate::components::{Button, ButtonVariant};
use crate::features::shared::drop_chance::format_real_drop_chance;
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BulkEditParams {
    pub scope: BulkScope,
    pub mode: BulkMode,
    pub delta: f64,
    pub fixed: f64,
    pub random_min: f64,
    pub random_max: f64,
    pub specific_vnum: f64,
    pub specific_percent: f64,
}

const MODES: [(BulkMode, &str); 4] = [
    (BulkMode::Delta, "Addieren/Subtrahieren"),
    (BulkMode::Fixed, "Fester Wert"),
    (BulkMode::Random, "Zufall in Bereich"),
    (BulkMode::SpecificItem, "Bestimmtes Item überall"),
];

fn seg_class(active: bool) -> &'static str {
    if active {
        "px-3 py-1 bg-primary text-primary-foreground"
    } else {
        "px-3 py-1 "
    }
}

#[component]
pub fn BulkEditPanel(
    #[prop(into)] has_groups: Signal<bool>,
    #[prop(into)] has_selection: Signal<bool>,
    #[prop(into)] selected_name: Signal<Option<String>>,
    #[prop(into)] on_apply: Callback<BulkEditParams>,
) -> impl IntoView {
    let scope = RwSignal::new(BulkScope::Current);
    let mode = RwSignal::new(BulkMode::Delta);
    let delta = RwSignal::new(0.0);
    let fixed = RwSignal::new(0.0);
    let random_min = RwSignal::new(0.0);
    let random_max = RwSignal::new(30.0);
    let specific_vnum = RwSignal::new(String::new());
    let specific_percent = RwSignal::new(0.0);
    let confirming = RwSignal::new(false);

    let confirm_apply = move || {
        on_apply.run(BulkEditParams {
            scope: scope.get_untracked(),
            mode: mode.get_untracked(),
            delta: delta.get_untracked(),
            fixed: fixed.get_untracked(),
            random_min: random_min.get_untracked(),
            random_max: random_max.get_untracked(),
            specific_vnum: js_number(&specific_vnum.get_untracked()),
            specific_percent: specific_percent.get_untracked(),
        });
        confirming.set(false);
    };


    view! {
        <div class="space-y-3 rounded-lg border border-border p-3">
            <h2 class="text-sm font-medium text-muted-foreground">"Prozentwerte in einem Rutsch ändern"</h2>

            <div class="flex overflow-hidden rounded-md border border-border text-sm w-fit">
                <button on:click=move |_| scope.set(BulkScope::Global) class=move || seg_class(scope.get() == BulkScope::Global)>
                    "Global (alle Mobs)"
                </button>
                <button
                    on:click=move |_| scope.set(BulkScope::Current)
                    disabled=move || !has_selection.get()
                    class=move || {
                        if scope.get() == BulkScope::Current {
                            "px-3 py-1 disabled:opacity-40 bg-primary text-primary-foreground"
                        } else {
                            "px-3 py-1 disabled:opacity-40 "
                        }
                    }
                >
                    "Nur dieser Mob"
                </button>
            </div>

            <div class="flex flex-wrap gap-2 text-sm">
                {MODES
                    .iter()
                    .map(|(m, label)| {
                        let m = *m;
                        view! {
                            <button
                                on:click=move |_| mode.set(m)
                                class=move || {
                                    if mode.get() == m {
                                        "rounded-md border border-border px-3 py-1 bg-primary text-primary-foreground"
                                    } else {
                                        "rounded-md border border-border px-3 py-1 "
                                    }
                                }
                            >
                                {*label}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>

            <Show when=move || mode.get() == BulkMode::Delta>
                <Field label="Betrag (z.B. -5 oder 5)">
                    <NumInput
                        value=num_sig(move || delta.get())
                        on_input=move |v: String| delta.set(js_number_or(&v, 0.0))
                        class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
            </Show>
            <Show when=move || mode.get() == BulkMode::Fixed>
                <div class="flex items-end gap-2">
                    <Field label="Neuer Wert für alle">
                        <NumInput
                            min="0"
                            max=MAX_PERCENT_ATTR
                            value=num_sig(move || fixed.get())
                            on_input=move |v: String| fixed.set(js_number_or(&v, 0.0))
                            class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                    <span class="pb-1.5 text-xs text-muted-foreground">
                        {move || format!("≈{} real", format_real_drop_chance(fixed.get()))}
                    </span>
                </div>
            </Show>
            <Show when=move || mode.get() == BulkMode::Random>
                <div class="flex items-end gap-3">
                    <Field label="Min">
                        <NumInput
                            min="0"
                            max=MAX_PERCENT_ATTR
                            value=num_sig(move || random_min.get())
                            on_input=move |v: String| random_min.set(js_number_or(&v, 0.0))
                            class="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                    <Field label="Max">
                        <NumInput
                            min="0"
                            max=MAX_PERCENT_ATTR
                            value=num_sig(move || random_max.get())
                            on_input=move |v: String| random_max.set(js_number_or(&v, 0.0))
                            class="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                    <span class="pb-1.5 text-xs text-muted-foreground">
                        {move || format!(
                            "≈{}–{} real",
                            format_real_drop_chance(random_min.get()),
                            format_real_drop_chance(random_max.get()),
                        )}
                    </span>
                </div>
            </Show>
            <Show when=move || mode.get() == BulkMode::SpecificItem>
                <div class="flex items-end gap-2">
                    <Field label="Item-VNUM">
                        <input
                            type="number"
                            prop:value=move || specific_vnum.get()
                            on:input=move |e| specific_vnum.set(event_target_value(&e))
                            class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                    <Field label="Prozent">
                        <NumInput
                            min="0"
                            max=MAX_PERCENT_ATTR
                            value=num_sig(move || specific_percent.get())
                            on_input=move |v: String| specific_percent.set(js_number_or(&v, 0.0))
                            class="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                    <span class="pb-1.5 text-xs text-muted-foreground">
                        {move || format!("≈{} real", format_real_drop_chance(specific_percent.get()))}
                    </span>
                </div>
            </Show>

            <Button
                variant=ButtonVariant::Outline
                on_click=move |_| confirming.set(true)
                disabled=Signal::derive(move || !has_groups.get() || (scope.get() == BulkScope::Current && !has_selection.get()))
            >
                "Anwenden"
            </Button>
            <p class="text-xs text-muted-foreground">
                "Wirkt zunächst nur auf die geladenen Daten - erst \"Speichern\" oben schreibt sie in die Datei."
            </p>

            <Show when=move || confirming.get()>
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm">
                            {move || {
                                let target = if scope.get() == BulkScope::Global {
                                    "aller Mobs".to_string()
                                } else {
                                    format!("von \"{}\"", selected_name.get().unwrap_or_else(|| "undefined".into()))
                                };
                                format!(
                                    "Prozentwerte {target} jetzt ändern? Das betrifft nur die geladenen Daten, bis du speicherst."
                                )
                            }}
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| confirming.set(false)>
                                "Abbrechen"
                            </Button>
                            <Button on_click=move |_| confirm_apply()>"Anwenden"</Button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
