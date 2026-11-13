//! Mob-Proto-Editor (Port von `src/features/mob-proto-editor/MobProtoEditor.tsx`).

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::shared::mob_drops_modal::rank_label;
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow, GenericRowEditor, MobBrief, MobDropsModal};
use crate::tauri::invoke;
use leptos::task::spawn_local;
use serde_json::json;
use crate::store::navigation::{nav, ModuleId, Section};
use leptos::prelude::*;

/// `String(Number(s))` - so wurde die übergebene vnum im alten Frontend
/// normalisiert (z.B. " 0101 " -> "101", ungültig -> "NaN").
pub fn normalize_vnum(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        return "0".into();
    }
    if t.chars().any(|c| c.is_ascii_alphabetic() && c != 'e' && c != 'E') {
        return "NaN".into();
    }
    match t.parse::<f64>() {
        Ok(n) if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e21 => format!("{}", n as i64),
        Ok(n) if n.is_finite() => n.to_string(),
        _ => "NaN".into(),
    }
}

/// Rang, Level und „Was droppt dieser Mob?“ über dem Zeilen-Editor.
#[component]
fn MobInfoBar(vnum: u32) -> impl IntoView {
    let brief = RwSignal::new(None::<Result<Option<MobBrief>, String>>);
    let drops_open = RwSignal::new(false);
    spawn_local(async move {
        brief.set(Some(invoke::<Option<MobBrief>>("get_mob_brief", &json!({ "vnum": vnum })).await));
    });
    view! {
        <div class="flex flex-wrap items-center gap-3 rounded-lg border border-border p-3 text-sm">
            {move || match brief.get() {
                None => view! { <span class="text-muted-foreground">"Lade Mob…"</span> }.into_any(),
                Some(Err(e)) => view! { <span class="text-xs text-amber-600 dark:text-amber-400">"Rang nicht lesbar: " {e}</span> }.into_any(),
                Some(Ok(None)) => view! { <span class="text-muted-foreground">{format!("Mob #{vnum} nicht gefunden.")}</span> }.into_any(),
                Some(Ok(Some(m))) => {
                    let rank = m.rank.clone();
                    view! {
                        <span class="font-medium">{m.name.clone()}</span>
                        {m.level.map(|l| view! { <span class="text-muted-foreground">{format!("Level {l}")}</span> })}
                        {match rank {
                            Some(r) => {
                                let common = matches!(r.as_str(), "PAWN" | "S_PAWN" | "KNIGHT" | "S_KNIGHT");
                                view! {
                                    <span class="rounded-md border border-border bg-muted px-2 py-0.5 text-xs font-medium">
                                        {format!("Rang {r} · {}", rank_label(&r))}
                                    </span>
                                    <span class="text-xs text-muted-foreground">
                                        {if common {
                                            format!("Bekommt die Common-Drops aus Block {r}.")
                                        } else {
                                            "Bosse bekommen keine Common-Drops, nur Mob-Drops.".to_string()
                                        }}
                                    </span>
                                }.into_any()
                            }
                            None => view! { <span class="text-xs text-muted-foreground">"Rang unbekannt"</span> }.into_any(),
                        }}
                    }.into_any()
                }
            }}
            <Button class="ml-auto" size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| drops_open.set(true)>
                <Icon name="List" class="size-3.5"/>
                "Was droppt dieser Mob?"
            </Button>
            {move || drops_open.get().then(|| view! { <MobDropsModal vnum=vnum on_close=move |_| drops_open.set(false)/> })}
        </div>
    }
}

#[component]
pub fn MobProtoEditor() -> impl IntoView {
    let editing_vnum = RwSignal::new(None::<String>);

    // Globale Suche (Strg+Umschalt+F) springt hierher mit einer vnum.
    if let Some(target_ref) = nav().consume_pending_selection(Section::MobProtoEditor) {
        editing_vnum.set(Some(normalize_vnum(&target_ref)));
    }

    view! {
        <div class="space-y-4">
            <PageHeader module=Section::MobProtoEditor title="Mob-Proto-Editor".to_string()/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Bearbeitet " <code>"player.mob_proto"</code>
                " (Monster-/NPC-Stats) direkt in der Datenbank. Anders als der Item Editor zeigt dies jede Spalte generisch (Rohwerte, keine bekannten Flag-Bedeutungen wie AI-Verhalten) - Spaltensemantik für diese Tabelle wurde nicht gegen einen echten Server verifiziert. Neue Mobs anlegen ist bewusst nicht Teil dieser ersten Version. Änderungen wirken in aller Regel erst nach einem Server-Neustart (Server-Steuerung → Neustarten), da "
                <code>"mob_proto"</code>
                " typischerweise beim Serverstart in den Speicher geladen wird."
            </p>

            <EntityBrowser
                kind=EntityKind::Mob
                pick_label="Bearbeiten"
                max_height_class="max-h-[28rem]"
                on_pick=move |r: EntityRow| editing_vnum.set(Some(r.vnum.to_string()))
            />

            {move || editing_vnum.get().map(|vnum| view! {
                {vnum.parse::<u32>().ok().map(|v| view! { <MobInfoBar vnum=v/> })}
                <GenericRowEditor
                    database="player"
                    table="mob_proto"
                    pk_value=vnum
                    on_close=move |_| editing_vnum.set(None)
                    activity_module=ModuleId::Section(Section::MobProtoEditor)
                />
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_like_string_number() {
        assert_eq!(normalize_vnum("101"), "101");
        assert_eq!(normalize_vnum(" 0101 "), "101");
        assert_eq!(normalize_vnum("abc"), "NaN");
    }
}
