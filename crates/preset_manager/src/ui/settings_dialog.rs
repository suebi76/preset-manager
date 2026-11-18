//! Einstellungen und „Über“ (F-43, Meilenstein 6).
//!
//! Ein Dialog für alles, was man selten braucht: welcher Camera-Raw-Ordner
//! verwaltet wird, wie groß die Oberfläche ist, wo Sicherungen und
//! Logdateien liegen, wie Updates ankommen und welche Version läuft.

use std::path::Path;

use eframe::egui::{self, Frame, Margin, RichText, Ui};
use egui_phosphor::regular as icon;
use preset_core::adobe::{locations::same_path, FolderKind, FoundFolder};
use preset_core::safe_fs::{default_backup_root, BACKUPS_TO_KEEP};

use crate::app::{open_folder, App, FolderSource, ToastKind, CUSTOM_DIR_VARIABLE};
use crate::logging;
use crate::settings::{LanguageChoice, UpdateMode, ZOOM_STEPS};
use crate::texts::{self, tr, tr_format, Language};
use crate::ui::{theme, widgets};
use crate::updates::{Status, UpdateEvent};

/// Was der Nutzer im Dialog ausgelöst hat; ausgeführt wird erst nach dem
/// Zeichnen, damit der Dialog nicht mitten im Bild den Zustand umbaut.
enum Action {
    PickFolder,
    AutomaticFolder,
    UseFolder(std::path::PathBuf),
    Open(std::path::PathBuf),
    UpdateMode(UpdateMode),
    Language(LanguageChoice),
    TestVersions(bool),
    CheckUpdates,
    InstallUpdate,
}

/// Abstand des Dialogs zum Fensterrand.
const DIALOG_MARGIN: f32 = 16.0;
/// Höhe der Zeile mit „Schließen“ samt Abstand darüber.
const FOOTER_HEIGHT: f32 = 46.0;

pub fn show(ctx: &egui::Context, app: &mut App, now: f64) {
    if !app.settings_open {
        return;
    }
    let mut action = None;
    let mut close = false;

    let response = egui::Modal::new(egui::Id::new("settings-dialog"))
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .stroke(egui::Stroke::new(1.0, theme::LINE))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(Margin::same(18)),
        )
        .show(ctx, |ui| {
            // Der Dialog passt sich dem Fenster an: nie breiter oder höher als
            // der Platz, den es hat (Abstand zum Rand und Innenrand
            // abgezogen). Was nicht passt, rollt – Kopf und „Schließen“
            // bleiben immer sichtbar.
            let screen = ctx.content_rect();
            let width = (screen.width() - 2.0 * (DIALOG_MARGIN + 18.0)).clamp(240.0, 540.0);
            let height = (screen.height() - 2.0 * (DIALOG_MARGIN + 18.0)).max(140.0);
            ui.set_width(width);
            ui.set_max_height(height);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(icon::GEAR_SIX)
                        .size(18.0)
                        .color(theme::ACCENT),
                );
                ui.label(RichText::new(texts::settings_title()).size(16.0).strong());
            });
            ui.add_space(10.0);

            let scroll_height = (ui.available_height() - FOOTER_HEIGHT).max(40.0);
            egui::ScrollArea::vertical()
                .max_height(scroll_height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    language_section(ui, app, &mut action);
                    separator(ui);
                    folder_section(ui, app, &mut action);
                    separator(ui);
                    zoom_section(ui, ctx);
                    separator(ui);
                    storage_section(ui, &mut action);
                    separator(ui);
                    update_section(ui, app, &mut action);
                    separator(ui);
                    about_section(ui, app);
                });

            ui.add_space(12.0);
            // Feste Höhe, sonst nimmt das rechtsbündige Layout den ganzen
            // restlichen Platz ein.
            let (footer, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), FOOTER_HEIGHT - 12.0),
                egui::Sense::hover(),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(footer), |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::primary_button(ui, texts::close(), true).clicked() {
                        close = true;
                    }
                });
            });
        });

    if close || response.should_close() {
        app.settings_open = false;
    }
    match action {
        Some(Action::PickFolder) => {
            if let Some(dir) = rfd::FileDialog::new()
                .set_title(texts::choose_camera_raw())
                .pick_folder()
            {
                app.choose_camera_raw_dir(ctx, Some(dir), now);
            }
        }
        Some(Action::AutomaticFolder) => app.choose_camera_raw_dir(ctx, None, now),
        Some(Action::UseFolder(dir)) => app.choose_camera_raw_dir(ctx, Some(dir), now),
        Some(Action::Open(path)) => {
            if let Err(message) = open_folder(&path) {
                app.toast(message, ToastKind::Error, now);
            }
        }
        Some(Action::UpdateMode(mode)) => {
            app.settings.updates = mode;
            app.update_settings_changed(now);
        }
        Some(Action::Language(choice)) => {
            app.settings.language = choice;
            texts::set_language(choice.resolve());
            app.update_settings_changed(now);
        }
        Some(Action::TestVersions(on)) => {
            app.settings.test_versions = on;
            app.update_settings_changed(now);
        }
        Some(Action::CheckUpdates) => app.updater.check(ctx),
        Some(Action::InstallUpdate) => {
            if let Some(UpdateEvent::RestartNow) = app.updater.install_now(ctx) {
                app.restart_pending = true;
            }
        }
        None => {}
    }
}

