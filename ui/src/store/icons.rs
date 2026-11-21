//! Gemeinsamer Item-Icon-Cache. Alle Module, die Item-Icons anzeigen, lesen
//! aus derselben Map und laden fehlende Icons über [`ensure_icons`] nach;
//! laufende Abfragen werden dedupliziert. Bei einem Serverwechsel
//! (`connection_profiles().revision`) wird der Cache geleert.

use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// Item-Icons nach vnum: `None` = kein Icon/Fehler, fehlt der Schlüssel =
/// noch nicht geladen.
pub type IconMap = HashMap<i64, Option<String>>;

static CACHE: OnceLock<RwSignal<IconMap>> = OnceLock::new();

thread_local! {
    /// vnums mit laufender Abfrage.
    static INFLIGHT: RefCell<HashSet<i64>> = RefCell::new(HashSet::new());
    /// Zählt bei jedem Leeren hoch; späte Antworten alter Läufe werden verworfen.
    static GENERATION: Cell<u64> = const { Cell::new(0) };
}

pub(super) fn init() {
    let _ = CACHE.set(RwSignal::new(IconMap::new()));
}

/// Der geteilte Cache (reaktiv lesbar).
pub fn icons() -> RwSignal<IconMap> {
    *CACHE.get().expect("store::init() nicht aufgerufen")
}

/// Welche vnums müssen noch abgefragt werden (ohne Duplikate, Reihenfolge der Eingabe)?
fn missing(cache: &IconMap, inflight: &HashSet<i64>, vnums: impl IntoIterator<Item = i64>) -> Vec<i64> {
    let mut seen = HashSet::new();
    vnums.into_iter().filter(|v| !cache.contains_key(v) && !inflight.contains(v) && seen.insert(*v)).collect()
}

/// Lädt fehlende Item-Icons nach; bereits geladene oder laufende werden übersprungen.
pub fn ensure_icons(vnums: impl IntoIterator<Item = i64>) {
    let cache = icons();
    let todo = cache.with_untracked(|c| INFLIGHT.with(|f| missing(c, &f.borrow(), vnums)));
    for vnum in todo {
        INFLIGHT.with(|f| f.borrow_mut().insert(vnum));
        let generation = GENERATION.with(Cell::get);
        spawn_local(async move {
            let res = fetch_icon(vnum).await;
            if GENERATION.with(Cell::get) != generation {
                return;
            }
            INFLIGHT.with(|f| f.borrow_mut().remove(&vnum));
            cache.update(|m| {
                m.insert(vnum, res);
            });
        });
    }
}

/// Fragt ein Icon frisch ab (ohne Cache), `None` bei Fehler/ohne Icon.
pub async fn fetch_icon(vnum: i64) -> Option<String> {
    invoke::<Option<String>>("get_item_icon", &json!({ "vnum": vnum })).await.ok().flatten()
}

/// Leert den gesamten Cache (Serverwechsel).
pub fn clear() {
    GENERATION.with(|g| g.set(g.get() + 1));
    INFLIGHT.with(|f| f.borrow_mut().clear());
    icons().update(|m| m.clear());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_skips_cached_inflight_and_duplicates() {
        let mut cache = IconMap::new();
        cache.insert(1, None);
        let inflight: HashSet<i64> = [2].into_iter().collect();
        assert_eq!(missing(&cache, &inflight, [1, 2, 3, 3, 4]), vec![3, 4]);
    }
}
