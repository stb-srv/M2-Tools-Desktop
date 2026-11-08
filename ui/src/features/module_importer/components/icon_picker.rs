use crate::features::module_importer::shared::file_name_of;
use leptos::prelude::*;

#[component]
pub fn IconPicker(
    pool: Vec<String>,
    #[prop(into)] value: Signal<Option<String>>,
    #[prop(into)] on_change: Callback<Option<String>>,
) -> impl IntoView {
    view! {
        <select
            on:change=move |e| {
                let v = event_target_value(&e);
                on_change.run(if v.is_empty() { None } else { Some(v) })
            }
            class="max-w-[12rem] rounded-md border border-border bg-background px-2 py-1 text-xs"
        >
            <option value="" prop:selected=move || value.with(|v| v.is_none())>"— kein Icon —"</option>
            {pool.into_iter().map(|p| {
                let label = file_name_of(&p);
                let p2 = p.clone();
                view! {
                    <option value=p prop:selected=move || value.with(|v| v.as_deref() == Some(p2.as_str()))>{label}</option>
                }
            }).collect_view()}
        </select>
    }
}
