//! „Verwalten“ für ein einzelnes Preset: Gruppe, Datei, Umbenennen,
//! Explorer, Export, Löschen.

use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as icon;
use preset_core::preset::Preset;

use crate::app::{is_adobe_origin, App};
use crate::texts::{self, tr};
use crate::ui::{theme, widgets};

use super::{danger_button, fact, group_picker, label, where_in_camera_raw, Action};

pub(super) fn manage_details(ui: &mut Ui, app: &App, preset: &Preset, actions: &mut Vec<Action>) {
    label(ui, tr("Gruppe", "Group"));
    if let Some(group) = group_picker(
        ui,
        app,
        preset.section(),
        &preset.group,
        "group-of-selection",
    ) {
        actions.push(Action::MoveTo(vec![preset.clone()], group));
    }
    ui.add_space(10.0);

    let origin = if is_adobe_origin(preset) {
        texts::origin_adobe()
    } else {
        texts::origin_own()
    };
    egui::Grid::new("preset-facts")
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            fact(
                ui,
                tr("Zu finden", "Found in"),
                where_in_camera_raw(preset.section()),
            );
            fact(ui, tr("Herkunft", "Origin"), origin);
            fact(ui, "UUID", preset.uuid.as_deref().unwrap_or("–"));
        });
    ui.add_space(8.0);
    label(ui, tr("Datei", "File"));
    ui.add(
        egui::Label::new(
            RichText::new(preset.path.display().to_string())
                .monospace()
                .size(11.0)
                .color(theme::MUTED),
        )
        .wrap(),
    );

    widgets::divider(ui);
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(format!("{}  {}", icon::PENCIL_SIMPLE, texts::rename()))
            .on_hover_text("F2")
            .clicked()
        {
            actions.push(Action::Rename(preset.clone()));
        }
        if ui
            .button(format!(
                "{}  {}",
                icon::FOLDER_OPEN,
                texts::show_in_explorer()
            ))
            .clicked()
        {
            actions.push(Action::Explorer(preset.path.clone()));
        }
        if ui
            .button(format!("{}  {}", icon::FILE_ZIP, texts::export_zip()))
            .on_hover_text(tr("Strg+E", "Ctrl+E"))
            .clicked()
        {
            actions.push(Action::Export(vec![preset.clone()], preset.name.clone()));
        }
    });
    ui.add_space(14.0);
    if danger_button(ui, &format!("{}  {}", icon::TRASH, texts::to_trash()))
        .on_hover_text(tr("Entf", "Del"))
        .clicked()
    {
        actions.push(Action::DeleteSelection);
    }
}
