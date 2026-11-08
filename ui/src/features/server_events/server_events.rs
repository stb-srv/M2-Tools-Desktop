use super::flags::{gm_command, is_curated, FlagDef, RATE_FLAGS, RATE_FLAGS_PREMIUM, SEASONAL_FLAGS, SPECIAL_EVENT_FLAGS};
use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::server_control::util::{copy_to_clipboard, js_number, js_number_json, js_number_string};
use crate::util::log_activity;
use crate::store::navigation::{nav, Section};
use crate::tauri::{invoke0, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EventFlagRow {
    name: String,
    value: i64,
}

#[component]
fn CopyButton(#[prop(into)] text: Signal<String>) -> impl IntoView {
    let copied = RwSignal::new(false);
    view! {
        <Button
            variant=ButtonVariant::Ghost
            size=ButtonSize::IconSm
            title=Signal::derive(move || format!("GM-Befehl kopieren: {}", text.get()))
            on_click=move |_| {
                let t = text.get_untracked();
                spawn_local(async move {
                    copy_to_clipboard(&t).await;
                    copied.set(true);
                    set_timeout(move || { let _ = copied.try_set(false); }, std::time::Duration::from_millis(1500));
                });
            }
        >
            {move || if copied.get() {
                view! { <Icon name="Check" class="size-3.5 text-green-600"/> }.into_any()
            } else {
                view! { <Icon name="Copy" class="size-3.5"/> }.into_any()
            }}
        </Button>
    }
}

#[component]
fn FlagRow(
    def: &'static FlagDef,
    #[prop(into)] current_value: Signal<Option<i64>>,
    #[prop(into)] unset_hint: String,
    on_save: Callback<(String, f64)>,
    on_reset: Callback<String>,
    #[prop(into)] busy: Signal<bool>,
) -> impl IntoView {
    let as_text = move || current_value.get().map(|v| v.to_string()).unwrap_or_default();
    let value = RwSignal::new(current_value.get_untracked().map(|v| v.to_string()).unwrap_or_default());

    Effect::new(move |_| {
        value.set(as_text());
    });

    let is_set = move || current_value.get().is_some();
    let dirty = move || value.get() != as_text();

    view! {
        <div class="flex flex-wrap items-center gap-2 border-t border-border py-2 first:border-t-0">
            <div class="min-w-0 flex-1">
                <p class="text-sm font-medium">
                    {def.label} " " <code class="text-xs text-muted-foreground">{def.name}</code>
                </p>
                <p class="text-xs text-muted-foreground">{def.hint}</p>
                <Show when=move || !is_set()>
                    <p class="text-xs text-muted-foreground italic">{unset_hint.clone()}</p>
                </Show>
            </div>
            <input
                type="number"
                prop:value=move || value.get()
                on:input=move |e| value.set(event_target_value(&e))
                placeholder="—"
                class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <Button
                size=ButtonSize::Sm
                disabled=Signal::derive(move || !dirty() || busy.get() || value.with(|v| v.is_empty()))
                on_click=move |_| on_save.run((def.name.to_string(), js_number(&value.get_untracked())))
            >
                "Speichern"
            </Button>
            <Show when=is_set>
                <Button size=ButtonSize::Sm variant=ButtonVariant::Outline disabled=busy on_click=move |_| on_reset.run(def.name.to_string())>
                    <Icon name="RotateCcw" class="size-3.5"/>
                </Button>
            </Show>
            <CopyButton text=Signal::derive(move || gm_command(def.name, &value.get()))/>
        </div>
    }
}

#[component]
fn FlagSection(#[prop(into)] title: String, default_open: bool, children: ChildrenFn) -> impl IntoView {
    let open = RwSignal::new(default_open);
    view! {
        <section class="rounded-lg border border-border p-4">
            <button class="flex w-full items-center gap-2 text-left" on:click=move |_| open.update(|v| *v = !*v)>
                <Icon name=Signal::derive(move || if open.get() { "ChevronDown" } else { "ChevronRight" }) class="size-4"/>
                <h2 class="text-sm font-medium text-muted-foreground">{title}</h2>
            </button>
            <Show when=move || open.get()>
                <div class="mt-2">{children()}</div>
            </Show>
        </section>
    }
}

#[component]
pub fn ServerEvents() -> impl IntoView {
    let flags = RwSignal::new(None::<Vec<EventFlagRow>>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let busy_name = RwSignal::new(None::<String>);

    let custom_name = RwSignal::new(String::new());
    let custom_value = RwSignal::new(String::new());

    async fn load(flags: RwSignal<Option<Vec<EventFlagRow>>>, loading: RwSignal<bool>, error: RwSignal<Option<String>>) {
        loading.set(true);
        error.set(None);
        match invoke0::<Vec<EventFlagRow>>("list_event_flags").await {
            Ok(f) => flags.set(Some(f)),
            Err(e) => error.set(Some(e)),
        }
        loading.set(false);
    }

    spawn_local(load(flags, loading, error));

    let flag_value = move |name: &'static str| {
        Signal::derive(move || flags.with(|f| f.as_ref().and_then(|f| f.iter().find(|r| r.name == name).map(|r| r.value))))
    };

    async fn save_flag(
        name: String,
        value: f64,
        flags: RwSignal<Option<Vec<EventFlagRow>>>,
        loading: RwSignal<bool>,
        error: RwSignal<Option<String>>,
        busy_name: RwSignal<Option<String>>,
    ) {
        busy_name.set(Some(name.clone()));
        error.set(None);
        match invoke_unit("set_event_flag", &json!({ "name": name, "value": js_number_json(value) })).await {
            Ok(()) => {
                log_activity(
                    Section::ServerEvents.into(),
                    "set",
                    &format!("Event-Flag '{name}' auf {} gesetzt", js_number_string(value)),
                    Some("event-flag"),
                    Some(&name),
                );
                load(flags, loading, error).await;
            }
            Err(e) => error.set(Some(e)),
        }
        busy_name.set(None);
    }

    let on_save = Callback::new(move |(name, value): (String, f64)| {
        spawn_local(save_flag(name, value, flags, loading, error, busy_name));
    });

    let reset_flag = move |name: String| {
        spawn_local(async move {
            busy_name.set(Some(name.clone()));
            error.set(None);
            match invoke_unit("delete_event_flag", &json!({ "name": name })).await {
                Ok(()) => {
                    log_activity(
                        Section::ServerEvents.into(),
                        "reset",
                        &format!("Event-Flag '{name}' zurückgesetzt"),
                        Some("event-flag"),
                        Some(&name),
                    );
                    load(flags, loading, error).await;
                }
                Err(e) => error.set(Some(e)),
            }
            busy_name.set(None);
        });
    };
    let on_reset = Callback::new(reset_flag);

    let add_custom = move || {
        let name = custom_name.get_untracked();
        let value = custom_value.get_untracked();
        if name.trim().is_empty() || value.trim().is_empty() {
            return;
        }
        spawn_local(async move {
            save_flag(name.trim().to_string(), js_number(&value), flags, loading, error, busy_name).await;
            custom_name.set(String::new());
            custom_value.set(String::new());
        });
    };

    let custom_flags = move || flags.with(|f| f.as_ref().map(|f| f.iter().filter(|r| !is_curated(&r.name)).cloned().collect::<Vec<_>>()).unwrap_or_default());

    let busy_for = move |name: &'static str| Signal::derive(move || busy_name.with(|b| b.as_deref() == Some(name)));

    let rows = move |defs: &'static [FlagDef], unset_hint: &'static str| {
        defs.iter()
            .map(|def| {
                view! {
                    <FlagRow
                        def=def
                        current_value=flag_value(def.name)
                        unset_hint=unset_hint
                        on_save=on_save
                        on_reset=on_reset
                        busy=busy_for(def.name)
                    />
                }
            })
            .collect_view()
    };

    view! {
        <div class="space-y-6 pb-10">
            <div class="flex items-center justify-between">
                <PageHeader module=Section::ServerEvents title="Server-Events".to_string()/>
                <Button variant=ButtonVariant::Outline on_click=move |_| spawn_local(load(flags, loading, error)) disabled=loading>
                    <Icon name="RefreshCw" class=Signal::derive(move || if loading.get() { "size-4 animate-spin".to_string() } else { "size-4 ".to_string() })/>
                    "Neu laden"
                </Button>
            </div>

            <div class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="Info" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Änderungen hier landen in derselben Tabelle wie der Ingame-GM-Befehl "<code>"eventflag"</code>", wirken aber - anders als der Ingame-Befehl - "
                    <strong>"erst nach einem kompletten Server-Neustart"</strong>
                    "(DB+Game+Login), da die Werte nur einmal beim Start des DB-Prozesses geladen werden. "
                    <button class="underline" on:click=move |_| nav().set_section(Section::ServerControl)>
                        "Zur Server-Steuerung"
                    </button>
                    ". Für sofortige Wirkung ohne Neustart: den kopierbaren GM-Befehl neben jedem Wert im Spiel als GM eintippen - der wirkt sofort und speichert gleichzeitig in dieselbe Tabelle."
                </span>
            </div>

            {move || error.get().map(|e| view! { <p class="whitespace-pre-wrap text-sm text-destructive">{e}</p> })}

            <FlagSection title="Rate-Multiplikatoren (Doppel-Drop, Doppel-EXP, ...)" default_open=true>
                <p class="mb-2 text-xs text-muted-foreground">
                    "100 = normal. Server erlaubt 0-1000 (0%-1000%). Ohne gesetzten Wert gilt automatisch 100 (Standard) - das ist der Normalzustand, kein \"aus\"."
                </p>
                {rows(RATE_FLAGS, "Nicht gesetzt - aktuell 100% (Standard).")}
            </FlagSection>

            <FlagSection title="Premium-Varianten (nur für Spieler mit aktivem Premium-Timer)" default_open=false>
                {rows(RATE_FLAGS_PREMIUM, "Nicht gesetzt - aktuell 100% (Standard).")}
            </FlagSection>

            <FlagSection title="Saison-Events (Sonder-Drops)" default_open=true>
                <p class="mb-2 text-xs text-muted-foreground">
                    "Diese Werte sind keine direkte Prozentangabe, sondern eine \"je kleiner, desto häufiger\"-Stärke (Server-Formel: 40000 × Stufen-Faktor ÷ Wert) - Details siehe EVENT.md im Projekt. Ohne gesetzten Wert ist das Event komplett "
                    <strong>"aus"</strong>
                    " (0), nicht \"Standard\"."
                </p>
                {SEASONAL_FLAGS.iter().map(|s| {
                    let def = &s.def;
                    view! {
                        <FlagRow
                            def=def
                            current_value=flag_value(def.name)
                            unset_hint=format!(
                                "Nicht gesetzt - Event ist aus. Server-Vorgabewert, falls unterhalb von {} gesetzt: {}.",
                                s.min,
                                s.fallback_default
                            )
                            on_save=on_save
                            on_reset=on_reset
                            busy=busy_for(def.name)
                        />
                    }
                }).collect_view()}
            </FlagSection>

            <FlagSection title="Sichtbare Events (Sound/Schnee/NPC/Mob-Spawn)" default_open=false>
                <p class="mb-2 text-xs text-muted-foreground">
                    "Kein separates Event-System - dieselben Flags wie oben, aber mit echten Spawn-/ Sichtbarkeits-Effekten statt einer Drop-Chance. 1 = an, 0 = aus."
                </p>
                {rows(SPECIAL_EVENT_FLAGS, "Nicht gesetzt - aus.")}
            </FlagSection>

            <FlagSection title="Weitere/eigene Flags" default_open=true>
                <p class="mb-2 text-xs text-muted-foreground">
                    "Das Flag-System ist beliebig erweiterbar (jeder Name funktioniert) - hier stehen alle in der Datenbank gesetzten Flags, die oben in keiner Liste auftauchen."
                </p>
                <Show when=move || custom_flags().is_empty()>
                    <p class="text-sm text-muted-foreground">"Keine weiteren Flags gesetzt."</p>
                </Show>
                <For each=custom_flags key=|f| (f.name.clone(), f.value) let:f>
                    {
                        let name = f.name.clone();
                        let name_busy = f.name.clone();
                        let cmd = gm_command(&f.name, &f.value.to_string());
                        view! {
                            <div class="flex items-center justify-between gap-2 border-t border-border py-2">
                                <span class="text-sm">
                                    <code>{f.name.clone()}</code>
                                    " = "
                                    {f.value}
                                </span>
                                <div class="flex items-center gap-1">
                                    <CopyButton text=cmd/>
                                    <Button
                                        size=ButtonSize::Sm
                                        variant=ButtonVariant::Outline
                                        on_click=move |_| on_reset.run(name.clone())
                                        disabled=Signal::derive(move || busy_name.with(|b| b.as_deref() == Some(name_busy.as_str())))
                                    >
                                        <Icon name="Trash2" class="size-3.5"/>
                                    </Button>
                                </div>
                            </div>
                        }
                    }
                </For>
                <div class="mt-3 flex items-end gap-2 border-t border-border pt-3">
                    <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                        "Flag-Name"
                        <input
                            prop:value=move || custom_name.get()
                            on:input=move |e| custom_name.set(event_target_value(&e))
                            maxlength=32
                            class="w-40 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </label>
                    <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                        "Wert"
                        <input
                            type="number"
                            prop:value=move || custom_value.get()
                            on:input=move |e| custom_value.set(event_target_value(&e))
                            class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </label>
                    <Button
                        size=ButtonSize::Sm
                        on_click=move |_| add_custom()
                        disabled=Signal::derive(move || custom_name.with(|n| n.trim().is_empty()) || custom_value.with(|v| v.trim().is_empty()))
                    >
                        <Icon name="Plus" class="size-3.5"/>
                        "Hinzufügen"
                    </Button>
                </div>
            </FlagSection>
        </div>
    }
}
