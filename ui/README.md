# M2Manager-Oberfläche (Rust/Leptos)

Die komplette Oberfläche ist Rust: Leptos 0.8 (CSR), kompiliert zu
WebAssembly und von Trunk gebaut; Tailwind läuft als Standalone-Binary.
Tauri lädt das Ergebnis aus `../dist`. Ein Bundle für alle Fenster:
`?view=manual` = Handbuch, `?view=wiki` = Quest-Wiki, sonst die Haupt-App.
Die Brücke zum Backend ist `src/tauri.rs` (nutzt `window.__TAURI__`,
dafür steht `withGlobalTauri` in `tauri.conf.json`).

## Struktur
- `ui/src/features/<modul>/` – je ein Modul (Unterdateien als Rust-Module, z.B. `components/…` → `mod.rs` + Dateien).
- `ui/src/features/shared/` – EntityBrowser, GenericRowEditor, ItemUsageModal, drop_chance, SectionTabs (re-export).
- `ui/src/components/` – Button, Icon, PageHeader, PasswordInput, Skeleton/Spinner, toast, markdown, section_tabs.
- `ui/src/util/` – `log_activity`, `open_manual`, `use_save_shortcut`, `cn`, `connection`, `csv::export_rows_as_csv`, `format::{de_int, de_float, de_datetime, de_date, de_time, bytes}`, `storage`.
- `ui/src/store/` – `navigation::{nav(), Section, ModuleId, EmbeddedModule, report_section_dirty}`, `theme`, `update`, `settings_section::{open_settings, SettingsSection}`, `connection_profiles`.
- `ui/src/tauri.rs` – `invoke`, `invoke0`, `invoke_unit`, `dialog_open(_one)`, `dialog_save`, `dialog_confirm`, `listen`, `emit`, `open_url`, `open_path`, `open_window`, `update_check`, `relaunch`, `app_version`.

## Muster
- `invoke("cmd", &json!({ "camelCaseArg": x }))` – Argumentnamen exakt wie im TS-Code (camelCase).
  Rückgabetypen als `#[derive(Deserialize)]`-Structs mit den snake_case-Feldnamen aus dem TS-Interface.
- Zustand: `RwSignal::new(..)`, abgeleitet: `Memo::new` / Closures. `useEffect` → `Effect::new`.
  Async: `leptos::task::spawn_local(async move { .. })`. Überholte Antworten mit einem Zähler in `StoredValue` verwerfen (siehe `entity_browser.rs`).
- Fehler: `invoke` liefert `Result<T, String>` – wie im TS anzeigen (inline oder `toast::error`).
- Icons: `<Icon name="Search" class="size-4"/>` (Namen wie lucide-react). Fehlt eins: `ui/src/components/icons_data.rs` ergänzen (sortiert!).
- Übersetzungen: `crate::i18n::t("key")` (reaktiv). Fest verdrahtete deutsche Texte bleiben fest verdrahtet.
- Buttons: `<Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| ..>`; `disabled` nimmt `Signal<bool>` (`Signal::derive(move || ..)`).
- Tabs innerhalb eines Bereichs: `SectionTabs` mit `SectionTab::new(id, label_fn, content_fn)`.
- Strg+S: `use_save_shortcut(section, enabled_fn, on_save_fn)`. Dirty-Punkt: `report_section_dirty`.
- Navigation mit Daten: `nav().go_to_with_selection(section, ref)` / `nav().consume_pending_selection(section)`.
- Reine Logik (früher `*.ts` mit `*.test.ts`) als eigenes Rust-Modul portieren und die Vitest-Tests als `#[cfg(test)]`-Tests übernehmen.
- Kein JavaScript, kein `js_sys::eval`. Nur `web-sys`/`js-sys`-Bindings (Canvas, WebGL, Events) sind erlaubt.

## Prüfen
```
cd ui
cargo check --target wasm32-unknown-unknown   # muss ohne Fehler durchlaufen
cargo test                                      # reine Logik-Tests (nativ)
trunk build                                     # erzeugt ../dist
```
