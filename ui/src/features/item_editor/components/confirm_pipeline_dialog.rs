use crate::components::{Button, ButtonVariant};
use crate::features::item_editor::types::Mode;
use leptos::prelude::*;

#[component]
pub fn ConfirmPipelineDialog(
    mode: Mode,
    item_vnum: i64,
    has_new_icon: bool,
    has_model_copy: bool,
    ref_model_vnum: Option<i64>,
    #[prop(into)] on_cancel: Callback<()>,
    #[prop(into)] on_confirm: Callback<()>,
) -> impl IntoView {
    let create = mode == Mode::Create;
    let ref_model = ref_model_vnum.map(|v| v.to_string()).unwrap_or_default();
    view! {
        <div class="fixed inset-0 flex items-center justify-center bg-black/50">
            <div class="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
                <p class="text-sm font-medium">
                    "Item " {item_vnum} " jetzt " {if create { "anlegen" } else { "speichern" }} "?"
                </p>
                <ul class="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
                    <li>"Datenbankeintrag wird " {if create { "erstellt" } else { "aktualisiert" }}</li>
                    {has_new_icon.then(|| view! {
                        <li>"Icon wird nach pack/icon/icon/item geschrieben"</li>
                        <li><code>"icon.epk"</code> " wird neu gepackt (Backup wird vorher angelegt)"</li>
                    })}
                    {has_model_copy.then(|| view! {
                        <li>"3D-Modell von vnum " {ref_model.clone()} " wird für vnum " {item_vnum} " kopiert"</li>
                        <li><code>"item.epk"</code> " wird neu gepackt (Backup wird vorher angelegt)"</li>
                    })}
                    {has_new_icon.then(|| view! {
                        <li>
                            <code>"item_list.txt"</code> " wird um vnum " {item_vnum}
                            " ergänzt/aktualisiert (Backup wird vorher angelegt)"
                        </li>
                    })}
                    <li>
                        <code>"item_proto"</code>
                        " wird aus der DB neu erzeugt und im Client ersetzt (Backup wird vorher angelegt)"
                    </li>
                </ul>
                <div class="flex justify-end gap-2">
                    <Button variant=ButtonVariant::Outline on_click=move |_| on_cancel.run(())>"Abbrechen"</Button>
                    <Button on_click=move |_| on_confirm.run(())>"Ausführen"</Button>
                </div>
            </div>
        </div>
    }
}
