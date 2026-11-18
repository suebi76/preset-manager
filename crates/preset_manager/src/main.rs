//! Preset Manager – das Fenster.
//!
//! Aufruf ohne Angaben öffnet die Übersicht. Werden Ordner oder ZIP-Dateien
//! übergeben (z. B. auf `Start.bat` gezogen), erscheint gleich die
//! Importvorschau.
//!
//! | Modul | Inhalt |
//! |---|---|
//! | [`app`] | Zustand der Anwendung, Ablauf eines Bildes |
//! | [`actions`] | was der Nutzer auslösen kann (Import, Verwalten, Ziehen, Tasten) |
//! | [`settings`] | Einstellungen der App (Ordner, Zoom, leere Gruppen) |
//! | [`logging`] | Logdatei für die Fehlersuche |
//! | [`updates`] | Installation und Selbst-Update (Velopack) |
//! | [`worker`] | Hintergrund-Thread für alle Dateizugriffe |
//! | [`ui`] | die Bereiche des Fensters |
//! | [`texts`] | alle Oberflächentexte |
//!
//! Das Konsolenwerkzeug für Tests und Skripte ist ein eigenes Programm:
//! `src/bin/preset_cli`.

// Im fertigen Programm kein Konsolenfenster neben dem Fenster öffnen.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// In Tests dürfen Abkürzungen benutzt werden, die im Programmcode verboten sind.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod actions;
mod app;
mod logging;
mod settings;
mod texts;
mod ui;
mod updates;
mod worker;

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;

fn main() -> eframe::Result {
    // Velopack zuerst: Beim Installieren, Aktualisieren und Deinstallieren
    // startet es das Programm kurz mit eigenen Angaben und beendet es wieder.
    velopack::VelopackApp::build().run();
    // Lebt bis zum Ende von `main`, damit das Log vollständig geschrieben wird.
    let _log = logging::init();
    let start_paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(texts::APP_TITLE)
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([820.0, 480.0])
            .with_drag_and_drop(true)
            .with_icon(window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        texts::APP_TITLE,
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc, start_paths)))),
    )
}

/// Das Symbol für Fenster und Taskleiste (`assets/icon.png`). Lässt es sich
/// nicht lesen, bleibt das Standardsymbol.
fn window_icon() -> Arc<egui::IconData> {
    let bytes = include_bytes!("../assets/icon.png");
    Arc::new(eframe::icon_data::from_png_bytes(bytes).unwrap_or_default())
}