/// Die Überschrift steht in beiden Sprachen – wer die falsche Sprache
/// erwischt hat, findet die Umstellung trotzdem.
fn language_section(ui: &mut Ui, app: &App, action: &mut Option<Action>) {
    widgets::section_heading(ui, "SPRACHE · LANGUAGE");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for choice in LanguageChoice::ALL {
            let label = match choice {
                LanguageChoice::System => {
                    let system = match texts::system_language() {
                        Language::German => "Deutsch",
                        Language::English => "English",
                    };
                    tr_format!("Wie Windows ({system})", "Same as Windows ({system})")
                }
                LanguageChoice::German => "Deutsch".to_owned(),
                LanguageChoice::English => "English".to_owned(),
            };
            if ui.radio(app.settings.language == choice, label).clicked()
                && app.settings.language != choice
            {
                *action = Some(Action::Language(choice));
            }
        }
    });
}

fn update_section(ui: &mut Ui, app: &App, action: &mut Option<Action>) {
    widgets::section_heading(ui, "UPDATES");
    ui.add_space(4.0);
    let updater = &app.updater;
    ui.label(updater.status_text());
    if let Status::Downloading { percent, .. } = updater.status {
        ui.add(egui::ProgressBar::new(f32::from(percent) / 100.0).desired_width(300.0));
    }
    if !updater.is_installed() {
        ui.label(
            RichText::new(tr(
                "Die installierte Fassung gibt es als PresetManager-Setup.exe unter Releases auf GitHub.",
                "The installable version is PresetManager-Setup.exe under Releases on GitHub.",
            ))
                .small()
                .color(theme::MUTED),
        );
        return;
    }
    ui.add_space(4.0);
    for mode in UpdateMode::ALL {
        let (label, hint) = texts::update_mode(mode);
        if ui
            .radio(app.settings.updates == mode, label)
            .on_hover_text(hint)
            .clicked()
            && app.settings.updates != mode
        {
            *action = Some(Action::UpdateMode(mode));
        }
    }
    ui.label(
        RichText::new(texts::update_mode(app.settings.updates).1)
            .small()
            .color(theme::MUTED),
    );
    let mut test_versions = app.settings.test_versions;
    if ui
        .checkbox(
            &mut test_versions,
            tr("Testversionen erhalten", "Receive test versions"),
        )
        .on_hover_text(tr(
            "Vorabversionen zum Ausprobieren. Sie können noch Fehler enthalten.",
            "Pre-releases for trying out. They may still contain bugs.",
        ))
        .changed()
    {
        *action = Some(Action::TestVersions(test_versions));
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !updater.busy(),
                egui::Button::new(format!(
                    "{}  {}",
                    icon::ARROWS_CLOCKWISE,
                    texts::update_check()
                )),
            )
            .clicked()
        {
            *action = Some(Action::CheckUpdates);
        }
        let label = match updater.status {
            Status::Available(_) => Some(texts::update_install()),
            Status::Ready(_) => Some(texts::update_restart()),
            _ => None,
        };
        if let Some(label) = label {
            if widgets::primary_button(ui, label, !app.restart_pending).clicked() {
                *action = Some(Action::InstallUpdate);
            }
        }
    });
}

