//! Gemeinsamer Dialog: Overlay, `role="dialog"`/`aria-modal`, Beschriftung über
//! die Überschrift, Escape zum Schließen (wenn `on_close` gesetzt), Fokus beim
//! Öffnen in den Dialog, Fokus-Fang (Tab/Umschalt+Tab bleiben im Dialog) und
//! Fokus zurück an die vorherige Stelle beim Schließen.

// Allgemeine Helfer-API: nicht jede Komponente wird aktuell genutzt.
#![allow(dead_code)]

use super::button::{Button, ButtonVariant};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, KeyboardEvent};

/// Fokussierbare Elemente innerhalb des Dialogs.
const FOCUSABLE: &str = "a[href], button:not([disabled]), input:not([disabled]):not([type=hidden]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex=\"-1\"])";

const DEFAULT_PANEL: &str = "w-96 space-y-3 rounded-lg border border-border bg-card p-4";

thread_local! {
    static NEXT_ID: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

fn focusables(root: &HtmlElement) -> Vec<HtmlElement> {
    let Ok(list) = root.query_selector_all(FOCUSABLE) else {
        return vec![];
    };
    (0..list.length())
        .filter_map(|i| list.item(i))
        .filter_map(|n| n.dyn_into::<HtmlElement>().ok())
        // Unsichtbare (display:none) Elemente haben keine Layout-Box.
        .filter(|e| e.offset_parent().is_some() || e.offset_width() > 0)
        .collect()
}

fn active_element() -> Option<HtmlElement> {
    web_sys::window()?.document()?.active_element()?.dyn_into::<HtmlElement>().ok()
}

/// Dialog mit Overlay. `class` sind die Klassen des Dialogfensters (Breite,
/// Abstände ...), `overlay_class` zusätzliche Overlay-Klassen (z.B. `z-50`).
/// Ohne `on_close` schließt Escape nicht - der Dialog wird dann über seine
/// eigenen Schaltflächen beendet.
#[component]
pub fn Modal(
    #[prop(optional, into)] class: String,
    #[prop(optional, into)] overlay_class: String,
    #[prop(optional, into)] on_close: Option<Callback<()>>,
    /// Id des Titel-Elements; ohne Angabe wird die erste Überschrift
    /// (`h1`-`h3` bzw. `p.font-medium`) im Dialog verwendet.
    #[prop(optional)] labelled_by: &'static str,
    /// Klick auf den abgedunkelten Hintergrund ruft `on_close` auf.
    #[prop(optional)] close_on_backdrop: bool,
    children: Children,
) -> impl IntoView {
    let panel = NodeRef::<leptos::html::Div>::new();
    let previous = StoredValue::new_local(None::<HtmlElement>);
    let class = if class.is_empty() { DEFAULT_PANEL.to_string() } else { class };

    Effect::new(move |_| {
        let Some(el) = panel.get() else { return };
        let el: HtmlElement = el.into();
        previous.set_value(active_element().filter(|a| !el.contains(Some(a))));
        if labelled_by.is_empty() {
            if let Ok(Some(title)) = el.query_selector("h1, h2, h3, p.font-medium") {
                if title.id().is_empty() {
                    let n = NEXT_ID.with(|c| {
                        c.set(c.get() + 1);
                        c.get()
                    });
                    title.set_id(&format!("modal-title-{n}"));
                }
                let _ = el.set_attribute("aria-labelledby", &title.id());
            }
        }
        // Nach dem Aufbau fokussieren, damit ein Feld, das selbst den Fokus
        // anfordert (`focus_on_mount`), Vorrang behält.
        set_timeout(
            move || {
                if active_element().is_some_and(|a| el.contains(Some(&a)) && a != el) {
                    return;
                }
                match focusables(&el).first() {
                    Some(first) => {
                        let _ = first.focus();
                    }
                    None => {
                        let _ = el.focus();
                    }
                }
            },
            std::time::Duration::from_millis(0),
        );
    });

    on_cleanup(move || {
        if let Some(prev) = previous.try_get_value().flatten() {
            if prev.is_connected() {
                let _ = prev.focus();
            }
        }
    });

    let on_keydown = move |e: KeyboardEvent| match e.key().as_str() {
        "Escape" => {
            if let Some(cb) = on_close {
                e.prevent_default();
                e.stop_propagation();
                cb.run(());
            }
        }
        "Tab" => {
            let Some(el) = panel.get_untracked() else { return };
            let el: HtmlElement = el.into();
            // Der innerste Dialog behandelt Tab allein (bei verschachtelten Dialogen).
            e.stop_propagation();
            let items = focusables(&el);
            let (Some(first), Some(last)) = (items.first(), items.last()) else {
                e.prevent_default();
                let _ = el.focus();
                return;
            };
            let active = active_element();
            let inside = active.as_ref().is_some_and(|a| el.contains(Some(a)));
            let at = |x: &HtmlElement| active.as_ref() == Some(x);
            if !inside {
                e.prevent_default();
                let _ = first.focus();
            } else if e.shift_key() && (at(first) || at(&el)) {
                e.prevent_default();
                let _ = last.focus();
            } else if !e.shift_key() && at(last) {
                e.prevent_default();
                let _ = first.focus();
            }
        }
        _ => {}
    };

    view! {
        <div
            class=format!("fixed inset-0 flex items-center justify-center bg-black/50 {overlay_class}")
            on:keydown=on_keydown
            on:click=move |e| {
                if close_on_backdrop && e.target() == e.current_target() {
                    if let Some(cb) = on_close {
                        cb.run(());
                    }
                }
            }
        >
            <div
                node_ref=panel
                role="dialog"
                aria-modal="true"
                aria-labelledby=(!labelled_by.is_empty()).then_some(labelled_by)
                tabindex="-1"
                class=format!("{class} outline-none")
            >
                {children()}
            </div>
        </div>
    }
}

/// Bestätigungsdialog (Titel, Text, Abbrechen/Bestätigen) auf Basis von [`Modal`].
#[component]
pub fn ConfirmDialog(
    #[prop(into)] title: String,
    #[prop(optional, into)] confirm_label: Option<String>,
    #[prop(optional)] danger: bool,
    #[prop(into)] on_confirm: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
    children: Children,
) -> impl IntoView {
    let label = confirm_label.unwrap_or_else(|| "Bestätigen".into());
    let variant = if danger { ButtonVariant::Destructive } else { ButtonVariant::Default };
    view! {
        <Modal on_close=on_cancel>
            <p class="text-sm font-medium">{title}</p>
            <div class="text-sm text-muted-foreground">{children()}</div>
            <div class="flex justify-end gap-2">
                <Button variant=ButtonVariant::Outline on_click=move |_| on_cancel.run(())>"Abbrechen"</Button>
                <Button variant=variant on_click=move |_| on_confirm.run(())>{label}</Button>
            </div>
        </Modal>
    }
}
