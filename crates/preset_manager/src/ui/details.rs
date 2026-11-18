//! Rechte Spalte: Details und Aktionen für die Auswahl oder die Gruppe.
//!
//! - ein Eintrag: „Wirkung“ – was das Preset einstellt ([`preset_view`]) –
//!   und „Verwalten“ – Name, Gruppe (änderbar), Herkunft, Datei, UUID
//! - mehrere Einträge: Anzahl je Art, verschieben, neue Gruppe, löschen
//! - keine Auswahl: die Gruppe – umbenennen, löschen

use eframe::egui::{self, Frame, Margin, RichText, ScrollArea, Ui};
use egui_phosphor::regular as icon;
use preset_core::preset::{Preset, Section};

use crate::actions::common_section;
use crate::app::{is_adobe_origin, show_in_explorer, App, Place, ToastKind};
use crate::texts::{self, tr, tr_format};
use crate::ui::preset_view::{self, SettingsCache};
use crate::ui::{theme, widgets};
use crate::worker::Manage;

pub fn show(ui: &mut Ui, app: &mut App, now: f64) {
    egui::Panel::right("details")
        .resizable(true)
        .default_size(310.0)
        .size_range(220.0..=520.0)
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

/// Welche Seite der Details für ein einzelnes Preset gezeigt wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailsTab {
    /// Was das Preset einstellt.
    #[default]
    Effect,
    /// Name, Gruppe, Datei.
    Manage,
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
    SetActive(Vec<Preset>, bool),
    SetGroupActive(Section, String, bool),
}

fn contents(ui: &mut Ui, app: &mut App, now: f64) {
    // Platz für die Bildlaufleiste, damit sie keine Werte verdeckt.
    ui.set_max_width(ui.available_width() - 10.0);
    let mut actions = Vec::new();
    let selected = app.selected_presets();
    match selected.as_slice() {
        [] => match app.place.clone() {
            Some(Place::Group { section, name }) => {
                group_details(ui, app, section, &name, &mut actions)
            }
            Some(Place::Problems) => {
                heading(ui, texts::details());
                ui.label(RichText::new(texts::problems_hint()).color(theme::MUTED));
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
            Action::SetActive(presets, active) => app.set_presets_active(presets, active, now),
            Action::SetGroupActive(section, name, active) => {
                app.set_group_active(section, &name, active, now)
            }
        }
    }
}

fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().strong().color(theme::MUTED));
    ui.add_space(8.0);
}

fn preset_details(ui: &mut Ui, app: &mut App, preset: &Preset, actions: &mut Vec<Action>) {
    heading(ui, texts::details());
    let (glyph, color) = widgets::type_icon(&preset.preset_type);
    type_badge(
        ui,
        glyph,
        color,
        widgets::type_label(&preset.preset_type),
        where_in_camera_raw(preset.section()),
    );
    state_row(ui, std::slice::from_ref(preset), actions);

    ui.horizontal(|ui| {
        for (tab, title) in [
            (DetailsTab::Effect, tr("Wirkung", "Effect")),
            (DetailsTab::Manage, tr("Verwalten", "Manage")),
        ] {
            if ui
                .add(egui::Button::selectable(app.details_tab == tab, title))
                .clicked()
            {
                app.details_tab = tab;
            }
        }
    });
    ui.add_space(10.0);

    match app.details_tab {
        DetailsTab::Effect => effect_details(ui, app, preset),
        DetailsTab::Manage => manage_details(ui, app, preset, actions),
    }
}

fn effect_details(ui: &mut Ui, app: &mut App, preset: &Preset) {
    match SettingsCache::get(&mut app.settings_cache, &preset.path) {
        Ok(settings) => {
            preset_view::summary(ui, settings);
            ui.add_space(12.0);
            preset_view::panels(ui, settings);
        }
        Err(message) => {
            ui.label(RichText::new(message).color(theme::ERROR));
        }
    }
}

fn manage_details(ui: &mut Ui, app: &App, preset: &Preset, actions: &mut Vec<Action>) {
    label(ui, tr("Name", "Name"));
    ui.horizontal(|ui| {
        widgets::truncated(ui, RichText::new(&preset.name).strong());
    });
    if ui
        .button(format!(
            "{}  {}  (F2)",
            icon::PENCIL_SIMPLE,
            texts::rename()
        ))
        .clicked()
    {
        actions.push(Action::Rename(preset.clone()));
    }
    ui.add_space(8.0);

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
    ui.add_space(8.0);

    let origin = if is_adobe_origin(preset) {
        texts::origin_adobe()
    } else {
        texts::origin_own()
    };
    field(ui, tr("Herkunft", "Origin"), origin);
    mono_field(ui, tr("Datei", "File"), &preset.path.display().to_string());
    mono_field(ui, "UUID", preset.uuid.as_deref().unwrap_or("–"));
    ui.add_space(8.0);
    if widgets::icon_button(ui, icon::FOLDER_OPEN, texts::show_in_explorer()).clicked() {
        actions.push(Action::Explorer(preset.path.clone()));
    }
    ui.add_space(4.0);
    if danger_button(ui, &format!("{}  {}", icon::TRASH, texts::to_trash())).clicked() {
        actions.push(Action::DeleteSelection);
    }
}

