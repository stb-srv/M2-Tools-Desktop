//! Tab-Leiste für Bereiche, die mehrere früher eigenständige Module bündeln.
//! Jeder Tab bleibt nach dem ersten Öffnen gemountet und wird nur versteckt,
//! damit beim Wechsel keine ungespeicherten Eingaben verloren gehen.

use leptos::prelude::*;

pub struct SectionTab {
    pub id: &'static str,
    pub label: Signal<String>,
    pub content: ViewFn,
}

impl SectionTab {
    pub fn new<V: IntoView + 'static>(
        id: &'static str,
        label: impl Fn() -> String + Send + Sync + 'static,
        content: impl Fn() -> V + Send + Sync + 'static,
    ) -> Self {
        Self { id, label: Signal::derive(label), content: ViewFn::from(content) }
    }
}

#[component]
pub fn SectionTabs(tabs: Vec<SectionTab>) -> impl IntoView {
    let first = tabs.first().map(|t| t.id).unwrap_or_default();
    let active = RwSignal::new(first);
    let visited = RwSignal::new(vec![first]);
    Effect::new(move |_| {
        let a = active.get();
        if !visited.with_untracked(|v| v.contains(&a)) {
            visited.update(|v| v.push(a));
        }
    });
    let headers = tabs
        .iter()
        .map(|t| {
            let id = t.id;
            let label = t.label;
            view! {
                <button
                    on:click=move |_| active.set(id)
                    class=move || {
                        if active.get() == id {
                            "rounded-t-md px-3 py-2 text-sm font-medium transition-colors border-b-2 border-primary text-foreground"
                        } else {
                            "rounded-t-md px-3 py-2 text-sm font-medium transition-colors text-muted-foreground hover:text-foreground"
                        }
                    }
                >
                    {move || label.get()}
                </button>
            }
        })
        .collect_view();
    let bodies = tabs
        .into_iter()
        .map(|t| {
            let id = t.id;
            let content = t.content;
            view! {
                <Show when=move || visited.with(|v| v.contains(&id))>
                    <div class=move || if active.get() == id { "h-full" } else { "h-full hidden" }>
                        {content.run()}
                    </div>
                </Show>
            }
        })
        .collect_view();
    view! {
        <div class="flex h-full flex-col gap-4">
            <div class="flex flex-wrap gap-1 border-b border-border">{headers}</div>
            <div class="min-h-0 flex-1">{bodies}</div>
        </div>
    }
}
