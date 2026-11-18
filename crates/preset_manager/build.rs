//! Bettet Symbol und Dateiinformationen in die `.exe` ein (Explorer,
//! Taskleiste, „Eigenschaften → Details“).
//!
//! Das Symbol entsteht mit `assets/make_icon.py`. Fehlt das Werkzeug zum
//! Einbetten (Windows SDK), wird das Programm trotzdem gebaut – nur ohne
//! Symbol in der Datei; der Build meldet das als Warnung.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("assets/icon.ico")
        .set("ProductName", "Preset Manager")
        .set(
            "FileDescription",
            "Preset Manager – Presets und Profile für Camera Raw verwalten",
        )
        .set("LegalCopyright", "© 2026 Steffen Schwabe, MIT-Lizenz");
    if let Err(error) = resource.compile() {
        println!("cargo:warning=Symbol konnte nicht eingebettet werden: {error}");
    }
}
