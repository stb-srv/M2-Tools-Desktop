//! localStorage-Zugriff, der nie panikt.

fn store() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn get(key: &str) -> Option<String> {
    store()?.get_item(key).ok()?
}

pub fn set(key: &str, value: &str) {
    if let Some(s) = store() {
        let _ = s.set_item(key, value);
    }
}

pub fn remove(key: &str) {
    if let Some(s) = store() {
        let _ = s.remove_item(key);
    }
}

pub fn get_json<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    serde_json::from_str(&get(key)?).ok()
}

pub fn set_json<T: serde::Serialize>(key: &str, value: &T) {
    if let Ok(s) = serde_json::to_string(value) {
        set(key, &s);
    }
}
