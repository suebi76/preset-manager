//! Einstellungen und „Über“ (F-43, Meilenstein 6).
//!
//! Ein Dialog für alles, was man selten braucht: welcher Camera-Raw-Ordner
//! verwaltet wird, wie groß die Oberfläche ist, wo Sicherungen und
//! Logdateien liegen, wie Updates ankommen und welche Version läuft.
//!
//! | Datei | Abschnitt |
//! |---|---|
//! | `general` | Sprache, Größe der Oberfläche |
//! | `updates` | Updates |
//! | `folders` | Camera-Raw-Ordner |
//! | `storage` | Speicherorte, Über |

mod folders;
mod general;
mod storage;
mod updates;

use eframe::egui::{self, Frame, Margin, RichText, Ui};
use egui_phosphor::regular as icon;

use crate::app::{open_folder, App, ToastKind};
use crate::settings::{LanguageChoice, UpdateMode};
use crate::texts::{self};
use crate::ui::{theme, widgets};
use crate::updates::UpdateEvent;
use folders::folder_section;
use general::language_section;
use general::zoom_section;
use storage::about_section;
use storage::storage_section;
use updates::update_section;

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
            widgets::scroll(
                ui,
                egui::ScrollArea::vertical()
                    .max_height(scroll_height)
                    .auto_shrink([false, true]),
                |ui| {
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
                },
            );

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

fn separator(ui: &mut Ui) {
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);
}
