//! Mehrere Einträge ausgewählt: gemeinsam verschieben, an- und abschalten,
//! exportieren, löschen.

use eframe::egui::{RichText, Ui};
use egui_phosphor::regular as icon;
use preset_core::preset::{Preset, Section};

use crate::actions::common_section;
use crate::app::App;
use crate::texts::{self, tr, tr_format};
use crate::ui::{theme, widgets};

use super::{danger_button, group_picker, label, state_switch, Action};

pub(super) fn multi_details(ui: &mut Ui, app: &App, presets: &[Preset], actions: &mut Vec<Action>) {
    let preset_count = presets
        .iter()
        .filter(|p| p.section() == Section::Presets)
        .count();
    let profile_count = presets.len() - preset_count;
    let subtitle = match (preset_count, profile_count) {
        (_, 0) => texts::presets(preset_count),
        (0, _) => texts::profiles(profile_count),
        _ => format!(
            "{} · {}",
            texts::presets(preset_count),
            texts::profiles(profile_count)
        ),
    };
    widgets::title_block(
        ui,
        icon::STACK,
        theme::ACCENT,
        &tr_format!("{} ausgewählt", "{} selected", presets.len()),
        &subtitle,
    );
    ui.add_space(8.0);
    state_switch(ui, presets, actions);
    widgets::divider(ui);

    match common_section(presets) {
        Some(section) => {
            label(ui, texts::move_to());
            if let Some(group) =
                group_picker(ui, app, section, texts::choose_group(), "move-selection")
            {
                actions.push(Action::MoveTo(presets.to_vec(), group));
            }
            ui.add_space(8.0);
            if ui
                .button(format!(
                    "{}  {}",
                    icon::FOLDER_SIMPLE,
                    texts::new_group_from_selection(),
                ))
                .on_hover_text(tr("Strg+G", "Ctrl+G"))
                .clicked()
            {
                actions.push(Action::GroupFromSelection);
            }
        }
        None => {
            ui.label(
                RichText::new(texts::mixed_sections())
                    .small()
                    .color(theme::MUTED),
            );
        }
    }
    ui.add_space(8.0);
    if ui
        .button(format!("{}  {}", icon::FILE_ZIP, texts::export_zip()))
        .on_hover_text(tr("Strg+E", "Ctrl+E"))
        .clicked()
    {
        actions.push(Action::Export(
            presets.to_vec(),
            crate::actions::suggested_name(presets),
        ));
    }
    ui.add_space(14.0);
    if danger_button(
        ui,
        &tr_format!(
            "{}  {} in den Papierkorb …",
            "{}  Move {} to Recycle Bin …",
            icon::TRASH,
            presets.len()
        ),
    )
    .clicked()
    {
        actions.push(Action::DeleteSelection);
    }
}
