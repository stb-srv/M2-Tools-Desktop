use super::{Button, ButtonSize, ButtonVariant, Icon};
use crate::store::navigation::ModuleId;
use leptos::prelude::*;

/// Einheitlicher Seitenkopf: Symbol des Bereichs, Titel, Hilfe-Knopf und
/// optional eigene Aktionen rechts. Eingebettete Module (Tabs) bekommen eine
/// kleinere Variante ohne Symbol.
#[component]
pub fn PageHeader(
    #[prop(into)] module: ModuleId,
    #[prop(into)] title: Signal<String>,
    #[prop(optional)] actions: Option<ChildrenFn>,
    #[prop(into, optional)] class: MaybeProp<String>,
) -> impl IntoView {
    let embedded = module.is_embedded();
    let icon = module.host().icon();
    view! {
        <div class=move || format!("flex flex-wrap items-center justify-between gap-3 {}", class.get().unwrap_or_default())>
            <div class="flex min-w-0 items-center gap-3">
                {(!embedded).then(|| view! {
                    <div class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-primary text-primary-foreground shadow-sm">
                        <Icon name=icon class="size-5"/>
                    </div>
                })}
                <h1 class=if embedded { "min-w-0 font-semibold text-lg" } else { "min-w-0 font-semibold text-2xl" }>
                    {move || title.get()}
                </h1>
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::IconSm
                    title="Hilfe zu diesem Modul"
                    on_click=move |_| crate::lib::open_manual(Some(module))
                >
                    <Icon name="HelpCircle" class="size-4"/>
                </Button>
            </div>
            {actions.map(|a| view! { <div class="flex flex-wrap items-center gap-2">{a()}</div> })}
        </div>
    }
}
