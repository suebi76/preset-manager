//! Einstellungen und „Über“ (F-43, Meilenstein 6).
//!
//! Ein Dialog für alles, was man selten braucht: welcher Camera-Raw-Ordner
//! verwaltet wird, wie groß die Oberfläche ist, wo Sicherungen und
//! Logdateien liegen, wie Updates ankommen und welche Version läuft.

use std::path::Path;

use eframe::egui::{self, Frame, Margin, RichText, Ui};
use egui_phosphor::regular as icon;
use preset_core::safe_fs::{default_backup_root, BACKUPS_TO_KEEP};

use crate::app::{open_folder, App, FolderSource, ToastKind, CUSTOM_DIR_VARIABLE};
use crate::logging;
use crate::settings::{UpdateMode, ZOOM_STEPS};
use crate::texts;
use crate::ui::{theme, widgets};
use crate::updates::{Status, UpdateEvent};

/// Was der Nutzer im Dialog ausgelöst hat; ausgeführt wird erst nach dem
/// Zeichnen, damit der Dialog nicht mitten im Bild den Zustand umbaut.
enum Action {
    PickFolder,
    AutomaticFolder,
    Open(std::path::PathBuf),
    UpdateMode(UpdateMode),
    TestVersions(bool),
    CheckUpdates,
    InstallUpdate,
}

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
            ui.set_width(540.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(icon::GEAR_SIX)
                        .size(18.0)
                        .color(theme::ACCENT),
                );
                ui.label(RichText::new(texts::SETTINGS_TITLE).size(16.0).strong());
            });
            ui.add_space(10.0);

            // Bei großer Schrift oder kleinem Fenster rollen statt abschneiden.
            let max_height = (ctx.content_rect().height() - 160.0).max(120.0);
            egui::ScrollArea::vertical()
                .max_height(max_height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
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

            ui.add_space(14.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::primary_button(ui, texts::CLOSE, true).clicked() {
                    close = true;
                }
            });
        });

    if close || response.should_close() {
        app.settings_open = false;
    }
    match action {
        Some(Action::PickFolder) => {
            if let Some(dir) = rfd::FileDialog::new()
                .set_title(texts::CHOOSE_CAMERA_RAW)
                .pick_folder()
            {
                app.choose_camera_raw_dir(ctx, Some(dir), now);
            }
        }
        Some(Action::AutomaticFolder) => app.choose_camera_raw_dir(ctx, None, now),
        Some(Action::Open(path)) => {
            if let Err(message) = open_folder(&path) {
                app.toast(message, ToastKind::Error, now);
            }
        }
        Some(Action::UpdateMode(mode)) => {
            app.settings.updates = mode;
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
            RichText::new("Die installierte Fassung gibt es als PresetManager-Setup.exe unter Releases auf GitHub.")
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
        .checkbox(&mut test_versions, "Testversionen erhalten")
        .on_hover_text("Vorabversionen zum Ausprobieren. Sie können noch Fehler enthalten.")
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
                    texts::UPDATE_CHECK
                )),
            )
            .clicked()
        {
            *action = Some(Action::CheckUpdates);
        }
        let label = match updater.status {
            Status::Available(_) => Some(texts::UPDATE_INSTALL),
            Status::Ready(_) => Some(texts::UPDATE_RESTART),
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
    widgets::section_heading(ui, "CAMERA-RAW-ORDNER");
    ui.add_space(4.0);
    path_line(ui, app.camera_raw_dir.as_deref());
    let source = match app.folder_source {
        FolderSource::Automatic => "Automatisch gefunden – derselbe Ordner für Photoshop, Photoshop Beta und Lightroom Classic.".to_owned(),
        FolderSource::Chosen => "Selbst gewählt.".to_owned(),
        FolderSource::Environment => format!(
            "Vorgegeben über die Umgebungsvariable {CUSTOM_DIR_VARIABLE}; solange sie gesetzt ist, lässt sich der Ordner hier nicht ändern."
        ),
    };
    ui.label(RichText::new(source).small().color(theme::MUTED));
    ui.add_space(6.0);

    let can_change = app.folder_source != FolderSource::Environment && !app.busy();
    ui.horizontal(|ui| {
        let pick = ui
            .add_enabled(
                can_change,
                egui::Button::new(format!(
                    "{}  {}",
                    icon::FOLDER_OPEN,
                    texts::CHOOSE_CAMERA_RAW
                )),
            )
            .on_disabled_hover_text("Erst warten, bis die laufende Aktion fertig ist.");
        if pick.clicked() {
            *action = Some(Action::PickFolder);
        }
        if app.folder_source == FolderSource::Chosen
            && ui
                .add_enabled(
                    can_change,
                    egui::Button::new(format!("{}  Automatisch suchen", icon::MAGIC_WAND)),
                )
                .clicked()
        {
            *action = Some(Action::AutomaticFolder);
        }
        if let Some(dir) = app.camera_raw_dir.as_deref().filter(|dir| dir.is_dir()) {
            if widgets::icon_button(ui, icon::ARROW_SQUARE_OUT, "Öffnen").clicked() {
                *action = Some(Action::Open(dir.to_path_buf()));
            }
        }
    });
}