fn folder_section(ui: &mut Ui, app: &App, action: &mut Option<Action>) {
    widgets::section_heading(ui, tr("CAMERA-RAW-ORDNER", "CAMERA RAW FOLDER"));
    ui.add_space(4.0);
    path_line(ui, app.camera_raw_dir.as_deref());
    let lightroom_elsewhere = lightroom_reads_catalog(app);
    let source = match app.folder_source {
        FolderSource::Automatic if lightroom_elsewhere => {
            tr("Automatisch gefunden.", "Found automatically.").to_owned()
        }
        FolderSource::Automatic => tr(
            "Automatisch gefunden – derselbe Ordner für Photoshop, Photoshop Beta und Lightroom Classic.",
            "Found automatically – the same folder for Photoshop, Photoshop Beta and Lightroom Classic.",
        )
        .to_owned(),
        FolderSource::Chosen => tr("Selbst gewählt.", "Chosen by you.").to_owned(),
        FolderSource::Environment => tr_format!(
            "Vorgegeben über die Umgebungsvariable {CUSTOM_DIR_VARIABLE}; solange sie gesetzt ist, lässt sich der Ordner hier nicht ändern.",
            "Set by the environment variable {CUSTOM_DIR_VARIABLE}; while it is set, the folder cannot be changed here."
        ),
    };
    ui.label(RichText::new(source).small().color(theme::MUTED));
    ui.add_space(6.0);

    let can_change = app.folder_source != FolderSource::Environment && !app.busy();
    if lightroom_elsewhere {
        lightroom_warning(ui);
    }
    found_folders(ui, app, can_change, action);
    ui.horizontal(|ui| {
        let pick = ui
            .add_enabled(
                can_change,
                egui::Button::new(format!(
                    "{}  {}",
                    icon::FOLDER_OPEN,
                    texts::choose_camera_raw()
                )),
            )
            .on_disabled_hover_text(texts::wait_for_action());
        if pick.clicked() {
            *action = Some(Action::PickFolder);
        }
        if app.folder_source == FolderSource::Chosen
            && ui
                .add_enabled(
                    can_change,
                    egui::Button::new(format!(
                        "{}  {}",
                        icon::MAGIC_WAND,
                        tr("Automatisch suchen", "Search automatically")
                    )),
                )
                .clicked()
        {
            *action = Some(Action::AutomaticFolder);
        }
        if let Some(dir) = app.camera_raw_dir.as_deref().filter(|dir| dir.is_dir()) {
            if widgets::icon_button(ui, icon::ARROW_SQUARE_OUT, tr("Öffnen", "Open")).clicked() {
                *action = Some(Action::Open(dir.to_path_buf()));
            }
        }
    });
}

/// Lightroom Classic liest Presets zurzeit neben dem Katalog, verwaltet wird
/// aber der Benutzerordner: Was hier installiert wird, sieht nur Photoshop.
fn lightroom_reads_catalog(app: &App) -> bool {
    let current = app.camera_raw_dir.as_deref();
    let uses_user_folder = app.found_folders.iter().any(|folder| {
        folder.kind == FolderKind::User
            && current.is_some_and(|dir| same_path(dir, &folder.camera_raw_dir))
    });
    let catalog_in_use = app.found_folders.iter().any(|folder| {
        matches!(
            folder.kind,
            FolderKind::LightroomCatalog { in_use: true, .. }
        )
    });
    uses_user_folder && catalog_in_use
}

