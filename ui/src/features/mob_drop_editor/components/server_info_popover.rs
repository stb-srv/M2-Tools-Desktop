use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::mob_drop_editor::server_notes::SERVER_NOTES;
use leptos::prelude::*;

/// Eigenständiger Auslöser + Popover ("Wie Prozentwerte auf diesem Server
/// wirken") - braucht keinen Zustand außerhalb dieser Komponente.
#[component]
pub fn ServerInfoPopover() -> impl IntoView {
    let open = RwSignal::new(false);
    view! {
        <div class="relative">
            <Button
                variant=ButtonVariant::Ghost
                size=ButtonSize::IconSm
                on_click=move |_| open.update(|v| *v = !*v)
                title="Wie Prozentwerte auf diesem Server wirken"
            >
                <Icon name="Info" class="size-4"/>
            </Button>
            <Show when=move || open.get()>
                <div class="absolute left-0 top-full z-10 mt-1 w-96 space-y-3 rounded-lg border border-border bg-card p-3 shadow-lg">
                    {SERVER_NOTES
                        .iter()
                        .map(|note| {
                            view! {
                                <div class="space-y-1.5">
                                    <p class="text-sm font-semibold">{note.title}</p>
                                    {note.intro.map(|i| view! { <p class="text-xs text-muted-foreground">{i}</p> })}
                                    {note.formula.map(|f| view! {
                                        <p class="rounded-md bg-muted/60 px-2 py-1 text-xs font-medium">{f}</p>
                                    })}
                                    {note.bullets.map(|b| view! {
                                        <ul class="list-disc space-y-1 pl-4 text-xs text-muted-foreground">
                                            {b.iter().map(|x| view! { <li>{*x}</li> }).collect_view()}
                                        </ul>
                                    })}
                                </div>
                            }
                        })
                        .collect_view()}
                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| open.set(false)>
                        "Schließen"
                    </Button>
                </div>
            </Show>
        </div>
    }
}
