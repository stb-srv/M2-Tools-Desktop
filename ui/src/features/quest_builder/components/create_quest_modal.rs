//! Dialog "Neue Quest anlegen" (aus components/CreateQuestModal.tsx):
//! Vorlagen-Formulare, Freitext-Assistent, Lua-Vorschau und NPC/Mob/Item-Picker.

use super::shared::{Field, Modal, VnumInput};
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::quest_builder::apply_types::APPLY_TYPES;
use crate::features::quest_builder::js_num::num_str;
use crate::features::quest_builder::quest_description_parser::{
    pick_best_match, resolve_description, LookupKind, NameCandidate, UnresolvedLookup,
};
use crate::features::quest_builder::quest_templates::{
    generate_dungeon_quest, generate_multi_step_quest, generate_quest_lua, DungeonFloor, DungeonFormState,
    MultiStepFormState, QuestFormState, QuestStep, StepKind, TemplateType, DEFAULT_COOLDOWN_DAYS,
};
use crate::features::shared::{EntityBrowser, EntityKind};
use crate::util::log_activity;
use crate::store::navigation::Section;
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use std::sync::Arc;

/// NPCs und Monster liegen auf diesem Server in derselben `mob_proto`-Tabelle
/// - "npc"/"mob" durchsuchen beide sie, nur "item" die item_proto.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PickerKind {
    Npc,
    Mob,
    Item,
}

#[derive(Clone)]
struct PickerState {
    kind: PickerKind,
    on_pick: Arc<dyn Fn(i64) + Send + Sync>,
    initial_query: Option<String>,
}

const INPUT_FULL: &str = "w-full rounded-md border border-border bg-background px-2 py-1 text-sm";

type Getter = Arc<dyn Fn() -> String + Send + Sync>;
type Setter = Arc<dyn Fn(String) + Send + Sync>;

fn text_input(class: &'static str, get: Getter, set: Setter) -> AnyView {
    view! { <input prop:value=move || get() on:input=move |e| set(event_target_value(&e)) class=class/> }.into_any()
}

fn text_input_ph(class: &'static str, placeholder: &'static str, get: Getter, set: Setter) -> AnyView {
    view! { <input prop:value=move || get() on:input=move |e| set(event_target_value(&e)) placeholder=placeholder class=class/> }
        .into_any()
}

fn number_input(class: &'static str, min: Option<&'static str>, max: Option<&'static str>, get: Getter, set: Setter) -> AnyView {
    view! { <input type="number" min=min max=max prop:value=move || get() on:input=move |e| set(event_target_value(&e)) class=class/> }
        .into_any()
}

fn text_area(class: &'static str, get: Getter, set: Setter) -> AnyView {
    view! { <textarea prop:value=move || get() on:input=move |e| set(event_target_value(&e)) class=class></textarea> }.into_any()
}

fn vnum(get: Getter, set: Setter, pick: impl Fn() + Send + Sync + 'static) -> AnyView {
    view! { <VnumInput value=Signal::derive(move || get()) on_change=move |v| set(v) on_pick=move |_| pick()/> }.into_any()
}

fn field(label: impl Into<String>, child: AnyView) -> AnyView {
    view! { <Field label=label.into()>{child}</Field> }.into_any()
}

fn hint(text: &'static str) -> AnyView {
    view! { <p class="text-xs text-muted-foreground">{text}</p> }.into_any()
}

/// Liest/schreibt ein Feld einer `RwSignal<Struct>`.
macro_rules! bind {
    ($sig:expr, $($f:tt)+) => {{
        let sig = $sig;
        (
            Arc::new(move || sig.with(|x| x.$($f)+.clone())) as Getter,
            Arc::new(move |v: String| sig.update(|x| x.$($f)+ = v)) as Setter,
        )
    }};
}

/// Wie [`bind!`], für ein Element einer Liste in der Struktur.
macro_rules! bind_at {
    ($sig:expr, $list:ident, $i:expr, $($f:tt)+) => {{
        let sig = $sig;
        let i = $i;
        (
            Arc::new(move || sig.with(|x| x.$list.get(i).map(|e| e.$($f)+.clone()).unwrap_or_default())) as Getter,
            Arc::new(move |v: String| sig.update(|x| if let Some(e) = x.$list.get_mut(i) { e.$($f)+ = v })) as Setter,
        )
    }};
}

fn apply_type_select(get: Getter, set: Setter) -> AnyView {
    view! {
        <select
            on:change=move |e| set(event_target_value(&e))
            class="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
        >
            {APPLY_TYPES
                .iter()
                .map(|(v, label)| {
                    let value = v.to_string();
                    let cmp = value.clone();
                    let get = get.clone();
                    view! { <option value=value prop:selected=move || get() == cmp>{*label}</option> }
                })
                .collect_view()}
        </select>
    }
    .into_any()
}

