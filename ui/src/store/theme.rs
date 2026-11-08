//! Hell/Dunkel/System und Oberflächen-Zoom (Strg +/-/0).

use crate::util::storage;
use leptos::prelude::*;
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl Theme {
    pub fn id(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
            Theme::System => "system",
        }
    }
    fn parse(s: &str) -> Theme {
        match s {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => Theme::System,
        }
    }
}

pub const UI_SCALES: [u32; 5] = [90, 100, 110, 125, 150];
const STORAGE_KEY: &str = "m2manager-theme";
const SCALE_KEY: &str = "m2manager-ui-scale";

#[derive(Clone, Copy)]
pub struct ThemeStore {
    pub theme: RwSignal<Theme>,
    pub scale: RwSignal<u32>,
}

static STORE: OnceLock<ThemeStore> = OnceLock::new();

fn system_dark() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
        .map(|m| m.matches())
        .unwrap_or(false)
}

fn apply_theme(theme: Theme) {
    let dark = theme == Theme::Dark || (theme == Theme::System && system_dark());
    if let Some(el) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.document_element()) {
        let _ = el.class_list().toggle_with_force("dark", dark);
    }
}

fn apply_scale(scale: u32) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = el.style().set_property("font-size", &format!("{scale}%"));
    }
}

fn clamp_scale(s: u32) -> u32 {
    s.clamp(UI_SCALES[0], UI_SCALES[UI_SCALES.len() - 1])
}

pub(super) fn init() {
    let theme = storage::get(STORAGE_KEY).map(|s| Theme::parse(&s)).unwrap_or(Theme::System);
    apply_theme(theme);
    let scale = storage::get(SCALE_KEY).and_then(|s| s.parse().ok()).filter(|s| *s > 0).map(clamp_scale).unwrap_or(100);
    apply_scale(scale);
    let store = ThemeStore { theme: RwSignal::new(theme), scale: RwSignal::new(scale) };
    let _ = STORE.set(store);

    // Strg + / Strg - / Strg 0 wie im Browser.
    let keydown = Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        if !(e.ctrl_key() || e.meta_key()) || e.alt_key() {
            return;
        }
        let st = theme_store();
        let scale = st.scale.get_untracked();
        let idx = UI_SCALES.iter().position(|s| *s >= scale).unwrap_or(UI_SCALES.len() - 1);
        match e.key().as_str() {
            "+" | "=" => st.set_scale(UI_SCALES[(idx + 1).min(UI_SCALES.len() - 1)]),
            "-" => st.set_scale(UI_SCALES[idx.saturating_sub(1)]),
            "0" => st.set_scale(100),
            _ => return,
        }
        e.prevent_default();
    });
    if let Some(w) = web_sys::window() {
        let _ = w.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref());
        if let Ok(Some(mq)) = w.match_media("(prefers-color-scheme: dark)") {
            let change = Closure::<dyn Fn()>::new(|| {
                if theme_store().theme.get_untracked() == Theme::System {
                    apply_theme(Theme::System);
                }
            });
            let _ = mq.add_event_listener_with_callback("change", change.as_ref().unchecked_ref());
            change.forget();
        }
    }
    keydown.forget();
}

pub fn theme_store() -> ThemeStore {
    *STORE.get().expect("store::init() nicht aufgerufen")
}

impl ThemeStore {
    pub fn set_theme(&self, t: Theme) {
        storage::set(STORAGE_KEY, t.id());
        apply_theme(t);
        self.theme.set(t);
    }
    pub fn set_scale(&self, s: u32) {
        let v = clamp_scale(s);
        storage::set(SCALE_KEY, &v.to_string());
        apply_scale(v);
        self.scale.set(v);
    }
}