fn multi_details(ui: &mut Ui, app: &App, presets: &[Preset], actions: &mut Vec<Action>) {
    heading(
        ui,
        &tr_format!("{} AUSGEWÄHLT", "{} SELECTED", presets.len()),
    );
    let preset_count = presets
        .iter()
        .filter(|p| p.section() == Section::Presets)
        .count();
    ui.horizontal(|ui| {
        widgets::glyph(ui, icon::SLIDERS_HORIZONTAL, theme::PRESET);
        ui.label(texts::presets(preset_count));
        ui.add_space(10.0);
        widgets::glyph(ui, icon::APERTURE, theme::PROFILE);
        ui.label(texts::profiles(presets.len() - preset_count));
    });
    ui.add_space(10.0);
    state_row(ui, presets, actions);

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
                    "{}  {}  ({})",
                    icon::FOLDER_SIMPLE,
                    texts::new_group_from_selection(),
                    tr("Strg+G", "Ctrl+G")
                ))
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
    ui.add_space(4.0);
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

fn group_details(ui: &mut Ui, app: &App, section: Section, name: &str, actions: &mut Vec<Action>) {
    heading(ui, texts::group());
    let (glyph, color) = widgets::section_icon(section);
    let empty = app.is_empty_group(section, name);
    let count = app
        .library
        .as_ref()
        .and_then(|library| library.group(section, name))
        .map_or(0, |group| group.presets.len());
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
    type_badge(ui, glyph, color, title, &subtitle);
    if !empty {
        group_state_row(ui, app, section, name, actions);
    }

    if ui
        .button(format!(
            "{}  {}  (F2)",
            icon::PENCIL_SIMPLE,
            texts::rename_group()
        ))
        .clicked()
    {
        actions.push(Action::RenameGroup(section, name.to_owned()));
    }
    ui.add_space(4.0);
    let delete_text = if empty {
        texts::remove_empty_group()
    } else {
        texts::group_to_trash()
    };
    if danger_button(ui, &format!("{}  {delete_text}", icon::TRASH)).clicked() {
        actions.push(Action::DeleteGroup(section, name.to_owned()));
    }
    ui.add_space(12.0);
    ui.label(
        RichText::new(texts::merge_hint())
            .small()
            .color(theme::FAINT),
    );
}

/// Aktiv oder deaktiviert – mit Schalter. Bei gemischter Auswahl beides.
fn state_row(ui: &mut Ui, presets: &[Preset], actions: &mut Vec<Action>) {
    let active = presets.iter().filter(|preset| preset.active).count();
    let inactive = presets.len() - active;
    Frame::new()
        .fill(if inactive == 0 {
            theme::RAISED
        } else {
            theme::WARN_BG
        })
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let (glyph, color, text) = match (active, inactive) {
                (_, 0) => (
                    icon::EYE,
                    theme::OK_TEXT,
                    tr(
                        "Aktiv – erscheint in Photoshop und Lightroom",
                        "Active – shown in Photoshop and Lightroom",
                    )
                    .to_owned(),
                ),
                (0, _) => (
                    icon::EYE_SLASH,
                    theme::WARN_TEXT,
                    tr(
                        "Deaktiviert – erscheint nicht in Photoshop und Lightroom",
                        "Deactivated – not shown in Photoshop and Lightroom",
                    )
                    .to_owned(),
                ),
                _ => (
                    icon::EYE_SLASH,
                    theme::WARN_TEXT,
                    tr_format!(
                        "{active} aktiv, {inactive} deaktiviert",
                        "{active} active, {inactive} deactivated"
                    ),
                ),
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(glyph).color(color));
                ui.label(RichText::new(text).color(color));
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if inactive > 0
                    && ui
                        .button(format!("{}  {}", icon::EYE, texts::activate()))
                        .clicked()
                {
                    actions.push(Action::SetActive(presets.to_vec(), true));
                }
                if active > 0
                    && ui
                        .button(format!("{}  {}", icon::EYE_SLASH, texts::deactivate()))
                        .on_hover_text(tr("Strg+D", "Ctrl+D"))
                        .clicked()
                {
                    actions.push(Action::SetActive(presets.to_vec(), false));
                }
            });
        });
    ui.add_space(10.0);
}

/// Zustand einer Gruppe: wie viele aktiv, wie viele deaktiviert.
fn group_state_row(
    ui: &mut Ui,
    app: &App,
    section: Section,
    name: &str,
    actions: &mut Vec<Action>,
) {
    let active = app.group_presets(section, name, true).len();
    let inactive = app.group_presets(section, name, false).len();
    ui.label(
        RichText::new(tr_format!(
            "{active} aktiv · {inactive} deaktiviert",
            "{active} active · {inactive} deactivated"
        ))
        .small()
        .color(theme::MUTED),
    );
    ui.horizontal(|ui| {
        if inactive > 0
            && ui
                .button(format!("{}  {}", icon::EYE, texts::activate_group()))
                .clicked()
        {
            actions.push(Action::SetGroupActive(section, name.to_owned(), true));
        }
        if active > 0
            && ui
                .button(format!(
                    "{}  {}",
                    icon::EYE_SLASH,
                    texts::deactivate_group()
                ))
                .clicked()
        {
            actions.push(Action::SetGroupActive(section, name.to_owned(), false));
        }
    });
    ui.add_space(8.0);
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
        texts::no_group()
    } else {
        current
    };
    egui::ComboBox::from_id_salt(id)
        .selected_text(shown)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for name in app.group_names(section) {
                let label = if name.is_empty() {
                    texts::no_group().to_owned()
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
        Section::Presets => texts::where_presets(),
        Section::Profiles => texts::where_profiles(),
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
