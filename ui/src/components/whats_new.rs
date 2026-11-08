//! Zeigt die echten Release-Notes einmal direkt nach dem Neustart auf einer
//! frisch installierten Version (nicht zu verwechseln mit dem kleinen
//! "Update verfügbar"-Punkt, der *vor* dem Installieren erscheint).
//!
//! Der Updater kennt kein "wir wurden gerade aktualisiert"-Ereignis, darum
//! wird es aus zwei lokalen Zuständen rekonstruiert: den zuletzt gemerkten
//! Notizen (`read_pending_update_notes`) und der laufenden App-Version. Passen
//! sie zusammen und wurde diese Version noch nicht gezeigt, ist das der erste
//! Start auf ihr.

use super::markdown::{Markdown, COMPACT_STYLE};
use super::{Button, ButtonSize, ButtonVariant, Icon};
use crate::util::storage;
use crate::store::update::read_pending_update_notes;
use crate::tauri::app_version;
use leptos::prelude::*;
use leptos::task::spawn_local;

const LAST_SHOWN_KEY: &str = "m2manager-whats-new-last-shown-version";

/// Welche Notizen (falls überhaupt) beim Start zu zeigen sind.
/// Rückgabe: `Some((version, notes))`.
pub fn notes_to_show(
    current_version: &str,
    pending: Option<(String, Option<String>)>,
    last_shown: Option<&str>,
) -> Option<(String, Option<String>)> {
    let (version, notes) = pending?;
    if version != current_version || last_shown == Some(current_version) {
        return None;
    }
    Some((current_version.to_string(), notes))
}

#[component]
pub fn WhatsNewDialog() -> impl IntoView {
    let version = RwSignal::new(None::<String>);
    let notes = RwSignal::new(None::<String>);

    spawn_local(async move {
        let Ok(current) = app_version().await else { return };
        let pending = read_pending_update_notes().map(|p| (p.version, p.notes));
        let last_shown = storage::get(LAST_SHOWN_KEY);
        if let Some((v, n)) = notes_to_show(&current, pending, last_shown.as_deref()) {
            notes.set(n);
            version.set(Some(v));
        }
    });

    let dismiss = move || {
        if let Some(v) = version.get_untracked() {
            storage::set(LAST_SHOWN_KEY, &v);
        }
        version.set(None);
        notes.set(None);
    };

    view! {
        <Show when=move || version.with(|v| v.is_some())>
            <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                <div class="flex max-h-[80vh] w-[32rem] flex-col rounded-lg border border-border bg-card">
                    <div class="flex items-center justify-between border-b border-border px-4 py-3">
                        <div class="flex items-center gap-2">
                            <Icon name="Sparkles" class="size-4 text-primary"/>
                            <h2 class="text-sm font-semibold">
                                {move || format!("Was ist neu in v{}", version.get().unwrap_or_default())}
                            </h2>
                        </div>
                        <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm on_click=move |_| dismiss() title="Schließen">
                            <Icon name="X" class="size-4"/>
                        </Button>
                    </div>
                    <div class="overflow-y-auto p-4">
                        {move || match notes.get().filter(|n| !n.is_empty()) {
                            Some(n) => view! { <Markdown content=n style=COMPACT_STYLE/> }.into_any(),
                            None => view! {
                                <p class="text-sm text-muted-foreground">"Keine Release-Notes für diese Version verfügbar."</p>
                            }
                            .into_any(),
                        }}
                    </div>
                    <div class="flex justify-end border-t border-border px-4 py-3">
                        <Button on_click=move |_| dismiss()>"Verstanden"</Button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::notes_to_show;

    #[test]
    fn shows_once_for_matching_version() {
        let pending = Some(("1.2.0".to_string(), Some("# Neu".to_string())));
        assert_eq!(notes_to_show("1.2.0", pending.clone(), None), Some(("1.2.0".into(), Some("# Neu".into()))));
        assert_eq!(notes_to_show("1.2.0", pending.clone(), Some("1.2.0")), None);
        assert_eq!(notes_to_show("1.3.0", pending, None), None);
        assert_eq!(notes_to_show("1.2.0", None, None), None);
    }
}
