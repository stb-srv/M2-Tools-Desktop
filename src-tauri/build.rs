fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows-msvc") {
        // Das Common-Controls-v6-Manifest, das tauri-build sonst nur in die App
        // einbettet, gilt hier für jede gelinkte Datei - auch für die Test-Binärdatei
        // (`cargo test --lib`). Neuere tao/wry-Versionen starten ohne dieses Manifest
        // nicht (STATUS_ENTRYPOINT_NOT_FOUND).
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
        let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
        tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)).expect("tauri-build fehlgeschlagen");
    } else {
        tauri_build::build();
    }
}
