//! Eine Gruppe gewählt: Zustand, Umbenennen, Export, Löschen.

use eframe::egui::{RichText, Ui};
use egui_phosphor::regular as icon;
use preset_core::preset::{Preset, Section};

use crate::app::App;
use crate::texts::{self, tr};
use crate::ui::{theme, widgets};

use super::{danger_button, state_switch, where_in_camera_raw, Action};

pub(super) fn group_details(
    ui: &mut Ui,
    app: &App,
    section: Section,
    name: &str,
    actions: &mut Vec<Action>,
) {
    let (glyph, color) = widgets::section_icon(section);
    let empty = app.is_empty_group(section, name);
    let active = app.group_presets(section, name, true).len();
    let inactive = app.group_presets(section, name, false).len();
    let count = active + inactive;
    let kind = match section {
        Section::Presets => texts::presets(count),
        Section::Profiles => texts::profiles(count),
    };
    let subtitle = if empty {
        tr("Neu – noch leer", "New – still empty").to_owned()
    } else {
        format!("{kind} · {}", where_in_camera_raw(section))
    };
    let title = if name.is_empty() {
        texts::no_group()
    } else {
        name
    };
    widgets::title_block(ui, glyph, color, title, &subtitle);
    ui.add_space(8.0);

    let presets: Vec<Preset> = app
        .group_presets(section, name, true)
        .into_iter()
        .chain(app.group_presets(section, name, false))
        .cloned()
        .collect();
    if !empty {
        state_switch(ui, &presets, actions);
    }
    widgets::divider(ui);

    ui.horizontal_wrapped(|ui| {
        if ui
            .button(format!(
                "{}  {}",
                icon::PENCIL_SIMPLE,
                texts::rename_group()
            ))
            .on_hover_text("F2")
            .clicked()
        {
            actions.push(Action::RenameGroup(section, name.to_owned()));
        }
        if !presets.is_empty()
            && ui
                .button(format!("{}  {}", icon::FILE_ZIP, texts::export_group_zip()))
                .clicked()
        {
            actions.push(Action::Export(presets.clone(), name.to_owned()));
        }
    });
    ui.add_space(8.0);
    ui.label(
        RichText::new(texts::merge_hint())
            .small()
            .color(theme::FAINT),
    );
    ui.add_space(14.0);
    let delete_text = if empty {
        texts::remove_empty_group()
    } else {
        texts::group_to_trash()
    };
    if danger_button(ui, &format!("{}  {delete_text}", icon::TRASH)).clicked() {
        actions.push(Action::DeleteGroup(section, name.to_owned()));
    }
}
