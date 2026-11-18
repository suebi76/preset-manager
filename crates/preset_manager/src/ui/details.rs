//! Rechte Spalte: Details und Aktionen für die Auswahl oder die Gruppe.
//!
//! - ein Eintrag: Art, Name, Gruppe (änderbar), Herkunft, Datei, UUID
//! - mehrere Einträge: Anzahl je Art, verschieben, neue Gruppe, löschen
//! - keine Auswahl: die Gruppe – umbenennen, löschen

use eframe::egui::{self, Frame, Margin, RichText, ScrollArea, Ui};
use egui_phosphor::regular as icon;
use preset_core::preset::{Preset, Section};

use crate::actions::common_section;
use crate::app::{is_adobe_origin, show_in_explorer, App, Place, ToastKind};
use crate::texts;
use crate::ui::{theme, widgets};
use crate::worker::Manage;

pub fn show(ui: &mut Ui, app: &mut App, now: f64) {
    egui::Panel::right("details")
        .resizable(true)
        .default_size(280.0)
        .size_range(220.0..=420.0)
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .inner_margin(Margin::same(12)),
        )
        .show(ui, |ui| {
            ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| contents(ui, app, now));
        });
}

/// Was in der Detailspalte ausgelöst wurde.
enum Action {
    Rename(Preset),
    MoveTo(Vec<Preset>, String),
    GroupFromSelection,
    Explorer(std::path::PathBuf),
    DeleteSelection,
    RenameGroup(Section, String),
    DeleteGroup(Section, String),
}

fn contents(ui: &mut Ui, app: &mut App, now: f64) {
    let mut actions = Vec::new();
    let selected = app.selected_presets();
    match selected.as_slice() {
        [] => match app.place.clone() {
            Some(Place::Group { section, name }) => {
                group_details(ui, app, section, &name, &mut actions)
            }
            Some(Place::Problems) => {
                heading(ui, texts::DETAILS);
                ui.label(RichText::new(texts::PROBLEMS_HINT).color(theme::MUTED));
            }
            None => {}
        },
        [preset] => preset_details(ui, app, preset, &mut actions),
        many => multi_details(ui, app, many, &mut actions),
    }
    for action in actions {
        match action {
            Action::Rename(preset) => app.start_rename_preset(&preset),
            Action::MoveTo(presets, group) => {
                let section = common_section(&presets).unwrap_or(Section::Presets);
                if let Some(problem) = app.move_problem(&presets, section, &group) {
                    app.toast(problem, ToastKind::Error, now);
                } else {
                    app.manage(Manage::Move { presets, group }, now);
                }
            }
            Action::GroupFromSelection => app.group_from_selection(now),
            Action::Explorer(path) => {
                if let Err(message) = show_in_explorer(&path) {
                    app.toast(message, ToastKind::Error, now);
                }
            }
            Action::DeleteSelection => app.request_delete_selection(),
            Action::RenameGroup(section, name) => app.start_rename_group(section, &name),
            Action::DeleteGroup(section, name) => app.request_delete_group(section, &name, now),
        }
    }
}

fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().strong().color(theme::MUTED));
    ui.add_space(8.0);
}

fn preset_details(ui: &mut Ui, app: &App, preset: &Preset, actions: &mut Vec<Action>) {
    heading(ui, texts::DETAILS);
    let (glyph, color) = widgets::type_icon(&preset.preset_type);
    type_badge(
        ui,
        glyph,
        color,
        widgets::type_label(&preset.preset_type),
        where_in_camera_raw(preset.section()),
    );

    label(ui, "Name");
    ui.horizontal(|ui| {
        widgets::truncated(ui, RichText::new(&preset.name).strong());
    });
    if ui
        .button(format!("{}  {}  (F2)", icon::PENCIL_SIMPLE, texts::RENAME))
        .clicked()
    {
        actions.push(Action::Rename(preset.clone()));
    }
    ui.add_space(8.0);

    label(ui, "Gruppe");
    if let Some(group) = group_picker(
        ui,
        app,
        preset.section(),
        &preset.group,
        "group-of-selection",
    ) {
        actions.push(Action::MoveTo(vec![preset.clone()], group));
    }
    ui.add_space(8.0);

    let origin = if is_adobe_origin(preset) {
        texts::ORIGIN_ADOBE
    } else {
        texts::ORIGIN_OWN
    };
    field(ui, "Herkunft", origin);
    mono_field(ui, "Datei", &preset.path.display().to_string());
    mono_field(ui, "UUID", preset.uuid.as_deref().unwrap_or("–"));
    ui.add_space(8.0);
    if widgets::icon_button(ui, icon::FOLDER_OPEN, texts::SHOW_IN_EXPLORER).clicked() {
        actions.push(Action::Explorer(preset.path.clone()));
    }
    ui.add_space(4.0);
    if danger_button(ui, &format!("{}  {}", icon::TRASH, texts::TO_TRASH)).clicked() {
        actions.push(Action::DeleteSelection);
    }
}

