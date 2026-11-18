//! Linke Spalte: Gruppen, getrennt nach Presets und Profilen (F-32).
//!
//! Gruppen sind Ablageziele: Einträge aus der Liste lassen sich darauf ziehen
//! (verschieben, F-34), Gruppen auf Gruppen (zusammenführen, F-35). Während
//! Einträge gezogen werden, erscheint je Bereich „＋ Neue Gruppe“.

use std::sync::Arc;

use eframe::egui::{self, DragAndDrop, Frame, Margin, RichText, ScrollArea, Sense, Ui};
use egui_phosphor::regular as icon;
use preset_core::preset::{Preset, PresetOrigin, Section};

use crate::actions::common_section;
use crate::app::{App, DragPayload, Place, RenameTarget};
use crate::texts::{self, tr};
use crate::ui::{theme, widgets};

pub fn show(ui: &mut Ui, app: &mut App, now: f64) {
    egui::Panel::left("groups")
        .resizable(true)
        .default_size(250.0)
        .size_range(180.0..=440.0)
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .inner_margin(Margin::symmetric(6, 6)),
        )
        .show(ui, |ui| {
            ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| contents(ui, app, now));
        });
}

/// Was in der Gruppenliste ausgelöst wurde. Ausgeführt wird erst nach dem
/// Zeichnen, damit der Zustand während des Zeichnens unverändert bleibt.
enum Action {
    Select(Place),
    NewGroup(Section),
    StartRename(Section, String),
    CommitRename,
    CancelRename,
    Drop(Arc<DragPayload>, Section, String),
    DropNew(Vec<Preset>, Section),
    Delete(Section, String),
}

fn contents(ui: &mut Ui, app: &mut App, now: f64) {
    app.group_rects.clear();
    if app.library.is_none() {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new(texts::loading()).color(theme::MUTED));
        });
        return;
    }

    let payload = DragAndDrop::payload::<DragPayload>(ui.ctx());
    let dragged_items: Option<Vec<Preset>> = match payload.as_deref() {
        Some(DragPayload::Items(items)) => Some(items.clone()),
        _ => None,
    };
    let mut actions = Vec::new();
    let searching = app.searching();

    for section in [Section::Presets, Section::Profiles] {
        if !app.filter.shows(section) {
            continue;
        }
        section_header(ui, section, &mut actions);
        let (glyph, color) = widgets::section_icon(section);

        for name in app.group_names(section) {
            let place = Place::Group {
                section,
                name: name.clone(),
            };
            let presets: &[Preset] = app
                .library
                .as_ref()
                .and_then(|library| library.group(section, &name))
                .map_or(&[], |group| group.presets.as_slice());
            let count = presets.len();
            let is_empty_group = app.is_empty_group(section, &name);
            let from_adobe = count > 0
                && presets
                    .iter()
                    .all(|preset| preset.origin == PresetOrigin::ImportedSettings);
            let selected = !searching && app.place.as_ref() == Some(&place);
            let renaming = matches!(
                &app.rename,
                Some(rename) if rename.target == RenameTarget::Group { section, name: name.clone() }
            );
            let label = if name.is_empty() {
                texts::no_group()
            } else {
                name.as_str()
            };
            let sense = if is_empty_group || renaming {
                Sense::click()
            } else {
                Sense::click_and_drag()
            };

            let mut rename_outcome = None;
            let response = widgets::list_row(
                ui,
                selected,
                sense,
                |ui| {
                    let count_text = if is_empty_group {
                        tr("leer", "empty").to_owned()
                    } else {
                        count.to_string()
                    };
                    ui.label(RichText::new(count_text).color(theme::MUTED));
                    if from_adobe {
                        ui.label(RichText::new(icon::DOWNLOAD_SIMPLE).color(theme::FAINT))
                            .on_hover_text(texts::origin_adobe());
                    }
                },
                |ui| {
                    widgets::glyph(ui, glyph, color);
                    match &mut app.rename {
                        Some(rename) if renaming => {
                            rename_outcome = Some(widgets::rename_field(
                                ui,
                                &mut rename.text,
                                &mut rename.focus,
                            ));
                        }
                        _ => {
                            let text = RichText::new(label);
                            let text = if is_empty_group {
                                text.color(theme::MUTED).italics()
                            } else {
                                text
                            };
                            widgets::truncated(ui, text);
                        }
                    }
                },
            );
            if is_empty_group {
                ui.painter().rect_stroke(
                    response.rect,
                    egui::CornerRadius::same(4),
                    egui::Stroke::new(1.0, theme::FAINT),
                    egui::StrokeKind::Inside,
                );
            }
            match rename_outcome {
                Some(widgets::RenameOutcome::Commit) => actions.push(Action::CommitRename),
                Some(widgets::RenameOutcome::Cancel) => actions.push(Action::CancelRename),
                _ => {}
            }

            // Ziehen und Ablegen
            if !is_empty_group && !renaming {
                response.dnd_set_drag_payload(DragPayload::Group {
                    section,
                    name: name.clone(),
                });
            }
            if let Some(hovered) = response.dnd_hover_payload::<DragPayload>() {
                match drop_problem(app, &hovered, section, &name) {
                    None => widgets::paint_drop_ok(ui, response.rect),
                    Some(problem) => {
                        widgets::paint_drop_bad(ui, response.rect);
                        if !problem.is_empty() {
                            app.drag_hint = Some(problem);
                        }
                    }
                }
            }
            if let Some(dropped) = response.dnd_release_payload::<DragPayload>() {
                actions.push(Action::Drop(dropped, section, name.clone()));
            }

            if response.clicked() && !renaming {
                actions.push(Action::Select(place.clone()));
            }
            if response.double_clicked() && !is_empty_group {
                actions.push(Action::StartRename(section, name.clone()));
            }
            response.context_menu(|ui| {
                if ui
                    .button(format!("{}  {}", icon::PENCIL_SIMPLE, texts::rename()))
                    .clicked()
                {
                    actions.push(Action::StartRename(section, name.clone()));
                    ui.close();
                }
                ui.separator();
                let delete_text = if is_empty_group {
                    texts::remove_empty_group()
                } else {
                    texts::group_to_trash()
                };
                if ui
                    .button(format!("{}  {delete_text}", icon::TRASH))
                    .clicked()
                {
                    actions.push(Action::Delete(section, name.clone()));
                    ui.close();
                }
            });

            app.group_rects.push((place, response.rect));
        }

        // „＋ Neue Gruppe“ als Ablageziel, solange Einträge gezogen werden.
        if let Some(items) = &dragged_items {
            let response = widgets::list_row(
                ui,
                false,
                Sense::hover(),
                |_| {},
                |ui| {
                    widgets::glyph(ui, icon::PLUS, theme::ACCENT);
                    ui.label(RichText::new(texts::new_group()).color(theme::ACCENT));
                },
            );
            if response.dnd_hover_payload::<DragPayload>().is_some() {
                match new_group_problem(items, section) {
                    None => widgets::paint_drop_ok(ui, response.rect),
                    Some(problem) => {
                        widgets::paint_drop_bad(ui, response.rect);
                        app.drag_hint = Some(problem.to_owned());
                    }
                }
            }
            if response.dnd_release_payload::<DragPayload>().is_some() {
                actions.push(Action::DropNew(items.clone(), section));
            }
        }
    }

    problems_section(ui, app, searching, &mut actions);

    ui.add_space(14.0);
    ui.label(
        RichText::new(texts::group_hint())
            .small()
            .color(theme::FAINT),
    );

    for action in actions {
        apply(app, action, now);
    }
}

