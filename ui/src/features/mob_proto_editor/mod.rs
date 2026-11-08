//! Mob-Proto-Editor (Port von `src/features/mob-proto-editor/MobProtoEditor.tsx`).

use crate::components::PageHeader;
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow, GenericRowEditor};
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
