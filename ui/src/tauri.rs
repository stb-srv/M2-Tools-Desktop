//! Dünne Rust-Brücke zur Tauri-API im Webview.
//!
//! Tauri stellt mit `app.withGlobalTauri = true` (siehe tauri.conf.json) alle
//! Kern- und Plugin-Funktionen unter `window.__TAURI__` bereit. Hier werden sie
//! einmal zentral über `js_sys::Reflect` aufgerufen, damit der restliche Code
//! nur typisierte Rust-Funktionen sieht (`invoke`, `dialog_open`, ...).

// Allgemeine Helfer-API: nicht jede Funktion/Variante wird aktuell genutzt.
#![allow(dead_code)]

use js_sys::{Array, Function, Object, Promise, Reflect};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

fn tauri_global() -> Result<JsValue, String> {
    let window = web_sys::window().ok_or("kein window")?;
    let t = Reflect::get(&window, &"__TAURI__".into()).map_err(js_err)?;
    if t.is_undefined() {
        return Err("Tauri-API nicht verfügbar (läuft die App außerhalb von Tauri?)".into());
    }
    Ok(t)
}

fn path(parts: &[&str]) -> Result<JsValue, String> {
    let mut cur = tauri_global()?;
    for p in parts {
        cur = Reflect::get(&cur, &(*p).into()).map_err(js_err)?;
        if cur.is_undefined() {
            return Err(format!("Tauri-API `{}` fehlt", parts.join(".")));
        }
    }
    Ok(cur)
}

/// Wandelt einen JS-Fehler in einen lesbaren String um - Tauri-Kommandos
/// lehnen meist mit einem reinen String ab (das `Err(String)` aus Rust).
pub fn js_err(v: JsValue) -> String {
    if let Some(s) = v.as_string() {
        return s;
    }
    if let Some(e) = v.dyn_ref::<js_sys::Error>() {
        return String::from(e.message());
    }
    js_sys::JSON::stringify(&v)
        .ok()
        .and_then(|s| s.as_string())
        .unwrap_or_else(|| format!("{v:?}"))
}

async fn call_fn(f: &JsValue, this: &JsValue, args: &[JsValue]) -> Result<JsValue, String> {
    let f: &Function = f.dyn_ref().ok_or("keine Funktion")?;
    let arr = Array::new();
    for a in args {
        arr.push(a);
    }
    let ret = f.apply(this, &arr).map_err(js_err)?;
    if ret.has_type::<Promise>() {
        JsFuture::from(Promise::from(ret)).await.map_err(js_err)
    } else {
        Ok(ret)
    }
}

async fn call(parts: &[&str], args: &[JsValue]) -> Result<JsValue, String> {
    let (last, parent_parts) = parts.split_last().expect("leerer Pfad");
    let parent = path(parent_parts)?;
    let f = Reflect::get(&parent, &(*last).into()).map_err(js_err)?;
    call_fn(&f, &parent, args).await
}

fn to_js<T: Serialize + ?Sized>(v: &T) -> Result<JsValue, String> {
    v.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| e.to_string())
}

fn from_js<T: DeserializeOwned>(v: JsValue) -> Result<T, String> {
    serde_wasm_bindgen::from_value(v).map_err(|e| e.to_string())
}

/// Ruft ein `#[tauri::command]` im Backend auf. `args` wird wie in JS als
/// Objekt mit camelCase-Schlüsseln übergeben (Tauri wandelt in snake_case um),
/// z.B. `invoke::<Option<String>>("get_setting", &json!({ "key": "x" }))`.
pub async fn invoke<T: DeserializeOwned>(cmd: &str, args: &impl Serialize) -> Result<T, String> {
    let v = call(&["core", "invoke"], &[cmd.into(), to_js(args)?]).await?;
    from_js(v)
}

/// Wie [`invoke`], aber ohne Argumente.
pub async fn invoke0<T: DeserializeOwned>(cmd: &str) -> Result<T, String> {
    let v = call(&["core", "invoke"], &[cmd.into()]).await?;
    from_js(v)
}

/// Wie [`invoke`], ignoriert den Rückgabewert.
pub async fn invoke_unit(cmd: &str, args: &impl Serialize) -> Result<(), String> {
    call(&["core", "invoke"], &[cmd.into(), to_js(args)?]).await.map(|_| ())
}