fn zoom_section(ui: &mut Ui, ctx: &egui::Context) {
    widgets::section_heading(ui, "DARSTELLUNG");
    ui.add_space(4.0);
    // Feste Stufen statt Schieberegler: Ein Regler wächst beim Ziehen mit
    // der Oberfläche unter dem Mauszeiger mit und läuft davon.
    let zoom = ctx.zoom_factor();
    ui.horizontal(|ui| {
        ui.label("Größe der Oberfläche");
        ui.add_space(8.0);
        let smaller = ZOOM_STEPS.iter().rev().find(|step| **step < zoom - 0.001);
        let larger = ZOOM_STEPS.iter().find(|step| **step > zoom + 0.001);
        if ui
            .add_enabled(smaller.is_some(), egui::Button::new(icon::MINUS))
            .on_hover_text("Kleiner")
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
            .on_hover_text("Größer")
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
                egui::Button::new("Zurücksetzen"),
            )
            .clicked()
        {
            ctx.set_zoom_factor(1.0);
        }
    });
    ui.label(
        RichText::new("Auch mit Strg + Plus / Minus, Strg + 0 setzt zurück.")
            .small()
            .color(theme::MUTED),
    );
}

fn storage_section(ui: &mut Ui, action: &mut Option<Action>) {
    widgets::section_heading(ui, "SICHERUNGEN UND PROTOKOLL");
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!(
            "Bevor die App eine Datei ersetzt oder ändert, sichert sie das Original. \
             Die Sicherungen der letzten {BACKUPS_TO_KEEP} Aktionen bleiben erhalten. \
             Gelöschte Einträge liegen im Windows-Papierkorb."
        ))
        .small()
        .color(theme::MUTED),
    );
    ui.add_space(4.0);
    folder_row(ui, "Sicherungen", default_backup_root().as_deref(), action);
    folder_row(ui, "Logdateien", logging::log_dir().as_deref(), action);
}

fn folder_row(ui: &mut Ui, label: &str, path: Option<&Path>, action: &mut Option<Action>) {
    ui.horizontal(|ui| {
        ui.add_sized([90.0, 20.0], egui::Label::new(label));
        match path {
            Some(path) => {
                if widgets::icon_button(ui, icon::FOLDER_OPEN, "Ordner öffnen")
                    .on_hover_text(path.display().to_string())
                    .clicked()
                {
                    *action = Some(Action::Open(path.to_path_buf()));
                }
            }
            None => {
                ui.label(RichText::new("%LOCALAPPDATA% ist nicht gesetzt").color(theme::MUTED));
            }
        }
    });
}

fn about_section(ui: &mut Ui, app: &App) {
    widgets::section_heading(ui, "ÜBER");
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
        RichText::new("MIT-Lizenz · © 2026 Steffen Schwabe")
            .small()
            .color(theme::MUTED),
    );
    ui.label(
        RichText::new(texts::TRADEMARK_NOTE)
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
