//! Laufender Entpack-Vorgang der Client-Archive. Liegt global statt in der
//! Komponente, damit das Entpacken im Hintergrund weiterläuft, wenn man das
//! Dashboard/die Einstellungen verlässt, und der Fortschritt beim
//! Zurückkommen wieder angezeigt wird.

use crate::components::toast;
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::cell::Cell;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct UnpackResult {
    pub name: String,
    pub files_copied: usize,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Progress {
    pub current: usize,
    pub total: usize,
    pub name: String,
}

#[derive(Clone, Copy)]
pub struct ClientUnpackStore {
    pub running: RwSignal<bool>,
    pub progress: RwSignal<Option<Progress>>,
    pub last_line: RwSignal<String>,
    pub results: RwSignal<Option<Vec<UnpackResult>>>,
    /// Wird nach jedem abgeschlossenen Lauf hochgezählt - Ansichten laden
    /// daraufhin ihren Entpack-Status neu.
    pub finished: RwSignal<u32>,
}

static STORE: OnceLock<ClientUnpackStore> = OnceLock::new();

thread_local! {
    static LISTENING: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn init() {
    let _ = STORE.set(ClientUnpackStore {
        running: RwSignal::new(false),
        progress: RwSignal::new(None),
        last_line: RwSignal::new(String::new()),
        results: RwSignal::new(None),
        finished: RwSignal::new(0),
    });
}

pub fn client_unpack_store() -> ClientUnpackStore {
    *STORE.get().expect("store::init() nicht aufgerufen")
}

/// Fortschritts-Events einmalig für die ganze Laufzeit abonnieren.
fn ensure_listening(store: ClientUnpackStore) {
    if LISTENING.with(|l| l.replace(true)) {
        return;
    }
    spawn_local(async move {
        let _ = crate::tauri::listen::<Progress>("client-unpack-progress", move |p| store.progress.set(Some(p))).await;
    });
    spawn_local(async move {
        let _ = crate::tauri::listen::<String>("client-unpack-output", move |l| {
            if !l.trim().is_empty() {
                store.last_line.set(l);
            }
        })
        .await;
    });
}

impl ClientUnpackStore {
    /// Startet das Entpacken im Hintergrund. `client_path`: `None` =
    /// gespeicherter Client-Ordner. Läuft schon ein Vorgang, passiert nichts.
    pub fn start(&self, client_path: Option<String>, names: Vec<String>) {
        if names.is_empty() || self.running.get_untracked() {
            return;
        }
        let store = *self;
        ensure_listening(store);
        store.running.set(true);
        store.progress.set(None);
        store.last_line.set(String::new());
        store.results.set(None);
        spawn_local(async move {
            let res = invoke::<Vec<UnpackResult>>(
                "unpack_client_archives",
                &json!({ "clientPath": client_path, "names": names }),
            )
            .await;
            store.running.set(false);
            store.progress.set(None);
            match res {
                Ok(r) => {
                    let failed = r.iter().filter(|x| x.error.is_some()).count();
                    if failed == 0 {
                        toast::success("Client-Dateien entpackt");
                    } else {
                        toast::error(&format!("{failed} Archiv(e) konnten nicht entpackt werden"));
                    }
                    store.results.set(Some(r));
                }
                Err(e) => toast::error(&e),
            }
            store.finished.update(|n| *n += 1);
        });
    }
}