// ---------------------------------------------------------------- Dialoge

#[derive(Serialize, Default, Clone)]
pub struct DialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

impl DialogFilter {
    pub fn new(name: &str, extensions: &[&str]) -> Self {
        Self { name: name.into(), extensions: extensions.iter().map(|s| s.to_string()).collect() }
    }
}

#[derive(Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpenDialogOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<DialogFilter>,
    pub directory: bool,
    pub multiple: bool,
}

#[derive(Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaveDialogOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<DialogFilter>,
}

/// Datei-/Ordner-Auswahl. Liefert die gewählten Pfade (leer = abgebrochen).
pub async fn dialog_open(opts: &OpenDialogOptions) -> Result<Vec<String>, String> {
    let v = call(&["dialog", "open"], &[to_js(opts)?]).await?;
    if v.is_null() || v.is_undefined() {
        return Ok(vec![]);
    }
    if let Some(s) = v.as_string() {
        return Ok(vec![s]);
    }
    from_js(v)
}

/// Einzelne Datei/Ordner wählen - `None` = abgebrochen.
pub async fn dialog_open_one(opts: &OpenDialogOptions) -> Result<Option<String>, String> {
    Ok(dialog_open(opts).await?.into_iter().next())
}

/// Speichern-unter-Dialog - `None` = abgebrochen.
pub async fn dialog_save(opts: &SaveDialogOptions) -> Result<Option<String>, String> {
    let v = call(&["dialog", "save"], &[to_js(opts)?]).await?;
    Ok(v.as_string())
}

/// Nativer Ja/Nein-Dialog (ersetzt `window.confirm`, das im Webview je nach
/// Plattform nicht blockiert).
pub async fn dialog_confirm(message: &str, title: Option<&str>) -> bool {
    let opts = Object::new();
    if let Some(t) = title {
        let _ = Reflect::set(&opts, &"title".into(), &t.into());
    }
    let _ = Reflect::set(&opts, &"kind".into(), &"warning".into());
    match call(&["dialog", "confirm"], &[message.into(), opts.into()]).await {
        Ok(v) => v.as_bool().unwrap_or(false),
        Err(_) => web_sys::window().and_then(|w| w.confirm_with_message(message).ok()).unwrap_or(false),
    }
}

/// Einfacher Hinweis-Dialog.
pub async fn dialog_message(message: &str) {
    let _ = call(&["dialog", "message"], &[message.into()]).await;
}

// ---------------------------------------------------------------- App / Prozess / Opener

pub async fn app_version() -> Result<String, String> {
    let v = call(&["app", "getVersion"], &[]).await?;
    v.as_string().ok_or_else(|| "keine Version".into())
}

pub async fn relaunch() -> Result<(), String> {
    call(&["process", "relaunch"], &[]).await.map(|_| ())
}

/// Öffnet eine URL im Standardbrowser des Systems.
pub async fn open_url(url: &str) -> Result<(), String> {
    call(&["opener", "openUrl"], &[url.into()]).await.map(|_| ())
}

/// Öffnet eine Datei/einen Ordner mit dem Standardprogramm.
pub async fn open_path(p: &str) -> Result<(), String> {
    call(&["opener", "openPath"], &[p.into()]).await.map(|_| ())
}

// ---------------------------------------------------------------- Events

/// Sendet ein Tauri-Event an alle Fenster.
pub async fn emit(event: &str, payload: &impl Serialize) -> Result<(), String> {
    call(&["event", "emit"], &[event.into(), to_js(payload)?]).await.map(|_| ())
}

/// Hört auf ein Tauri-Event. Der Rückgabewert hebt das Abo beim Drop *nicht*
/// auf - [`Unlisten::unlisten`] explizit aufrufen (z.B. in `on_cleanup`).
pub async fn listen<T: DeserializeOwned + 'static>(
    event: &str,
    handler: impl Fn(T) + 'static,
) -> Result<Unlisten, String> {
    let cb = Closure::<dyn Fn(JsValue)>::new(move |ev: JsValue| {
        let payload = Reflect::get(&ev, &"payload".into()).unwrap_or(JsValue::NULL);
        match serde_wasm_bindgen::from_value::<T>(payload) {
            Ok(p) => handler(p),
            Err(e) => web_sys::console::warn_1(&format!("Event-Payload: {e}").into()),
        }
    });
    let f = call(&["event", "listen"], &[event.into(), cb.as_ref().clone()]).await?;
    cb.forget();
    Ok(Unlisten(f))
}