#[component]
pub fn CreateQuestModal(#[prop(into)] on_close: Callback<()>, #[prop(into)] on_created: Callback<String>) -> impl IntoView {
    let new_category = RwSignal::new(String::new());
    let new_category_preview = RwSignal::new(String::new());
    let new_name = RwSignal::new(String::new());
    let new_name_preview = RwSignal::new(String::new());
    let template_type = RwSignal::new(TemplateType::Dialog);
    let form = RwSignal::new(QuestFormState::default());
    let dungeon_form = RwSignal::new(DungeonFormState::default());
    let multi_form = RwSignal::new(MultiStepFormState::default());
    let description_text = RwSignal::new(String::new());
    let description_busy = RwSignal::new(false);
    let description_notes = RwSignal::new(None::<Vec<String>>);
    let description_unresolved = RwSignal::new(Vec::<UnresolvedLookup>::new());
    let create_error = RwSignal::new(None::<String>);
    let creating_busy = RwSignal::new(false);
    let picker = RwSignal::new(None::<PickerState>);

    Effect::new(move |_| {
        let cat = new_category.get();
        let name = new_name.get();
        if cat.trim().is_empty() {
            new_category_preview.set(String::new());
        } else {
            spawn_local(async move {
                let r = invoke::<String>("sanitize_quest_identifier", &json!({ "name": cat })).await;
                new_category_preview.try_set(r.unwrap_or_default());
            });
        }
        if name.trim().is_empty() {
            new_name_preview.set(String::new());
        } else {
            spawn_local(async move {
                let r = invoke::<String>("sanitize_quest_identifier", &json!({ "name": name })).await;
                new_name_preview.try_set(r.unwrap_or_default());
            });
        }
    });

    let open_picker = move |kind: PickerKind, on_pick: Arc<dyn Fn(i64) + Send + Sync>, initial_query: Option<String>| {
        picker.set(Some(PickerState { kind, on_pick, initial_query }));
    };
    let pick_value = move |vnum: i64| {
        if let Some(p) = picker.get_untracked() {
            (p.on_pick)(vnum);
            picker.set(None);
        }
    };
    // VNUM-Feld mit Suchen-Knopf, der das Ergebnis über `set` zurückschreibt.
    let vnum_field = move |label: &'static str, kind: PickerKind, (get, set): (Getter, Setter)| -> AnyView {
        let set2 = set.clone();
        field(
            label,
            vnum(get, set, move || {
                let set = set2.clone();
                open_picker(kind, Arc::new(move |v| set(v.to_string())), None)
            }),
        )
    };

    let update_step = move |index: usize, f: &dyn Fn(&mut QuestStep)| {
        multi_form.update(|m| {
            if let Some(s) = m.steps.get_mut(index) {
                f(s)
            }
        })
    };

    // "Semi-KI"-Freitext-Assistent: reine Mustererkennung, keine echte KI.
    // Löst Namen über search_items/search_mobs auf und übernimmt das
    // Ergebnis in den Mehrschritt-Baukasten zur Prüfung.
    let analyze_description = move || {
        let text = description_text.get_untracked();
        if text.trim().is_empty() {
            return;
        }
        description_busy.set(true);
        description_notes.set(None);
        description_unresolved.set(vec![]);
        spawn_local(async move {
            let lookup = |phrase: String, kind: LookupKind| async move {
                let command = if kind == LookupKind::Item { "search_items" } else { "search_mobs" };
                let results = invoke::<Vec<NameCandidate>>(command, &json!({ "query": phrase })).await?;
                Ok(pick_best_match(&phrase, &results))
            };
            match resolve_description(&text, lookup).await {
                Ok(r) => {
                    template_type.try_set(TemplateType::MultiStep);
                    multi_form.try_set(MultiStepFormState {
                        steps: if r.steps.is_empty() { vec![QuestStep::default()] } else { r.steps },
                        repeatable: r.repeatable,
                        cooldown_days: r.cooldown_days.map(num_str).unwrap_or_else(|| DEFAULT_COOLDOWN_DAYS.to_string()),
                    });
                    description_notes.try_set(Some(r.notes));
                    description_unresolved.try_set(r.unresolved);
                }
                Err(e) => web_sys::console::error_1(&e.into()),
            }
            description_busy.try_set(false);
        });
    };

    // Öffnet für einen nicht aufgelösten Namen denselben Picker wie das
    // VNUM-Feld, vorausgefüllt mit der erkannten Namensphrase.
    let resolve_unresolved = move |index: usize| {
        let Some(entry) = description_unresolved.with_untracked(|u| u.get(index).cloned()) else { return };
        let kind = match entry.kind {
            LookupKind::Npc => PickerKind::Npc,
            LookupKind::Mob => PickerKind::Mob,
            LookupKind::Item => PickerKind::Item,
        };
        let (step_index, f) = (entry.step_index, entry.field);
        open_picker(kind, Arc::new(move |v| update_step(step_index, &|s| f.set(s, v.to_string()))), Some(entry.query.clone()));
        description_unresolved.update(|u| {
            if index < u.len() {
                u.remove(index);
            }
        });
    };

    let preview = Memo::new(move |_| {
        let name = new_name_preview.get();
        let name = if name.is_empty() { "neue_quest".to_string() } else { name };
        match template_type.get() {
            TemplateType::Dungeon => dungeon_form.with(|f| generate_dungeon_quest(&name, f)),
            TemplateType::MultiStep => multi_form.with(|f| generate_multi_step_quest(&name, f)),
            t => form.with(|f| generate_quest_lua(&name, t, f)),
        }
    });

    let submit_create = move || {
        let category = new_category_preview.get_untracked();
        let name = new_name_preview.get_untracked();
        if category.is_empty() || name.is_empty() {
            return;
        }
        creating_busy.set(true);
        create_error.set(None);
        let content = preview.get_untracked();
        let extension = if template_type.get_untracked() == TemplateType::Dungeon { "quest" } else { "lua" };
        spawn_local(async move {
            let res = invoke::<String>(
                "create_quest_file",
                &json!({ "category": category, "name": name, "content": content, "extension": extension }),
            )
            .await;
            match res {
                Ok(path) => {
                    log_activity(Section::QuestBuilder.into(), "create", &format!("Quest '{path}' angelegt"), Some("quest"), Some(&path));
                    on_close.run(());
                    on_created.run(path);
                }
                Err(e) => {
                    create_error.try_set(Some(e));
                }
            }
            creating_busy.try_set(false);
        });
    };

    // ------------------------------------------------------------ Teil-Ansichten

    let simple_header = move || {
        view! {
            {vnum_field("NPC-VNUM", PickerKind::Npc, bind!(form, npc_vnum))}
            {field("Dialog-Auslöser (chat-Label)", { let (g, s) = bind!(form, chat_label); text_input(INPUT_FULL, g, s) })}
            {field("NPC-Titel (say_title)", { let (g, s) = bind!(form, npc_title); text_input(INPUT_FULL, g, s) })}
        }
        .into_any()
    };

    let collect_view = move || {
        view! {
            <div class="flex gap-2">
                {vnum_field("Item-VNUM (abzugeben)", PickerKind::Item, bind!(form, required_item_vnum))}
                {field("Anzahl", { let (g, s) = bind!(form, required_item_count); number_input("w-20 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("1"), None, g, s) })}
            </div>
            {field("Text bei Erfolg", { let (g, s) = bind!(form, success_text); text_area("h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
            {field("Text wenn noch nicht genug Items", { let (g, s) = bind!(form, fail_text); text_area("h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
        }
        .into_any()
    };

    let chance_collect_view = move || {
        view! {
            <div class="flex gap-2">
                {vnum_field("Item-VNUM (abzugeben)", PickerKind::Item, bind!(form, required_item_vnum))}
                {field("Anzahl pro Versuch", { let (g, s) = bind!(form, items_per_attempt); number_input("w-20 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("1"), None, g, s) })}
            </div>
            <div class="flex gap-2">
                {field("Chance der Annahme (%)", { let (g, s) = bind!(form, accept_chance_percent); number_input("w-24 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("0"), Some("100"), g, s) })}
                {field("Nötige Erfolge bis Belohnung", { let (g, s) = bind!(form, required_successes); number_input("w-24 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("1"), None, g, s) })}
            </div>
            {hint("Das Item wird bei jedem Versuch verbraucht - unabhängig davon, ob der Wurf erfolgreich war (genau wie in Biologie/Orkzahn.lua auf dem Server).")}
            {field("Text bei Erfolg (%d = aktueller Fortschritt, %d = Ziel)", { let (g, s) = bind!(form, progress_text); text_input(INPUT_FULL, g, s) })}
            {field("Text bei Fehlschlag", { let (g, s) = bind!(form, fail_text); text_area("h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
            {field("Text wenn Item nicht vorhanden", { let (g, s) = bind!(form, no_item_text); text_area("h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
            {field("Text bei Abschluss (genug Erfolge erreicht)", { let (g, s) = bind!(form, complete_text); text_area("h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
        }
        .into_any()
    };

    let kill_view = move || {
        view! {
            <div class="flex gap-2">
                {vnum_field("Monster-VNUM", PickerKind::Mob, bind!(form, mob_vnum))}
                {field("Anzahl zu töten", { let (g, s) = bind!(form, required_kills); number_input("w-24 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("1"), None, g, s) })}
            </div>
            {field("Fortschrittstext (%d = aktuell, %d = Ziel)", { let (g, s) = bind!(form, progress_text); text_input(INPUT_FULL, g, s) })}
            {field("Text bei Erfolg", { let (g, s) = bind!(form, success_text); text_area("h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
        }
        .into_any()
    };

    let use_view = move || {
        view! {
            {vnum_field("Item-VNUM (zu benutzen)", PickerKind::Item, bind!(form, use_item_vnum))}
            {field("Text beim Benutzen", { let (g, s) = bind!(form, use_text); text_area("h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
        }
        .into_any()
    };

    let buffed_view = move || {
        let slots = (0..4usize)
            .map(|slot| {
                view! {
                    <div class="flex gap-2">
                        {field(format!("Attribut {}", slot + 1), { let (g, s) = bind!(form, buffed_attr_type[slot]); apply_type_select(g, s) })}
                        {field("Wert", { let (g, s) = bind!(form, buffed_attr_value[slot]); number_input("w-24 rounded-md border border-border bg-background px-2 py-1 text-sm", None, None, g, s) })}
                    </div>
                }
            })
            .collect_view();
        view! {
            <div class="flex gap-2">
                {vnum_field("Zu verschenkendes Item-VNUM", PickerKind::Item, bind!(form, buffed_item_vnum))}
                {field("Anzahl", { let (g, s) = bind!(form, buffed_item_count); number_input("w-20 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("1"), None, g, s) })}
            </div>
            {hint("Für eine fertige \"+9\"-Waffe/Rüstung hier die VNUM der +9-Stufe eintragen (z.B. aus dem Aufwertungs-Editor), nicht die +0-Basis - die Kette selbst wird hierdurch nicht ausgelöst, nur ein bereits existierendes Item vergeben.")}
            <div class="space-y-1 rounded-md border border-border p-2">
                <span class="text-xs font-medium text-muted-foreground">"Feste Bonus-Attribute (bis zu 4, leer = Slot ungenutzt)"</span>
                {slots}
            </div>
            {field("Belohnung Yang (optional)", { let (g, s) = bind!(form, reward_money); number_input("w-32 rounded-md border border-border bg-background px-2 py-1 text-sm", None, None, g, s) })}
        }
        .into_any()
    };

    let reward_view = move || {
        view! {
            <div class="flex gap-2">
                {vnum_field("Belohnung Item-VNUM (optional)", PickerKind::Item, bind!(form, reward_item_vnum))}
                {field("Anzahl", { let (g, s) = bind!(form, reward_item_count); number_input("w-20 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("1"), None, g, s) })}
            </div>
            {field("Belohnung Yang (optional)", { let (g, s) = bind!(form, reward_money); number_input("w-32 rounded-md border border-border bg-background px-2 py-1 text-sm", None, None, g, s) })}
        }
        .into_any()
    };

    let floor_count = Memo::new(move |_| dungeon_form.with(|d| d.floors.len()));
    let floor_view = move |index: usize| {
        view! {
            <div class="space-y-1 rounded-md border border-border p-2">
                <div class="flex items-center justify-between">
                    <span class="text-xs font-medium">"Etage " {index + 1}</span>
                    <Show when=move || { floor_count.get() > 1 }>
                        <Button
                            variant=ButtonVariant::Ghost
                            size=ButtonSize::IconSm
                            on_click=move |_| dungeon_form.update(|d| if d.floors.len() > 1 { d.floors.remove(index); })
                        >
                            <Icon name="X" class="size-3.5"/>
                        </Button>
                    </Show>
                </div>
                {vnum_field("Boss-/Trigger-Monster-VNUM (löst nächste Etage aus)", PickerKind::Mob, bind_at!(dungeon_form, floors, index, boss_vnum))}
                <div class="flex gap-2">
                    {field("Einstiegs-X", { let (g, s) = bind_at!(dungeon_form, floors, index, entry_x); number_input("w-20 rounded-md border border-border bg-background px-2 py-1 text-sm", None, None, g, s) })}
                    {field("Einstiegs-Y", { let (g, s) = bind_at!(dungeon_form, floors, index, entry_y); number_input("w-20 rounded-md border border-border bg-background px-2 py-1 text-sm", None, None, g, s) })}
                </div>
                {field("Regen-Datei (Monster-Spawns dieser Etage)", { let (g, s) = bind_at!(dungeon_form, floors, index, regen_file); text_input_ph(INPUT_FULL, "data/dungeon/meindungeon/etage1_regen.txt", g, s) })}
            </div>
        }
        .into_any()
    };

    let dungeon_view = move || {
        view! {
            {vnum_field("Einstiegs-NPC-VNUM", PickerKind::Npc, bind!(dungeon_form, entry_npc_vnum))}
            {field("Dialog-Auslöser (chat-Label)", { let (g, s) = bind!(dungeon_form, entry_chat_label); text_input(INPUT_FULL, g, s) })}
            {field("NPC-Titel (say_title)", { let (g, s) = bind!(dungeon_form, entry_title); text_input(INPUT_FULL, g, s) })}
            {field("Einstiegstext", { let (g, s) = bind!(dungeon_form, entry_text); text_area("h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
            <div class="flex gap-2">
                {field("Mindest-Level (0 = keins)", { let (g, s) = bind!(dungeon_form, required_level); number_input("w-24 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("0"), None, g, s) })}
                {field("Dungeon-Map-Index", { let (g, s) = bind!(dungeon_form, dungeon_map_index); number_input("w-28 rounded-md border border-border bg-background px-2 py-1 text-sm", None, None, g, s) })}
            </div>
            <div class="flex gap-2">
                {field("Map-Index-Bereich Start", { let (g, s) = bind!(dungeon_form, map_index_range_start); number_input("w-28 rounded-md border border-border bg-background px-2 py-1 text-sm", None, None, g, s) })}
                {field("…bis (exklusiv)", { let (g, s) = bind!(dungeon_form, map_index_range_end); number_input("w-28 rounded-md border border-border bg-background px-2 py-1 text-sm", None, None, g, s) })}
            </div>
            {hint("Der Map-Index-Bereich ist die vom Server für diesen Dungeon-Typ vergebene Instanz-Map-Range (z.B. 660000–670000 beim Daemonenturm) - kommt aus der Server-Dungeon-Konfiguration, nicht aus diesem Tool. Ohne den richtigen Bereich lösen die Boss-Kill-Trigger unten nicht korrekt aus.")}

            <div class="space-y-2 rounded-md border border-border p-2">
                <div class="flex items-center justify-between">
                    <span class="text-xs font-medium text-muted-foreground">"Etagen (" {move || floor_count.get()} ")"</span>
                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| dungeon_form.update(|d| d.floors.push(DungeonFloor::default()))>
                        <Icon name="Plus" class="size-3.5"/>
                        "Etage"
                    </Button>
                </div>
                {move || (0..floor_count.get()).map(floor_view).collect_view()}
                {hint("Die Regen-Datei muss auf dem Server bereits existieren (eigenes Dateiformat, hier nicht erzeugt) - für „Etage 1\" wird sie beim Betreten geladen, für jede weitere Etage beim Töten des Bosses der vorherigen Etage.")}
            </div>

            <div class="flex gap-2">
                {vnum_field("Belohnung Item-VNUM (optional, letzte Etage)", PickerKind::Item, bind!(dungeon_form, reward_item_vnum))}
                {field("Anzahl", { let (g, s) = bind!(dungeon_form, reward_item_count); number_input("w-20 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("1"), None, g, s) })}
            </div>
            <div class="flex gap-2">
                {field("Belohnung Yang (optional)", { let (g, s) = bind!(dungeon_form, reward_money); number_input("w-32 rounded-md border border-border bg-background px-2 py-1 text-sm", None, None, g, s) })}
                {field("Sekunden bis Rauswurf nach Abschluss", { let (g, s) = bind!(dungeon_form, exit_delay_seconds); number_input("w-24 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("0"), None, g, s) })}
            </div>
        }
        .into_any()
    };

    let step_count = Memo::new(move |_| multi_form.with(|m| m.steps.len()));
    let step_view = move |index: usize| {
        let kind = Memo::new(move |_| multi_form.with(|m| m.steps.get(index).map(|s| s.kind).unwrap_or(StepKind::Dialog)));
        let kind_fields = move || match kind.get() {
            StepKind::Dialog => field("Dialogtext", { let (g, s) = bind_at!(multi_form, steps, index, dialog_text); text_area("h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) }),
            StepKind::Collect => view! {
                <div class="flex gap-2">
                    {vnum_field("Item-VNUM (abzugeben)", PickerKind::Item, bind_at!(multi_form, steps, index, required_item_vnum))}
                    {field("Anzahl", { let (g, s) = bind_at!(multi_form, steps, index, required_item_count); number_input("w-20 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("1"), None, g, s) })}
                </div>
                {field("Text bei Erfolg", { let (g, s) = bind_at!(multi_form, steps, index, success_text); text_area("h-14 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
                {field("Text wenn noch nicht genug Items", { let (g, s) = bind_at!(multi_form, steps, index, fail_text); text_area("h-14 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
            }
            .into_any(),
            StepKind::Kill => view! {
                <div class="flex gap-2">
                    {vnum_field("Monster-VNUM", PickerKind::Mob, bind_at!(multi_form, steps, index, mob_vnum))}
                    {field("Anzahl zu töten", { let (g, s) = bind_at!(multi_form, steps, index, required_kills); number_input("w-24 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("1"), None, g, s) })}
                </div>
                {field("Fortschrittstext (%d = aktuell, %d = Ziel)", { let (g, s) = bind_at!(multi_form, steps, index, progress_text); text_input(INPUT_FULL, g, s) })}
                {field("Text bei Erfolg", { let (g, s) = bind_at!(multi_form, steps, index, success_text); text_area("h-14 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
            }
            .into_any(),
            StepKind::Use => view! {
                {vnum_field("Item-VNUM (zu benutzen)", PickerKind::Item, bind_at!(multi_form, steps, index, use_item_vnum))}
                {field("Text beim Benutzen", { let (g, s) = bind_at!(multi_form, steps, index, use_text); text_area("h-14 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
            }
            .into_any(),
        };
        let npc_fields = move || {
            (kind.get() != StepKind::Use).then(|| {
                view! {
                    <div class="flex gap-2">
                        {vnum_field("NPC-VNUM", PickerKind::Npc, bind_at!(multi_form, steps, index, npc_vnum))}
                        {field("chat-Label", { let (g, s) = bind_at!(multi_form, steps, index, chat_label); text_input("w-28 rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) })}
                    </div>
                    {field("NPC-Titel (say_title)", { let (g, s) = bind_at!(multi_form, steps, index, npc_title); text_input(INPUT_FULL, g, s) })}
                }
            })
        };
        let attr_slots = (0..4usize)
            .map(|slot| {
                view! {
                    <div class="flex gap-2">
                        {field(format!("Attribut {}", slot + 1), { let (g, s) = bind_at!(multi_form, steps, index, reward_attr_type[slot]); apply_type_select(g, s) })}
                        {field("Wert", { let (g, s) = bind_at!(multi_form, steps, index, reward_attr_value[slot]); number_input("w-24 rounded-md border border-border bg-background px-2 py-1 text-sm", None, None, g, s) })}
                    </div>
                }
            })
            .collect_view();
        view! {
            <div class="space-y-1 rounded-md border border-border p-2">
                <div class="flex items-center justify-between">
                    <span class="text-xs font-medium">"Schritt " {index + 1}</span>
                    <Show when=move || { step_count.get() > 1 }>
                        <Button
                            variant=ButtonVariant::Ghost
                            size=ButtonSize::IconSm
                            on_click=move |_| multi_form.update(|m| if m.steps.len() > 1 { m.steps.remove(index); })
                        >
                            <Icon name="X" class="size-3.5"/>
                        </Button>
                    </Show>
                </div>
                <Field label="Art".to_string()>
                    <select
                        on:change=move |e| {
                            if let Some(k) = StepKind::from_id(&event_target_value(&e)) {
                                update_step(index, &|s| s.kind = k);
                            }
                        }
                        class=INPUT_FULL
                    >
                        {StepKind::ALL
                            .iter()
                            .map(|k| {
                                let k = *k;
                                view! { <option value=k.id() prop:selected=move || kind.get() == k>{k.label()}</option> }
                            })
                            .collect_view()}
                    </select>
                </Field>
                {npc_fields}
                {kind_fields}
                <div class="space-y-1 rounded-md border border-border p-2">
                    <span class="text-xs font-medium text-muted-foreground">"Zwischenbelohnung (optional)"</span>
                    <div class="flex gap-2">
                        {vnum_field("Item-VNUM", PickerKind::Item, bind_at!(multi_form, steps, index, reward_item_vnum))}
                        {field("Anzahl", { let (g, s) = bind_at!(multi_form, steps, index, reward_item_count); number_input("w-20 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("1"), None, g, s) })}
                    </div>
                    {field("Yang", { let (g, s) = bind_at!(multi_form, steps, index, reward_money); number_input("w-32 rounded-md border border-border bg-background px-2 py-1 text-sm", None, None, g, s) })}
                    <span class="text-xs font-medium text-muted-foreground">
                        "Feste Bonus-Attribute (optional, überschreibt den Zufalls-Roll - z.B. \"Max. HP +250\")"
                    </span>
                    {attr_slots}
                </div>
            </div>
        }
        .into_any()
    };

    let multi_step_view = move || {
        view! {
            <div class="space-y-2 rounded-md border border-border p-2">
                <label class="flex items-center gap-2 text-sm">
                    <input
                        type="checkbox"
                        prop:checked=move || multi_form.with(|m| m.repeatable)
                        on:change=move |e| {
                            let checked = event_target_checked(&e);
                            multi_form.update(|m| m.repeatable = checked);
                        }
                    />
                    "Wiederholbar (mit Cooldown)"
                </label>
                <Show when=move || multi_form.with(|m| m.repeatable)>
                    {field("Cooldown (Tage)", { let (g, s) = bind!(multi_form, cooldown_days); number_input("w-24 rounded-md border border-border bg-background px-2 py-1 text-sm", Some("0"), None, g, s) })}
                    {hint("Nach Abschluss springt die Quest zurück auf Schritt 1 statt zu enden - Schritt 1 ist bis zum Ablauf des Cooldowns gesperrt, alle Töten-Zähler in der Kette werden bei jedem Abschluss automatisch zurückgesetzt.")}
                </Show>
                <div class="flex items-center justify-between">
                    <span class="text-xs font-medium text-muted-foreground">"Schritte (" {move || step_count.get()} ")"</span>
                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| multi_form.update(|m| m.steps.push(QuestStep::default()))>
                        <Icon name="Plus" class="size-3.5"/>
                        "Schritt"
                    </Button>
                </div>
                {move || (0..step_count.get()).map(step_view).collect_view()}
                {hint("Jeder Schritt wird ein eigener Quest-Zustand - der Spieler kommt automatisch beim nächsten Schritt an, sobald der vorherige abgeschlossen ist (set_state im Hintergrund, kein Zutun des Spielers nötig). Beim letzten Schritt wird die Zwischenbelohnung zur eigentlichen Abschlussbelohnung.")}
            </div>
        }
        .into_any()
    };

    let template_fields = move || {
        let t = template_type.get();
        let simple = !matches!(t, TemplateType::Dungeon | TemplateType::MultiStep);
        view! {
            {simple.then(simple_header)}
            {matches!(t, TemplateType::Dialog | TemplateType::BuffedItem).then(|| field("Dialogtext", { let (g, s) = bind!(form, dialog_text); text_area("h-20 w-full rounded-md border border-border bg-background px-2 py-1 text-sm", g, s) }))}
            {(t == TemplateType::Collect).then(collect_view)}
            {(t == TemplateType::ChanceCollect).then(chance_collect_view)}
            {(t == TemplateType::Kill).then(kill_view)}
            {(t == TemplateType::Use).then(use_view)}
            {(t == TemplateType::BuffedItem).then(buffed_view)}
            {(simple && t != TemplateType::BuffedItem).then(reward_view)}
            {(t == TemplateType::Dungeon).then(dungeon_view)}
            {(t == TemplateType::MultiStep).then(multi_step_view)}
        }
    };

    let extension = move || if template_type.get() == TemplateType::Dungeon { "quest" } else { "lua" };

    view! {
        <Modal on_close=move |_| on_close.run(()) wide=true>
            <div class="flex max-h-[80vh] gap-4">
                <div class="w-80 shrink-0 space-y-3 overflow-y-auto pr-2">
                    <p class="text-sm font-medium">"Neue Quest anlegen"</p>
                    <div class="flex gap-2">
                        {field("Kategorie (Ordner)", text_input_ph("w-36 rounded-md border border-border bg-background px-2 py-1 text-sm", "z.B. Biologie", Arc::new(move || new_category.get()), Arc::new(move |v| new_category.set(v))))}
                        {field("Name", text_input_ph("w-36 rounded-md border border-border bg-background px-2 py-1 text-sm", "z.B. Orkzahn", Arc::new(move || new_name.get()), Arc::new(move |v| new_name.set(v))))}
                    </div>
                    <Show when=move || !new_category.with(|c| c.trim().is_empty()) || !new_name.with(|n| n.trim().is_empty())>
                        <p class="text-xs text-muted-foreground">
                            "Wird gespeichert als: "
                            <code>
                                {move || {
                                    let c = new_category_preview.get();
                                    let n = new_name_preview.get();
                                    format!(
                                        "{}/{}.{}",
                                        if c.is_empty() { "…".to_string() } else { c },
                                        if n.is_empty() { "…".to_string() } else { n },
                                        extension(),
                                    )
                                }}
                            </code>
                        </p>
                    </Show>

                    <div class="space-y-2 rounded-md border border-border p-2">
                        <div class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
                            <Icon name="Sparkles" class="size-3.5"/>
                            "Freitext-Assistent (optional)"
                        </div>
                        <p class="text-xs text-muted-foreground">
                            "Mustererkennung, keine echte KI - einfache, klare Sätze funktionieren am besten (z.B. \"Rede mit Hans, sammle 10 Wolfsfelle, dann bekommt man 100 Yang.\"). NPC und Item am besten mit \"bei\" trennen (\"gib 20 Orkzähne bei Gemi ab\") statt in einer Aufzählung (\"bringe der Gemi, 20 Orkzähne\") - sonst kann die Zuordnung danebengehen. Füllt den Mehrschritt-Baukasten unten vor - Ergebnis vor dem Anlegen prüfen!"
                        </p>
                        <textarea
                            prop:value=move || description_text.get()
                            on:input=move |e| description_text.set(event_target_value(&e))
                            placeholder="Rede mit Hans, sammle 10 Wolfsfelle, dann bekommt man 100 Yang."
                            class="h-16 w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                        ></textarea>
                        <Button
                            variant=ButtonVariant::Outline
                            size=ButtonSize::Sm
                            on_click=move |_| analyze_description()
                            disabled=Signal::derive(move || description_text.with(|t| t.trim().is_empty()) || description_busy.get())
                        >
                            <Icon name="Sparkles" class="size-3.5"/>
                            {move || if description_busy.get() { "Analysiere…" } else { "Analysieren" }}
                        </Button>
                        {move || {
                            description_notes.get().map(|notes| {
                                if notes.is_empty() {
                                    view! {
                                        <p class="text-xs text-muted-foreground">
                                            "Alles erkannt - Ergebnis im Mehrschritt-Baukasten unten geprüft übernehmen."
                                        </p>
                                    }
                                    .into_any()
                                } else {
                                    view! {
                                        <div class="space-y-1.5 rounded-md border border-amber-500/40 bg-amber-500/10 p-2 text-xs text-amber-700 dark:text-amber-400">
                                            {notes.into_iter().map(|n| view! { <p>{n}</p> }).collect_view()}
                                            {move || {
                                                description_unresolved
                                                    .get()
                                                    .into_iter()
                                                    .enumerate()
                                                    .map(|(i, entry)| {
                                                        view! {
                                                            <div class="flex items-center justify-between gap-2">
                                                                <span>
                                                                    {format!("\"{}\" nicht gefunden (Schritt {})", entry.query, entry.step_index + 1)}
                                                                </span>
                                                                <Button
                                                                    variant=ButtonVariant::Outline
                                                                    size=ButtonSize::Sm
                                                                    on_click=move |_| resolve_unresolved(i)
                                                                    class="shrink-0"
                                                                >
                                                                    <Icon name="Search" class="size-3.5"/>
                                                                    "Suchen"
                                                                </Button>
                                                            </div>
                                                        }
                                                    })
                                                    .collect_view()
                                            }}
                                        </div>
                                    }
                                    .into_any()
                                }
                            })
                        }}
                    </div>

                    <Field label="Vorlage".to_string()>
                        <select
                            on:change=move |e| {
                                if let Some(t) = TemplateType::from_id(&event_target_value(&e)) {
                                    template_type.set(t);
                                }
                            }
                            class=INPUT_FULL
                        >
                            {TemplateType::ALL
                                .iter()
                                .map(|t| {
                                    let t = *t;
                                    view! { <option value=t.id() prop:selected=move || template_type.get() == t>{t.label()}</option> }
                                })
                                .collect_view()}
                        </select>
                    </Field>
                    <p class="text-xs text-muted-foreground">{move || template_type.get().hint()}</p>

                    {template_fields}

                    {move || create_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                    <Button
                        on_click=move |_| submit_create()
                        disabled=Signal::derive(move || {
                            new_category_preview.with(|c| c.is_empty()) || new_name_preview.with(|n| n.is_empty()) || creating_busy.get()
                        })
                    >
                        <Icon name="Wand2" class="size-4"/>
                        {move || if creating_busy.get() { "Lege an…" } else { "Anlegen" }}
                    </Button>
                </div>

                <div class="flex min-w-0 flex-1 flex-col gap-1">
                    <span class="text-xs font-medium text-muted-foreground">"Vorschau (nach dem Anlegen als Code editierbar)"</span>
                    <pre class="min-h-0 flex-1 overflow-auto rounded-md border border-border bg-muted/30 p-2 font-mono text-xs">
                        {move || preview.get()}
                    </pre>
                </div>
            </div>
        </Modal>

        // NPC/Mob/Item-Picker
        {move || {
            picker.get().map(|p| {
                let kind = if p.kind == PickerKind::Item { EntityKind::Item } else { EntityKind::Mob };
                view! {
                    <Modal on_close=move |_| picker.set(None)>
                        <EntityBrowser
                            kind=kind
                            pick_label="Übernehmen"
                            auto_focus=true
                            max_height_class="max-h-64"
                            initial_query=p.initial_query.clone().unwrap_or_default()
                            on_pick=move |r: crate::features::shared::EntityRow| pick_value(r.vnum)
                        />
                    </Modal>
                }
            })
        }}
    }
}
