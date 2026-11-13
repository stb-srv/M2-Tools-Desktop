//! „Server jetzt neu starten“ - für Dateien, die der Server nur beim Start
//! liest (z.B. common_drop_item.txt). Nutzt die Befehle der Server-Steuerung.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::server_control::restart_server_with_saved_commands;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Clone, PartialEq)]
enum State {
    Idle,
    Confirm,
    Running,
    Done,
    Failed(String),
}

#[component]
pub fn RestartServerButton() -> impl IntoView {
    let state = RwSignal::new(State::Idle);
    let run = move || {
        state.set(State::Running);
        spawn_local(async move {
            match restart_server_with_saved_commands().await {
                Ok(()) => state.set(State::Done),
                Err(e) => state.set(State::Failed(e)),
            }
        });
    };
    view! {
        <span class="inline-flex flex-wrap items-center gap-2">
            {move || match state.get() {
                State::Idle => view! {
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| state.set(State::Confirm)>
                        <Icon name="RotateCcw" class="size-3.5"/>
                        "Server jetzt neu starten"
                    </Button>
                }.into_any(),
                State::Confirm => view! {
                    <span class="text-sm">"Server stoppen und wieder starten? Spieler werden getrennt."</span>
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Destructive on_click=move |_| run()>"Ja, neu starten"</Button>
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost on_click=move |_| state.set(State::Idle)>"Abbrechen"</Button>
                }.into_any(),
                State::Running => view! {
                    <span class="flex items-center gap-1 text-sm text-muted-foreground">
                        <Icon name="Loader2" class="size-3.5 animate-spin"/>
                        "Server startet neu…"
                    </span>
                }.into_any(),
                State::Done => view! {
                    <span class="flex items-center gap-1 text-sm text-green-600">
                        <Icon name="CheckCircle2" class="size-3.5"/>
                        "Server neu gestartet."
                    </span>
                }.into_any(),
                State::Failed(e) => view! {
                    <span class="text-sm text-destructive">{e}</span>
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost on_click=move |_| state.set(State::Idle)>"OK"</Button>
                }.into_any(),
            }}
        </span>
    }
}