fn multi_details(ui: &mut Ui, app: &App, presets: &[Preset], actions: &mut Vec<Action>) {
    heading(ui, &format!("{} AUSGEWÄHLT", presets.len()));
    let preset_count = presets
        .iter()
        .filter(|p| p.section() == Section::Presets)
        .count();
    ui.horizontal(|ui| {
        widgets::glyph(ui, icon::SLIDERS_HORIZONTAL, theme::PRESET);
        ui.label(texts::plural(preset_count, "Preset", "Presets"));
        ui.add_space(10.0);
        widgets::glyph(ui, icon::APERTURE, theme::PROFILE);
        ui.label(texts::plural(
            presets.len() - preset_count,
            "Profil",
            "Profile",
        ));
    });
    ui.add_space(10.0);

    match common_section(presets) {
        Some(section) => {
            label(ui, texts::MOVE_TO);
            if let Some(group) =
                group_picker(ui, app, section, texts::CHOOSE_GROUP, "move-selection")
            {
                actions.push(Action::MoveTo(presets.to_vec(), group));
            }
            ui.add_space(8.0);
            if ui
                .button(format!(
                    "{}  {}  (Strg+G)",
                    icon::FOLDER_SIMPLE,
                    texts::NEW_GROUP_FROM_SELECTION
                ))
                .clicked()
            {
                actions.push(Action::GroupFromSelection);
            }
        }
        None => {
            ui.label(
                RichText::new(texts::MIXED_SECTIONS)
                    .small()
                    .color(theme::MUTED),
            );
        }
    }
    ui.add_space(4.0);
    if danger_button(
        ui,
        &format!("{}  {} in den Papierkorb …", icon::TRASH, presets.len()),
    )
    .clicked()
    {
        actions.push(Action::DeleteSelection);
    }
}

fn group_details(ui: &mut Ui, app: &App, section: Section, name: &str, actions: &mut Vec<Action>) {
    heading(ui, texts::GROUP);
    let (glyph, color) = widgets::section_icon(section);
    let empty = app.is_empty_group(section, name);
    let count = app
        .library
        .as_ref()
        .and_then(|library| library.group(section, name))
        .map_or(0, |group| group.presets.len());
    let kind = match section {
        Section::Presets => texts::plural(count, "Preset", "Presets"),
        Section::Profiles => texts::plural(count, "Profil", "Profile"),
    };
    let subtitle = if empty {
        "Neu – noch leer".to_owned()
    } else {
        format!("{kind} · {}", where_in_camera_raw(section))
    };
    let title = if name.is_empty() {
        texts::NO_GROUP
    } else {
        name
    };
    type_badge(ui, glyph, color, title, &subtitle);

    if ui
        .button(format!(
            "{}  {}  (F2)",
            icon::PENCIL_SIMPLE,
            texts::RENAME_GROUP
        ))
        .clicked()
    {
        actions.push(Action::RenameGroup(section, name.to_owned()));
    }
    ui.add_space(4.0);
    let delete_text = if empty {
        texts::REMOVE_EMPTY_GROUP
    } else {
        texts::GROUP_TO_TRASH
    };
    if danger_button(ui, &format!("{}  {delete_text}", icon::TRASH)).clicked() {
        actions.push(Action::DeleteGroup(section, name.to_owned()));
    }
    ui.add_space(12.0);
    ui.label(RichText::new(texts::MERGE_HINT).small().color(theme::FAINT));
}

/// Auswahlliste der Gruppen eines Bereichs. Gibt die neu gewählte Gruppe zurück.
fn group_picker(
    ui: &mut Ui,
    app: &App,
    section: Section,
    current: &str,
    id: &str,
) -> Option<String> {
    let mut chosen = None;
    let shown = if current.is_empty() {
        texts::NO_GROUP
    } else {
        current
    };
    egui::ComboBox::from_id_salt(id)
        .selected_text(shown)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for name in app.group_names(section) {
                let label = if name.is_empty() {
                    texts::NO_GROUP.to_owned()
                } else {
                    name.clone()
                };
                if ui
                    .add(egui::Button::selectable(name == current, label))
                    .clicked()
                    && name != current
                {
                    chosen = Some(name);
                }
            }
        });
    chosen
}

fn where_in_camera_raw(section: Section) -> &'static str {
    match section {
        Section::Presets => texts::WHERE_PRESETS,
        Section::Profiles => texts::WHERE_PROFILES,
    }
}

fn danger_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(theme::ERROR))
            .stroke(egui::Stroke::new(1.0, theme::ERROR.gamma_multiply(0.6))),
    )
}

fn type_badge(ui: &mut Ui, glyph: &str, color: egui::Color32, title: &str, subtitle: &str) {
    Frame::new()
        .fill(theme::RAISED)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(glyph).size(22.0).color(color));
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).strong());
                    ui.label(RichText::new(subtitle).small().color(theme::MUTED));
                });
            });
        });
    ui.add_space(12.0);
}

fn label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::MUTED));
}

fn field(ui: &mut Ui, title: &str, value: &str) {
    label(ui, title);
    ui.label(value);
    ui.add_space(6.0);
}

fn mono_field(ui: &mut Ui, title: &str, value: &str) {
    label(ui, title);
    ui.add(egui::Label::new(RichText::new(value).monospace().size(11.0)).wrap());
    ui.add_space(6.0);
}
