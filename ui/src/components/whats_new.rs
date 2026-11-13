//! Zeigt die Release-Notes einmal direkt nach dem ersten Start auf einer
//! neuen Version (nicht zu verwechseln mit dem kleinen "Update
//! verfügbar"-Punkt, der *vor* dem Installieren erscheint).
//!
//! Die Notizen kommen aus dem beim Bauen eingebetteten `CHANGELOG.md`
//! (`util::release_notes`), funktionieren also auch offline und auch nach
//! einer manuellen Installation. Wurden Versionen übersprungen, stehen alle
//! Änderungen seit der zuletzt gesehenen Version im Dialog.
//!
//! Der Updater kennt kein "wir wurden gerade aktualisiert"-Ereignis, darum
//! wird es aus der zuletzt gezeigten Version (localStorage) und der laufenden
//! App-Version rekonstruiert. Bei einer Neuinstallation (noch nie etwas
//! gemerkt) wird nichts gezeigt, nur die Version gemerkt.

use super::markdown::{Markdown, COMPACT_STYLE};
use super::{Button, ButtonSize, ButtonVariant, Icon};
use crate::store::update::read_pending_update_notes;
use crate::tauri::app_version;
use crate::util::changelog::version_key;
use crate::util::{release_notes, storage};
use leptos::prelude::*;
use leptos::task::spawn_local;

const LAST_SHOWN_KEY: &str = "m2manager-whats-new-last-shown-version";

/// Was beim Start passieren soll.
#[derive(Debug, PartialEq, Eq)]
pub enum StartAction {
    /// Nichts tun (diese Version wurde schon gezeigt).
    Nothing,
    /// Nur die Version merken (Neuinstallation oder Downgrade).
    Remember,
    /// Dialog zeigen mit allen Änderungen nach `since` (exklusiv) bis zur
    /// laufenden Version; `None` = nur die laufende Version.
    Show { since: Option<String> },
}

/// Entscheidet anhand der zuletzt gezeigten Version und der Version des
/// zuletzt gefundenen Updates (`pending`), ob "Was ist neu" erscheint.
pub fn start_action(current: &str, last_shown: Option<&str>, pending: Option<&str>) -> StartAction {
    let newer = |v: &str| version_key(v) < version_key(current);
    match (last_shown, pending) {
        (Some(v), _) if v == current => StartAction::Nothing,
        (Some(v), _) if newer(v) => StartAction::Show { since: Some(v.to_string()) },
        (Some(_), _) => StartAction::Remember,
        // Älteres Gerät ohne gemerkte Version, aber mit einem früher
        // gefundenen Update: das ist ein Update, keine Neuinstallation.
        (None, Some(p)) if p == current => StartAction::Show { since: None },
        (None, Some(p)) if newer(p) => StartAction::Show { since: Some(p.to_string()) },
        (None, _) => StartAction::Remember,
    }
}

#[component]
pub fn WhatsNewDialog() -> impl IntoView {
    let version = RwSignal::new(None::<String>);
    let notes = RwSignal::new(None::<String>);

    spawn_local(async move {
        let Ok(current) = app_version().await else { return };
        let pending = read_pending_update_notes();
        let last_shown = storage::get(LAST_SHOWN_KEY);
        match start_action(&current, last_shown.as_deref(), pending.as_ref().map(|p| p.version.as_str())) {
            StartAction::Nothing => {}
            StartAction::Remember => storage::set(LAST_SHOWN_KEY, &current),
            StartAction::Show { since } => {
                // Eingebetteter Changelog zuerst, die Notizen aus latest.json
                // nur als Rückfall (gleiche Quelle, aber evtl. ältere Builds).
                let fallback = pending.filter(|p| p.version == current).and_then(|p| p.notes);
                notes.set(release_notes::notes_since(since.as_deref(), &current).or(fallback));
                version.set(Some(current));
            }
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
                <div class="flex max-h-[80vh] w-[40rem] max-w-[90vw] flex-col rounded-lg border border-border bg-card">
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
    use super::{start_action, StartAction};

    #[test]
    fn decides_when_to_show() {
        let show = |s: Option<&str>| StartAction::Show { since: s.map(str::to_string) };
        assert_eq!(start_action("1.2.0", Some("1.2.0"), None), StartAction::Nothing);
        assert_eq!(start_action("1.2.0", Some("1.1.0"), None), show(Some("1.1.0")));
        assert_eq!(start_action("1.10.0", Some("1.9.0"), None), show(Some("1.9.0")));
        assert_eq!(start_action("1.2.0", Some("1.3.0"), None), StartAction::Remember);
        assert_eq!(start_action("1.2.0", None, Some("1.2.0")), show(None));
        assert_eq!(start_action("1.2.0", None, Some("1.0.0")), show(Some("1.0.0")));
        assert_eq!(start_action("1.2.0", None, Some("1.3.0")), StartAction::Remember);
        assert_eq!(start_action("1.2.0", None, None), StartAction::Remember);
    }
}