fn section_header(ui: &mut Ui, section: Section, actions: &mut Vec<Action>) {
    let (heading, new_group_tip) = match section {
        Section::Presets => (
            texts::section_presets(),
            tr("Neue Preset-Gruppe", "New preset group"),
        ),
        Section::Profiles => (
            texts::section_profiles(),
            tr("Neue Profil-Gruppe", "New profile group"),
        ),
    };
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(heading).small().strong().color(theme::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button =
                egui::Button::new(RichText::new(icon::PLUS).color(theme::MUTED)).frame(false);
            if ui.add(button).on_hover_text(new_group_tip).clicked() {
                actions.push(Action::NewGroup(section));
            }
        });
    });
}

fn problems_section(ui: &mut Ui, app: &App, searching: bool, actions: &mut Vec<Action>) {
    if app.problems.is_empty() {
        return;
    }
    widgets::section_heading(ui, texts::section_problems());
    let selected = !searching && app.place == Some(Place::Problems);
    let count = app.problems.len();
    let response = widgets::list_row(
        ui,
        selected,
        Sense::click(),
        |ui| {
            ui.label(RichText::new(count.to_string()).color(theme::MUTED));
        },
        |ui| {
            widgets::glyph(ui, icon::WARNING, theme::WARN);
            widgets::truncated(ui, texts::problems_group());
        },
    );
    if response.clicked() {
        actions.push(Action::Select(Place::Problems));
    }
}

/// Warum die Einträge nicht in eine neue Gruppe dieses Bereichs dürfen.
fn new_group_problem(items: &[Preset], section: Section) -> Option<&'static str> {
    match common_section(items) {
        None => Some(texts::mixed_sections()),
        Some(own) if own == section => None,
        Some(Section::Profiles) => Some(texts::profile_into_preset_group()),
        Some(Section::Presets) => Some(texts::preset_into_profile_group()),
    }
}

/// Warum das Gezogene nicht auf diese Gruppe darf – `None`, wenn es geht,
/// leerer Text, wenn es nur sinnlos wäre (Gruppe auf sich selbst).
fn drop_problem(app: &App, payload: &DragPayload, section: Section, name: &str) -> Option<String> {
    match payload {
        DragPayload::Items(items) => app.move_problem(items, section, name),
        DragPayload::Group {
            section: source_section,
            name: source,
        } => app.merge_problem(*source_section, source, section, name),
    }
}

fn apply(app: &mut App, action: Action, now: f64) {
    match action {
        Action::Select(place) => app.select_place(place),
        Action::NewGroup(section) => app.create_empty_group(section),
        Action::StartRename(section, name) => app.start_rename_group(section, &name),
        Action::CommitRename => app.commit_rename(now),
        Action::CancelRename => app.rename = None,
        Action::Drop(payload, section, name) => app.drop_on_group(&payload, section, &name, now),
        Action::DropNew(items, section) => app.drop_on_new_group(&items, section, now),
        Action::Delete(section, name) => app.request_delete_group(section, &name, now),
    }
}