pub struct Unlisten(JsValue);

impl Unlisten {
    pub fn unlisten(&self) {
        if let Some(f) = self.0.dyn_ref::<Function>() {
            let _ = f.call0(&JsValue::NULL);
        }
    }
}

// ---------------------------------------------------------------- Fenster

/// Öffnet ein weiteres Fenster (Handbuch/Quest-Wiki) oder fokussiert es, falls
/// es schon offen ist. Gibt `true` zurück, wenn das Fenster bereits existierte.
pub async fn open_window(label: &str, url: &str, title: &str, width: f64, height: f64) -> Result<bool, String> {
    let ww = path(&["webviewWindow", "WebviewWindow"])?;
    let get_by_label = Reflect::get(&ww, &"getByLabel".into()).map_err(js_err)?;
    let existing = call_fn(&get_by_label, &ww, &[label.into()]).await?;
    if !existing.is_null() && !existing.is_undefined() {
        let set_focus = Reflect::get(&existing, &"setFocus".into()).map_err(js_err)?;
        call_fn(&set_focus, &existing, &[]).await?;
        return Ok(true);
    }
    let opts = Object::new();
    let _ = Reflect::set(&opts, &"url".into(), &url.into());
    let _ = Reflect::set(&opts, &"title".into(), &title.into());
    let _ = Reflect::set(&opts, &"width".into(), &width.into());
    let _ = Reflect::set(&opts, &"height".into(), &height.into());
    let ctor: &Function = ww.dyn_ref().ok_or("WebviewWindow fehlt")?;
    let args = Array::of2(&label.into(), &opts);
    Reflect::construct(ctor, &args).map_err(js_err)?;
    Ok(false)
}

// ---------------------------------------------------------------- Updater

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub version: String,
    pub body: Option<String>,
    handle: JsValue,
}

/// Fragt den Update-Server ab. `Ok(None)` = keine neuere Version.
pub async fn update_check() -> Result<Option<UpdateInfo>, String> {
    let v = call(&["updater", "check"], &[]).await?;
    if v.is_null() || v.is_undefined() {
        return Ok(None);
    }
    let version = Reflect::get(&v, &"version".into()).ok().and_then(|x| x.as_string()).unwrap_or_default();
    let body = Reflect::get(&v, &"body".into()).ok().and_then(|x| x.as_string());
    Ok(Some(UpdateInfo { version, body, handle: v }))
}

/// Fortschritt beim Herunterladen eines Updates.
#[derive(Clone, Debug)]
pub enum DownloadEvent {
    Started { content_length: Option<u64> },
    Progress { chunk_length: u64 },
    Finished,
}

impl UpdateInfo {
    /// Lädt das Update herunter und installiert es (danach [`relaunch`]).
    pub async fn download_and_install(&self, on_event: impl Fn(DownloadEvent) + 'static) -> Result<(), String> {
        let cb = Closure::<dyn Fn(JsValue)>::new(move |ev: JsValue| {
            let kind = Reflect::get(&ev, &"event".into()).ok().and_then(|x| x.as_string()).unwrap_or_default();
            let data = Reflect::get(&ev, &"data".into()).unwrap_or(JsValue::UNDEFINED);
            let num = |k: &str| Reflect::get(&data, &k.into()).ok().and_then(|x| x.as_f64()).map(|f| f as u64);
            match kind.as_str() {
                "Started" => on_event(DownloadEvent::Started { content_length: num("contentLength") }),
                "Progress" => on_event(DownloadEvent::Progress { chunk_length: num("chunkLength").unwrap_or(0) }),
                "Finished" => on_event(DownloadEvent::Finished),
                _ => {}
            }
        });
        let f = Reflect::get(&self.handle, &"downloadAndInstall".into()).map_err(js_err)?;
        let res = call_fn(&f, &self.handle, &[cb.as_ref().clone()]).await;
        drop(cb);
        res.map(|_| ())
    }
}
