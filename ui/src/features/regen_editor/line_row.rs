//! Eine Zeile der Regen-Datei als Formular (Tabellenansicht und
//! Auswahl-Panel der Karte).

use super::model::{LineKind, NumKey, RegenLine, SPAWN_TYPES};
use super::num_input::NumInput;
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use leptos::prelude::*;

#[component]
fn NumField(
    label: &'static str,
    line: Signal<RegenLine>,
    key: NumKey,
    on_change: Callback<RegenLine>,
    #[prop(optional)] width: Option<&'static str>,
) -> impl IntoView {
    let width = width.unwrap_or("w-16");
    view! {
        <label class="flex flex-col gap-0.5 text-[10px] text-muted-foreground">
            {label}
            <NumInput
                value=Signal::derive(move || line.with(|l| l.num(key)))
                on_change=move |v| {
                    let mut l = line.get_untracked();
                    l.set_num(key, v);
                    on_change.run(l);
                }
                class=format!("{width} rounded-md border border-border bg-background px-1.5 py-1 text-sm")
            />
        </label>
    }
}

#[component]
pub fn RegenLineRow(
    #[prop(into)] line: Signal<RegenLine>,
    #[prop(into)] on_change: Callback<RegenLine>,
    #[prop(into)] on_remove: Callback<()>,
) -> impl IntoView {
    let kind = Memo::new(move |_| line.with(|l| l.kind()));

    let remove_button = move |extra: &'static str| {
        view! {
            <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm class=extra on_click=move |_| on_remove.run(())>
                <Icon name="Trash2" class="size-3.5"/>
            </Button>
        }
    };

    move || match kind.get() {
        LineKind::Raw => view! {
            <div class="flex items-center gap-2 rounded-md border border-dashed border-border px-2 py-1 text-xs text-muted-foreground">
                <span class="flex-1 truncate font-mono">
                    {move || line.with(|l| match l {
                        RegenLine::Raw { text } if !text.is_empty() => text.clone(),
                        _ => "(leere Zeile)".to_string(),
                    })}
                </span>
                {remove_button("")}
            </div>
        }
        .into_any(),
        LineKind::Exception => view! {
            <div class="flex flex-wrap items-end gap-2 rounded-md border border-border p-2">
                <span class="mb-1 text-xs font-medium">"Ausnahmezone"</span>
                <NumField label="X" line=line key=NumKey::CenterX on_change=on_change/>
                <NumField label="Y" line=line key=NumKey::CenterY on_change=on_change/>
                <NumField label="Radius X" line=line key=NumKey::RadiusX on_change=on_change/>
                <NumField label="Radius Y" line=line key=NumKey::RadiusY on_change=on_change/>
                <NumField label="Z-Sektion" line=line key=NumKey::ZSection on_change=on_change/>
                {remove_button("ml-auto")}
            </div>
        }
        .into_any(),
        LineKind::Spawn => {
            let spawn_type = Memo::new(move |_| line.with(|l| l.spawn_type().to_string()));
            let update = move |f: &dyn Fn(&mut RegenLine)| {
                let mut l = line.get_untracked();
                f(&mut l);
                on_change.run(l);
            };
            view! {
                <div class="flex flex-wrap items-end gap-2 rounded-md border border-border p-2">
                    <label class="flex flex-col gap-0.5 text-[10px] text-muted-foreground">
                        "Typ"
                        <select
                            on:change=move |e| {
                                let v = event_target_value(&e);
                                update(&|l| {
                                    if let RegenLine::Spawn(s) = l {
                                        s.spawn_type = v.clone();
                                    }
                                });
                            }
                            class="w-36 rounded-md border border-border bg-background px-1.5 py-1 text-sm"
                        >
                            {SPAWN_TYPES
                                .iter()
                                .map(|t| {
                                    let value = t.value;
                                    view! {
                                        <option value=value selected=move || spawn_type.with(|s| s == value)>
                                            {t.label}
                                        </option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    </label>
                    <NumField label="X" line=line key=NumKey::CenterX on_change=on_change/>
                    <NumField label="Y" line=line key=NumKey::CenterY on_change=on_change/>
                    <NumField label="Radius X" line=line key=NumKey::RadiusX on_change=on_change/>
                    <NumField label="Radius Y" line=line key=NumKey::RadiusY on_change=on_change/>
                    <NumField label="Z-Sektion" line=line key=NumKey::ZSection on_change=on_change/>
                    <NumField label="Richtung" line=line key=NumKey::Direction on_change=on_change/>
                    <label class="flex flex-col gap-0.5 text-[10px] text-muted-foreground">
                        "Regen-Zeit"
                        <input
                            prop:value=move || line.with(|l| match l {
                                RegenLine::Spawn(s) => s.regen_time.clone(),
                                _ => String::new(),
                            })
                            on:input=move |e| {
                                let v = event_target_value(&e);
                                update(&|l| {
                                    if let RegenLine::Spawn(s) = l {
                                        s.regen_time = v.clone();
                                    }
                                });
                            }
                            placeholder="z.B. 1h30m"
                            class="w-20 rounded-md border border-border bg-background px-1.5 py-1 text-sm"
                        />
                    </label>
                    <NumField label="Max. Anzahl" line=line key=NumKey::MaxCount on_change=on_change/>
                    <NumField label="Mob-VNUM" line=line key=NumKey::Vnum on_change=on_change width="w-20"/>
                    {remove_button("ml-auto")}
                </div>
            }
            .into_any()
        }
    }
}
