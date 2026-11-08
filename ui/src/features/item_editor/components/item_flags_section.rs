use super::shared::{Field, FlagSelect, NumInput};
use crate::components::Icon;
use crate::features::item_editor::item_flags::{
    value_hints, value_labels_by_type, weapon_attack_speed_label, weapon_display_damage,
    weapon_effective_attack_speed, FlagOption, ANTI_FLAGS, APPLY_TYPES, IMMUNE_FLAGS, ITEM_FLAGS, LIMIT_TYPES,
    WEAPON_SUBTYPE_TWO_HANDED, WEAR_FLAGS,
};
use crate::features::item_editor::types::{num, ItemProtoInput};
use leptos::prelude::*;

#[component]
fn FlagGroup(
    #[prop(into)] title: Signal<String>,
    options: &'static [FlagOption],
    #[prop(into)] value: Signal<i64>,
    #[prop(into)] on_toggle: Callback<i64>,
    #[prop(into)] on_raw: Callback<i64>,
) -> impl IntoView {
    view! {
        <div class="space-y-1">
            <div class="flex items-center gap-2">
                <p class="text-xs font-medium text-muted-foreground">{move || title.get()}</p>
                <input
                    type="number"
                    prop:value=move || value.get().to_string()
                    on:input=move |e| on_raw.run(num(&event_target_value(&e)))
                    class="w-24 rounded-md border border-border bg-background px-1.5 py-0.5 text-xs"
                />
            </div>
            <div class="flex flex-wrap gap-2">
                {options.iter().map(|opt| {
                    let bit = opt.value;
                    view! {
                        <label class="flex items-center gap-1 text-xs">
                            <input
                                type="checkbox"
                                prop:checked=move || value.get() & bit != 0
                                on:change=move |_| on_toggle.run(bit)
                            />
                            {opt.label}
                        </label>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

const VALUE_KEYS: [&str; 6] = ["value0", "value1", "value2", "value3", "value4", "value5"];
const APPLY_KEYS: [(&str, &str); 4] = [
    ("applytype0", "applyvalue0"),
    ("applytype1", "applyvalue1"),
    ("applytype2", "applyvalue2"),
    ("applytype3", "applyvalue3"),
];

#[component]
pub fn ItemFlagsSection(
    #[prop(into)] item: Signal<ItemProtoInput>,
    /// Setzt ein Zahlenfeld (Spaltenname, Wert).
    #[prop(into)] set: Callback<(&'static str, i64)>,
    /// Schaltet ein Bit in wearflag/antiflag/immuneflag/flag um.
    #[prop(into)] toggle_flag: Callback<(&'static str, i64)>,
) -> impl IntoView {
    let advanced_open = RwSignal::new(false);
    let field = move |k: &'static str| Signal::derive(move || item.with(|i| i.num(k)));
    let setter = move |k: &'static str| Callback::new(move |v: i64| set.run((k, v)));
    let item_type = Memo::new(move |_| item.with(|i| i.r#type));

    let flag_group = move |key: &'static str, title: &'static str, options: &'static [FlagOption]| {
        let value = field(key);
        view! {
            <FlagGroup
                title=Signal::derive(move || format!("{title} ({key} = {})", value.get()))
                options=options
                value=value
                on_toggle=Callback::new(move |bit| toggle_flag.run((key, bit)))
                on_raw=setter(key)
            />
        }
    };

    view! {
        <section class="space-y-3 rounded-lg border border-border p-4">
            <button class="text-sm font-medium text-muted-foreground" on:click=move |_| advanced_open.update(|v| *v = !*v)>
                "Flags & Werte (erweitert) " {move || if advanced_open.get() { "▲" } else { "▼" }}
            </button>
            <Show when=move || advanced_open.get()>
                <div class="space-y-4">
                    <p class="text-xs text-muted-foreground">
                        "Bits aus dem echten Server-Quellcode verifiziert (length.h/item_length.h). Rohwerte (Hex/Dezimal) bleiben trotzdem immer verfügbar."
                    </p>

                    {flag_group("wearflag", "Trageort", WEAR_FLAGS)}
                    {flag_group("antiflag", "Klassen-/Geschlechtssperre", ANTI_FLAGS)}
                    {flag_group("immuneflag", "Immunität", IMMUNE_FLAGS)}
                    {flag_group("flag", "Item-Eigenschaften", ITEM_FLAGS)}

                    <div class="flex flex-wrap gap-3">
                        <Field label="Limit-Typ 0">
                            <FlagSelect
                                options=LIMIT_TYPES
                                leading=(0, "—")
                                value=field("limittype0")
                                on_change=setter("limittype0")
                                class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                        </Field>
                        <Field label="Wert">
                            <NumInput
                                value=field("limitvalue0")
                                on_change=setter("limitvalue0")
                                class="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                        </Field>
                    </div>

                    <p class="text-xs text-muted-foreground">
                        {move || value_hints(item_type.get()).unwrap_or(
                            "value0-5 (Effektwerte) hängen vom Typ/Subtyp ab und wurden für diesen Typ noch nicht im Quellcode nachgeschlagen.",
                        )}
                        " Am zuverlässigsten über \"Referenz-Item übernehmen\" oben befüllen und dann anpassen."
                    </p>
                    <div class="grid grid-cols-3 gap-2 sm:grid-cols-6">
                        {VALUE_KEYS.iter().enumerate().map(|(i, k)| {
                            let k = *k;
                            let label = Signal::derive(move || {
                                value_labels_by_type(item_type.get()).map(|l| l[i].to_string()).unwrap_or_else(|| k.to_string())
                            });
                            view! {
                                <Field label=label>
                                    <NumInput
                                        value=field(k)
                                        on_change=setter(k)
                                        class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    />
                                </Field>
                            }
                        }).collect_view()}
                    </div>

                    {move || item.with(|it| {
                        if it.r#type == 1 {
                            let d = weapon_display_damage(it.value3, it.value4, it.value5);
                            let effective_speed = weapon_effective_attack_speed(it.value0, it.subtype);
                            let damage = if d.max > d.min { format!("{} ~ {}", d.min, d.max) } else { d.min.to_string() };
                            let two_handed = (it.subtype == WEAPON_SUBTYPE_TWO_HANDED)
                                .then(|| format!(" ({} - 10 Zweihand-Malus = {})", it.value0, effective_speed));
                            let magic = (it.value1 > 0 || it.value2 > 0)
                                .then(|| format!(" · Magie-Angriff: {} ~ {}", it.value1, it.value2));
                            Some(view! {
                                <div class="rounded-md border border-border bg-muted/40 p-2 text-xs">
                                    <p class="font-medium">"Anzeige im Client (Tooltip-Vorschau):"</p>
                                    <p>
                                        "Angriffskraft: " {damage} " · Angriffstempo: "
                                        {weapon_attack_speed_label(effective_speed)}
                                        {two_handed}
                                        {magic}
                                    </p>
                                </div>
                            }.into_any())
                        } else if it.r#type == 2 {
                            let magic_def = (it.value0 > 0).then(|| format!(" · Magie-Verteidigung: {}", it.value0));
                            Some(view! {
                                <div class="rounded-md border border-border bg-muted/40 p-2 text-xs">
                                    <p class="font-medium">"Anzeige im Client (Tooltip-Vorschau):"</p>
                                    <p>"Verteidigung: " {it.value1 + it.value5 * 2} {magic_def}</p>
                                </div>
                            }.into_any())
                        } else if it.r#type == 3 && it.subtype == 2 {
                            let v0 = it.value0;
                            Some(view! {
                                <div class="space-y-1 rounded-md border border-border bg-muted/40 p-2 text-xs">
                                    <p class="font-medium">"Aufwertungs-Schriftrolle (USE_TUNING):"</p>
                                    <p>
                                        "value0 = Schriftrollen-ID — 0-6 sind fest im Server-Quellcode belegt (Chukbok, Hyuniron, Yongsin, Musin, Yagong, Memo, B-Dragon). Für eine neue, generische Schriftrolle einen Wert ≥ 7 verwenden."
                                    </p>
                                    {(0..=6).contains(&v0).then(|| view! {
                                        <p class="flex items-center gap-1 text-amber-600 dark:text-amber-400">
                                            <Icon name="AlertTriangle" class="size-3.5 shrink-0"/>
                                            " value0 = " {v0}
                                            " kollidiert mit einer fest verdrahteten Alt-Schriftrolle — für eine neue generische Schriftrolle mindestens 7 verwenden."
                                        </p>
                                    })}
                                    <p>
                                        "value2 = Erfolgschance in % (0-100), value3 = Verhalten bei Fehlschlag (0 = Item wird abgestuft falls möglich, 1 = Item bleibt bei Fehlschlag erhalten). Erfordert den char_item.cpp-Patch für generische Boost-Schriftrollen (bereits auf dem Server eingespielt, wirkt erst nach dem nächsten Bauen & Einspielen der Server-Software)."
                                    </p>
                                </div>
                            }.into_any())
                        } else {
                            None
                        }
                    })}

                    <p class="text-xs text-muted-foreground">
                        "Stat-Boni (applytype/applyvalue) — Typen aus dem echten Server-Quellcode verifiziert ("
                        <code>"enum EApplyTypes"</code>
                        ")."
                    </p>
                    <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
                        {APPLY_KEYS.iter().map(|(t_key, v_key)| {
                            let (t_key, v_key) = (*t_key, *v_key);
                            view! {
                                <div class="flex items-end gap-2">
                                    <Field label="Typ">
                                        <FlagSelect
                                            options=APPLY_TYPES
                                            value=field(t_key)
                                            on_change=setter(t_key)
                                            class="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                        />
                                    </Field>
                                    <Field label="Wert">
                                        <NumInput
                                            value=field(v_key)
                                            on_change=setter(v_key)
                                            class="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                        />
                                    </Field>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </div>
            </Show>
        </section>
    }
}
