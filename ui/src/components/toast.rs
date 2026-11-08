//! Kurze Rückmeldungen (gespeichert/gelöscht/Fehler) unten rechts.
//! Aufruf von überall, auch aus async-Code: `toast::success("Gespeichert")`.

use super::Icon;
use leptos::prelude::*;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastVariant {
    Success,
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToastItem {
    pub id: u64,
    pub variant: ToastVariant,
    pub title: Option<String>,
    pub description: String,
}

#[derive(Clone, Copy)]
struct ToastStore {
    items: RwSignal<Vec<ToastItem>>,
    next: RwSignal<u64>,
}

static STORE: OnceLock<ToastStore> = OnceLock::new();

pub fn init() {
    let _ = STORE.set(ToastStore { items: RwSignal::new(vec![]), next: RwSignal::new(1) });
}

fn store() -> ToastStore {
    *STORE.get().expect("toast::init() nicht aufgerufen")
}

const DEFAULT_TIMEOUT_MS: u64 = 5000;

pub fn show(variant: ToastVariant, description: &str, title: Option<&str>, timeout_ms: Option<u64>) {
    let s = store();
    let id = s.next.get_untracked();
    s.next.set(id + 1);
    s.items.update(|v| {
        v.push(ToastItem { id, variant, title: title.map(str::to_string), description: description.to_string() })
    });
    let ms = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    if ms > 0 {
        set_timeout(move || dismiss(id), std::time::Duration::from_millis(ms));
    }
}

pub fn dismiss(id: u64) {
    store().items.update(|v| v.retain(|t| t.id != id));
}

pub fn success(msg: &str) {
    show(ToastVariant::Success, msg, None, None)
}
pub fn error(msg: &str) {
    show(ToastVariant::Error, msg, None, None)
}
pub fn warning(msg: &str) {
    show(ToastVariant::Warning, msg, None, None)
}
pub fn info(msg: &str) {
    show(ToastVariant::Info, msg, None, None)
}

fn variant_icon(v: ToastVariant) -> (&'static str, &'static str, &'static str) {
    match v {
        ToastVariant::Success => ("CheckCircle2", "border-success/30", "text-success"),
        ToastVariant::Error => ("XCircle", "border-destructive/30", "text-destructive"),
        ToastVariant::Warning => ("AlertTriangle", "border-warning/30", "text-warning"),
        ToastVariant::Info => ("Info", "border-border", "text-muted-foreground"),
    }
}

/// Einmal an der App-Wurzel einhängen.
#[component]
pub fn Toaster() -> impl IntoView {
    let s = store();
    view! {
        <div class="fixed bottom-4 right-4 z-50 flex w-96 max-w-[calc(100vw-2rem)] flex-col-reverse gap-2 outline-none">
            <For each=move || s.items.get() key=|t| t.id let:t>
                {
                    let (icon, border, color) = variant_icon(t.variant);
                    let id = t.id;
                    view! {
                        <div class=format!("pointer-events-auto flex w-full items-start gap-2 rounded-lg border bg-card p-3 text-card-foreground shadow-lg transition-all duration-200 {border}")>
                            <Icon name=icon class=format!("mt-0.5 size-4 shrink-0 {color}")/>
                            <div class="min-w-0 flex-1">
                                {t.title.clone().map(|ti| view! { <div class="text-sm font-medium">{ti}</div> })}
                                <div class="text-sm text-muted-foreground whitespace-pre-wrap break-words">{t.description.clone()}</div>
                            </div>
                            <button
                                aria-label="Schließen"
                                on:click=move |_| dismiss(id)
                                class="shrink-0 rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
                            >
                                <Icon name="X" class="size-3.5"/>
                            </button>
                        </div>
                    }
                }
            </For>
        </div>
    }
}
