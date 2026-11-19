//! Das Kontextmenü einer Zeile (Rechtsklick).

use eframe::egui::{self};
use egui_phosphor::regular as icon;
use preset_core::preset::Preset;

use crate::app::App;
use crate::texts::{self, tr};

use super::Action;

pub(super) fn context_menu(
    response: &egui::Response,
    app: &App,
    preset: &Preset,
    actions: &mut Vec<Action>,
) {
    let single = app.selection.len() <= 1 || !app.selection.contains(&preset.path);
    response.context_menu(|ui| {
        if !app.selection.contains(&preset.path) {
            actions.push(Action::SelectOnly(preset.path.clone()));
        }
        if ui
            .add_enabled(
                single,
                egui::Button::new(format!(
                    "{}  {}  (F2)",
                    icon::PENCIL_SIMPLE,
                    texts::rename()
                )),
            )
            .clicked()
        {
            actions.push(Action::StartRename(preset.clone()));
            ui.close();
        }
        if ui
            .button(format!(
                "{}  {}  ({})",
                icon::FOLDER_SIMPLE,
                texts::new_group_from_selection(),
                tr("Strg+G", "Ctrl+G")
            ))
            .clicked()
        {
            actions.push(Action::GroupFromSelection);
            ui.close();
        }
        let targets = if single {
            vec![preset.clone()]
        } else {
            app.selected_presets()
        };
        let deactivate = targets.iter().all(|target| target.active);
        let (glyph, text) = if deactivate {
            (icon::EYE_SLASH, texts::deactivate())
        } else {
            (icon::EYE, texts::activate())
        };
        if ui
            .button(format!("{glyph}  {text}  ({})", tr("Strg+D", "Ctrl+D")))
            .clicked()
        {
            actions.push(Action::SetActive(targets.clone(), !deactivate));
            ui.close();
        }
        if ui
            .button(format!(
                "{}  {}  ({})",
                icon::FILE_ZIP,
                texts::export_zip(),
                tr("Strg+E", "Ctrl+E")
            ))
            .clicked()
        {
            actions.push(Action::Export(targets));
            ui.close();
        }
        if ui
            .add_enabled(
                single,
                egui::Button::new(format!(
                    "{}  {}",
                    icon::FOLDER_OPEN,
                    texts::show_in_explorer()
                )),
            )
            .clicked()
        {
            actions.push(Action::Explorer(preset.path.clone()));
            ui.close();
        }
        ui.separator();
        if ui
            .button(format!(
                "{}  {}  ({})",
                icon::TRASH,
                texts::to_trash(),
                tr("Entf", "Del")
            ))
            .clicked()
        {
            actions.push(Action::Delete);
            ui.close();
        }
    });
}
