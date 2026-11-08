use super::super::IconMap;
use super::shared::Modal;
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow, RowRenderer};
use leptos::prelude::*;
use std::sync::Arc;

#[component]
pub fn ItemPickerModal(
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_pick: Callback<EntityRow>,
    icons: RwSignal<IconMap>,
    #[prop(into)] on_rows_change: Callback<Vec<EntityRow>>,
) -> impl IntoView {
    let leading: RowRenderer = Arc::new(move |r: &EntityRow| {
        let vnum = r.vnum;
        (move || {
            icons.with(|m| m.get(&vnum).cloned().flatten()).map(|src| {
                view! { <img src=src alt="" class="size-6 shrink-0 object-contain [image-rendering:pixelated]"/> }
            })
        })
        .into_any()
    });
    view! {
        <Modal on_close=on_close>
            <EntityBrowser
                kind=EntityKind::Item
                pick_label="Hinzufügen"
                auto_focus=true
                max_height_class="max-h-64"
                on_pick=on_pick
                on_rows_change=on_rows_change
                render_leading=leading
            />
        </Modal>
    }
}
