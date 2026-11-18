//! Kopfleiste: Name, Suche, „Presets importieren“, Einstellungen.

use eframe::egui::{self, Frame, Margin, RichText, TextEdit, Ui};
use egui_phosphor::regular as icon;

use crate::app::{App, FolderSource};
use crate::texts;
use crate::ui::{theme, widgets};

pub fn show(ui: &mut Ui, app: &mut App, now: f64) {
    egui::Panel::top("topbar")
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .inner_margin(Margin::symmetric(12, 8))
                .stroke(egui::Stroke::new(1.0, theme::LINE)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon::SLIDERS_HORIZONTAL).size(18.0).color(theme::ACCENT));
                ui.label(RichText::new(texts::APP_TITLE).size(14.0).strong());
                ui.add_space(16.0);

                ui.label(RichText::new(icon::MAGNIFYING_GLASS).color(theme::MUTED));
                let search_id = egui::Id::new("search");
                let search = ui.add(
                    TextEdit::singleline(&mut app.search)
                        .id(search_id)
                        .hint_text(texts::SEARCH_HINT)
                        .desired_width(320.0),
                );
                if app.focus_search {
                    search.request_focus();
                    app.focus_search = false;
                }
                if search.changed() {
                    app.selection.clear();
                }
                if let (Some(dir), true) = (
                    &app.camera_raw_dir,
                    app.folder_source != FolderSource::Automatic,
                ) {
                    ui.add_space(8.0);
                    widgets::chip(ui, "Anderer Ordner", theme::WARN_TEXT, theme::WARN_BG)
                        .on_hover_text(format!("Camera-Raw-Ordner: {}", dir.display()));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let settings = ui
                        .add(egui::Button::new(RichText::new(icon::GEAR_SIX).size(16.0)).frame(false))
                        .on_hover_text(texts::SETTINGS_TIP);
                    if settings.clicked() {
                        app.settings_open = true;
                    }
                    ui.add_space(6.0);
                    let button = widgets::primary_button(
                        ui,
                        format!("{}  {}", icon::TRAY_ARROW_DOWN, texts::IMPORT_BUTTON),
                        app.import.is_none() && !app.scanning,
                    )
                    .on_hover_text("Ordner oder ZIP auswählen – oder einfach ins Fenster ziehen (Strg+Umschalt+I: ZIP)");
                    egui::Popup::menu(&button).show(|ui| {
                        if ui.button(format!("{}  {}", icon::FOLDER_OPEN, texts::IMPORT_FOLDER)).clicked() {
                            ui.close();
                            app.pick_folder(now);
                        }
                        if ui.button(format!("{}  {}", icon::FILE_ZIP, texts::IMPORT_ZIP)).clicked() {
                            ui.close();
                            app.pick_zips(now);
                        }
                    });
                });
            });
        });
}
