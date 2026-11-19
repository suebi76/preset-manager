//! Rechte Spalte: Details und Aktionen für die Auswahl oder die Gruppe.
//!
//! - ein Eintrag: „Wirkung“ – was das Preset einstellt ([`preset_view`]) –
//!   und „Verwalten“ – Name, Gruppe (änderbar), Herkunft, Datei, UUID
//! - mehrere Einträge: Anzahl je Art, verschieben, neue Gruppe, löschen
//! - keine Auswahl: die Gruppe – umbenennen, löschen
//!
//! | Datei | Inhalt |
//! |---|---|
//! | `manage` | ein Preset verwalten |
//! | `multi` | Mehrfachauswahl |
//! | `group` | Gruppe |

mod group;
mod manage;
mod multi;

use eframe::egui::{self, Frame, Margin, RichText, ScrollArea, Ui};
use preset_core::preset::{Preset, Section};

use crate::actions::common_section;
use crate::app::{show_in_explorer, App, Place, ToastKind};
use crate::texts::{self, tr, tr_format};
use crate::ui::before_after;
use crate::ui::preset_view::{self, SettingsCache};
use crate::ui::{theme, widgets};
use crate::worker::Manage;
use group::group_details;
use manage::manage_details;
use multi::multi_details;

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
            widgets::scroll(ui, ScrollArea::vertical().auto_shrink(false), |ui| {
                contents(ui, app, now)
            });
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
    Export(Vec<Preset>, String),
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
                heading(ui, texts::details());
                ui.label(RichText::new(texts::problems_hint()).color(theme::MUTED));
            }
            None => {}
        },
        [preset] => preset_details(ui, app, preset, &mut actions, now),
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
            Action::Export(presets, name) => app.export_presets(presets, &name, now),
        }
    }
}

fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().strong().color(theme::MUTED));
    ui.add_space(8.0);
}

fn preset_details(
    ui: &mut Ui,
    app: &mut App,
    preset: &Preset,
    actions: &mut Vec<Action>,
    now: f64,
) {
    let (glyph, color) = widgets::type_icon(&preset.preset_type);
    let group = if preset.group.is_empty() {
        texts::no_group()
    } else {
        &preset.group
    };
    widgets::title_block(
        ui,
        glyph,
        if preset.active {
            color
        } else {
            color.gamma_multiply(0.45)
        },
        &preset.name,
        &format!("{} · {group}", widgets::type_label(&preset.preset_type)),
    );
    ui.add_space(8.0);
    state_switch(ui, std::slice::from_ref(preset), actions);
    ui.add_space(10.0);

    let mut tab = app.details_tab;
    widgets::segmented(
        ui,
        &mut tab,
        &[
            (DetailsTab::Effect, tr("Wirkung", "Effect")),
            (DetailsTab::Manage, tr("Verwalten", "Manage")),
        ],
    );
    app.details_tab = tab;
    ui.add_space(12.0);

    match app.details_tab {
        DetailsTab::Effect => effect_details(ui, app, preset, now),
        DetailsTab::Manage => manage_details(ui, app, preset, actions),
    }
}

fn effect_details(ui: &mut Ui, app: &mut App, preset: &Preset, now: f64) {
    let settings = SettingsCache::get(&mut app.settings_cache, &preset.path).clone();
    match &settings {
        Ok(settings) => {
            preset_view::summary(ui, settings);
            ui.add_space(10.0);
            before_after::show(ui, app, preset, settings, now);
            ui.add_space(6.0);
            preset_view::panels(ui, settings);
        }
        Err(message) => {
            ui.label(RichText::new(message).color(theme::ERROR));
        }
    }
}

/// Aktiv oder deaktiviert als Schalter mit kurzer Erklärung. Bei gemischter
/// Auswahl zeigt er „aus“ und aktiviert beim Klick alle.
fn state_switch(ui: &mut Ui, presets: &[Preset], actions: &mut Vec<Action>) {
    let active = presets.iter().filter(|preset| preset.active).count();
    let inactive = presets.len() - active;
    let all_active = inactive == 0;
    let (text, color) = match (active, inactive) {
        (_, 0) => (
            tr(
                "Aktiv in Photoshop und Lightroom",
                "Active in Photoshop and Lightroom",
            )
            .to_owned(),
            theme::TEXT,
        ),
        (0, _) => (
            tr(
                "Deaktiviert – nicht in Photoshop und Lightroom",
                "Deactivated – not in Photoshop and Lightroom",
            )
            .to_owned(),
            theme::WARN_TEXT,
        ),
        _ => (
            tr_format!(
                "{active} aktiv · {inactive} deaktiviert",
                "{active} active · {inactive} deactivated"
            ),
            theme::WARN_TEXT,
        ),
    };
    ui.horizontal(|ui| {
        let tip = if all_active {
            tr("Deaktivieren (Strg+D)", "Deactivate (Ctrl+D)")
        } else {
            tr("Aktivieren (Strg+D)", "Activate (Ctrl+D)")
        };
        if widgets::toggle_switch(ui, all_active)
            .on_hover_text(tip)
            .clicked()
        {
            actions.push(Action::SetActive(presets.to_vec(), !all_active));
        }
        ui.label(RichText::new(text).color(color));
    });
}

fn fact(ui: &mut Ui, title: &str, value: &str) {
    ui.label(RichText::new(title).small().color(theme::MUTED));
    ui.add(egui::Label::new(value).wrap());
    ui.end_row();
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

fn label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::MUTED));
}