fn lightroom_warning(ui: &mut Ui) {
    Frame::new()
        .fill(theme::WARN_BG)
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!(
                    "{}  {}",
                    icon::WARNING,
                    tr(
                        "In Lightroom Classic ist „Presets mit diesem Katalog speichern“ eingeschaltet. Was hier installiert wird, sieht dann nur Photoshop – für Lightroom unten den Katalogordner wählen.",
                        "Lightroom Classic has “Store presets with this catalog” turned on. What you install here then shows up only in Photoshop – for Lightroom, choose the catalog folder below.",
                    )
                ))
                .small()
                .color(theme::WARN_TEXT),
            );
        });
    ui.add_space(6.0);
}

/// Die gefundenen Camera-Raw-Ordner zur Auswahl – nur, wenn es mehr als den
/// Benutzerordner gibt.
fn found_folders(ui: &mut Ui, app: &App, can_change: bool, action: &mut Option<Action>) {
    if app.found_folders.len() < 2 {
        return;
    }
    ui.label(
        RichText::new(tr(
            "Gefunden auf diesem Rechner:",
            "Found on this computer:",
        ))
        .small()
        .color(theme::MUTED),
    );
    for folder in &app.found_folders {
        let current = app
            .camera_raw_dir
            .as_deref()
            .is_some_and(|dir| same_path(dir, &folder.camera_raw_dir));
        ui.add_enabled_ui(can_change, |ui| {
            let response = ui
                .radio(current, found_title(folder))
                .on_hover_text(folder.camera_raw_dir.display().to_string())
                .on_disabled_hover_text(texts::wait_for_action());
            if response.clicked() && !current {
                *action = Some(match folder.kind {
                    FolderKind::User => Action::AutomaticFolder,
                    FolderKind::LightroomCatalog { .. } => {
                        Action::UseFolder(folder.camera_raw_dir.clone())
                    }
                });
            }
        });
        ui.indent(folder.camera_raw_dir.as_os_str(), |ui| {
            ui.label(
                RichText::new(found_note(folder))
                    .small()
                    .color(theme::FAINT),
            );
        });
    }
    ui.add_space(6.0);
}

fn found_title(folder: &FoundFolder) -> String {
    match &folder.kind {
        FolderKind::User => tr("Benutzerordner", "User folder").to_owned(),
        FolderKind::LightroomCatalog { catalog, .. } => {
            let name = catalog
                .file_stem()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            tr_format!("Lightroom-Katalog „{name}“", "Lightroom catalog “{name}”")
        }
    }
}

fn found_note(folder: &FoundFolder) -> String {
    match &folder.kind {
        FolderKind::User => tr(
            "Für Photoshop, Photoshop Beta und Lightroom Classic – der Standard.",
            "For Photoshop, Photoshop Beta and Lightroom Classic – the default.",
        )
        .to_owned(),
        FolderKind::LightroomCatalog { in_use: true, .. } => tr(
            "Nur Lightroom Classic, „Presets mit diesem Katalog speichern“ ist an.",
            "Lightroom Classic only, “Store presets with this catalog” is on.",
        )
        .to_owned(),
        FolderKind::LightroomCatalog { in_use: false, .. } => tr(
            "Nur Lightroom Classic, und nur wenn dort „Presets mit diesem Katalog speichern“ an ist – zurzeit aus.",
            "Lightroom Classic only, and only while “Store presets with this catalog” is on there – currently off.",
        )
        .to_owned(),
    }
}

