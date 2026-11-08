use leptos::prelude::*;

/// Pulsierender Platzhalter für Lade-Zustände.
#[component]
pub fn Skeleton(#[prop(into, optional)] class: MaybeProp<String>) -> impl IntoView {
    view! { <div data-slot="skeleton" class=move || format!("animate-pulse rounded-md bg-muted {}", class.get().unwrap_or_default())></div> }
}

/// Kleiner Lade-Kreisel (ersetzt `<Loader2 className="animate-spin" />`-Varianten).
#[component]
pub fn Spinner(#[prop(into, optional)] class: MaybeProp<String>) -> impl IntoView {
    view! { <super::Icon name="Loader2" class=Signal::derive(move || format!("animate-spin {}", class.get().unwrap_or_else(|| "size-4".into())))/> }
}