fn zoom_section(ui: &mut Ui, ctx: &egui::Context) {
    widgets::section_heading(ui, tr("DARSTELLUNG", "DISPLAY"));
    ui.add_space(4.0);
    // Feste Stufen statt Schieberegler: Ein Regler wächst beim Ziehen mit
    // der Oberfläche unter dem Mauszeiger mit und läuft davon.
    let zoom = ctx.zoom_factor();
    ui.horizontal(|ui| {
        ui.label(tr("Größe der Oberfläche", "Interface size"));
        ui.add_space(8.0);
        let smaller = ZOOM_STEPS.iter().rev().find(|step| **step < zoom - 0.001);
        let larger = ZOOM_STEPS.iter().find(|step| **step > zoom + 0.001);
        if ui
            .add_enabled(smaller.is_some(), egui::Button::new(icon::MINUS))
            .on_hover_text(tr("Kleiner", "Smaller"))
            .clicked()
        {
            if let Some(step) = smaller {
                ctx.set_zoom_factor(*step);
            }
        }
        ui.add_sized(
            [52.0, 20.0],
            egui::Label::new(RichText::new(format!("{:.0} %", zoom * 100.0)).strong()),
        );
        if ui
            .add_enabled(larger.is_some(), egui::Button::new(icon::PLUS))
            .on_hover_text(tr("Größer", "Larger"))
            .clicked()
        {
            if let Some(step) = larger {
                ctx.set_zoom_factor(*step);
            }
        }
        ui.add_space(8.0);
        if ui
            .add_enabled(
                (zoom - 1.0).abs() > 0.001,
                egui::Button::new(tr("Zurücksetzen", "Reset")),
            )
            .clicked()
        {
            ctx.set_zoom_factor(1.0);
        }
    });
    ui.label(
        RichText::new(tr(
            "Auch mit Strg + Plus / Minus, Strg + 0 setzt zurück.",
            "Also with Ctrl + Plus / Minus, Ctrl + 0 resets.",
        ))
        .small()
        .color(theme::MUTED),
    );
}

fn storage_section(ui: &mut Ui, action: &mut Option<Action>) {
    widgets::section_heading(ui, tr("SICHERUNGEN UND PROTOKOLL", "BACKUPS AND LOG"));
    ui.add_space(4.0);
    ui.label(
        RichText::new(tr_format!(
            "Bevor die App eine Datei ersetzt oder ändert, sichert sie das Original. \
             Die Sicherungen der letzten {BACKUPS_TO_KEEP} Aktionen bleiben erhalten. \
             Gelöschte Einträge liegen im Windows-Papierkorb.",
            "Before the app replaces or changes a file, it backs up the original. \
             Backups of the last {BACKUPS_TO_KEEP} actions are kept. \
             Deleted items are in the Windows Recycle Bin."
        ))
        .small()
        .color(theme::MUTED),
    );
    ui.add_space(4.0);
    folder_row(
        ui,
        tr("Sicherungen", "Backups"),
        default_backup_root().as_deref(),
        action,
    );
    folder_row(
        ui,
        tr("Logdateien", "Log files"),
        logging::log_dir().as_deref(),
        action,
    );
}

fn folder_row(ui: &mut Ui, label: &str, path: Option<&Path>, action: &mut Option<Action>) {
    ui.horizontal(|ui| {
        ui.add_sized([90.0, 20.0], egui::Label::new(label));
        match path {
            Some(path) => {
                if widgets::icon_button(ui, icon::FOLDER_OPEN, tr("Ordner öffnen", "Open folder"))
                    .on_hover_text(path.display().to_string())
                    .clicked()
                {
                    *action = Some(Action::Open(path.to_path_buf()));
                }
            }
            None => {
                ui.label(
                    RichText::new(tr(
                        "%LOCALAPPDATA% ist nicht gesetzt",
                        "%LOCALAPPDATA% is not set",
                    ))
                    .color(theme::MUTED),
                );
            }
        }
    });
}

fn about_section(ui: &mut Ui, app: &App) {
    widgets::section_heading(ui, tr("ÜBER", "ABOUT"));
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!(
            "{} {}",
            texts::APP_TITLE,
            app.updater.current_version
        ))
        .strong(),
    );
    ui.label(
        RichText::new(tr(
            "MIT-Lizenz · © 2026 Steffen Schwabe",
            "MIT License · © 2026 Steffen Schwabe",
        ))
        .small()
        .color(theme::MUTED),
    );
    ui.label(
        RichText::new(texts::trademark_note())
            .small()
            .color(theme::MUTED),
    );
}

/// Ein Pfad in einer Zeile, bei Platzmangel gekürzt, voll im Tooltip.
fn path_line(ui: &mut Ui, path: Option<&Path>) {
    let text = path.map_or_else(|| "–".to_owned(), |path| path.display().to_string());
    widgets::truncated(ui, RichText::new(&text).monospace()).on_hover_text(text);
}

fn separator(ui: &mut Ui) {
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);
}
