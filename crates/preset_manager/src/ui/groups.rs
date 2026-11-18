//! Linke Spalte: Gruppen, getrennt nach Presets und Profilen (F-32), darunter
//! deutlich abgesetzt die deaktivierten (F-48): Eine Gruppe mit aktiven und
//! deaktivierten Einträgen steht in beiden Teilen, jeweils mit ihrer Anzahl.
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
    Select(Place, bool),
    NewGroup(Section),
    StartRename(Section, String),
    CommitRename,
    CancelRename,
    Drop(Arc<DragPayload>, Section, String, bool),
    DropNew(Vec<Preset>, Section),
    Delete(Section, String, bool),
    SetActive(Section, String, bool),
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
        group_rows(
            ui,
            app,
            section,
            true,
            dragged_items.as_deref(),
            &mut actions,
        );
    }

    if app.has_inactive() {
        inactive_header(ui);
        for section in [Section::Presets, Section::Profiles] {
            if !app.filter.shows(section) || app.group_names_in_state(section, false).is_empty() {
                continue;
            }
            ui.add_space(4.0);
            ui.label(
                RichText::new(match section {
                    Section::Presets => texts::section_presets(),
                    Section::Profiles => texts::section_profiles(),
                })
                .small()
                .color(theme::FAINT),
            );
            group_rows(
                ui,
                app,
                section,
                false,
                dragged_items.as_deref(),
                &mut actions,
            );
        }
    }

    problems_section(ui, app, searching, &mut actions);

    for action in actions {
        apply(app, action, now);
    }
}

/// Die Gruppen eines Bereichs in einem Zustand (aktiv/deaktiviert).
fn group_rows(
    ui: &mut Ui,
    app: &mut App,
    section: Section,
    active: bool,
    dragged_items: Option<&[Preset]>,
    actions: &mut Vec<Action>,
) {
    let searching = app.searching();
    let (glyph, color) = widgets::section_icon(section);

    for name in app.group_names_in_state(section, active) {
        let place = Place::Group {
            section,
            name: name.clone(),
        };
        let (count, from_adobe) = {
            let presets = app.group_presets(section, &name, active);
            let from_adobe = !presets.is_empty()
                && presets
                    .iter()
                    .all(|preset| preset.origin == PresetOrigin::ImportedSettings);
            (presets.len(), from_adobe)
        };
        let is_empty_group = active && app.is_empty_group(section, &name);
        let selected =
            !searching && app.place.as_ref() == Some(&place) && app.place_active == active;
        // Umbenannt wird in der Zeile der angezeigten Ansicht.
        let renaming = app.place_active == active
            && matches!(
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
                if active {
                    widgets::glyph(ui, glyph, color);
                } else {
                    widgets::glyph(ui, icon::EYE_SLASH, theme::FAINT)
                        .on_hover_text(texts::inactive_hint());
                }
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
                        } else if !active {
                            text.color(theme::MUTED)
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
            match drop_problem(app, &hovered, section, &name, active) {
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
            actions.push(Action::Drop(dropped, section, name.clone(), active));
        }

        if response.clicked() && !renaming {
            actions.push(Action::Select(place.clone(), active));
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
            if !is_empty_group {
                let (glyph, text) = if active {
                    (icon::EYE_SLASH, texts::deactivate_group())
                } else {
                    (icon::EYE, texts::activate_group())
                };
                if ui.button(format!("{glyph}  {text}")).clicked() {
                    actions.push(Action::SetActive(section, name.clone(), !active));
                    ui.close();
                }
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
                actions.push(Action::Delete(section, name.clone(), active));
                ui.close();
            }
        });

        app.group_rects.push((place, response.rect));
    }

    // „＋ Neue Gruppe“ als Ablageziel, solange Einträge gezogen werden.
    if let (true, Some(items)) = (active, dragged_items) {
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
            actions.push(Action::DropNew(items.to_vec(), section));
        }
    }
}

/// Trennt die deaktivierten Gruppen deutlich von den aktiven.
fn inactive_header(ui: &mut Ui) {
    ui.add_space(14.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::LINE);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        widgets::glyph(ui, icon::EYE_SLASH, theme::MUTED);
        ui.label(
            RichText::new(texts::section_inactive())
                .small()
                .strong()
                .color(theme::MUTED),
        );
    })
    .response
    .on_hover_text(texts::inactive_hint());
    ui.label(
        RichText::new(texts::inactive_hint())
            .small()
            .color(theme::FAINT),
    );
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
        actions.push(Action::Select(Place::Problems, true));
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
fn drop_problem(
    app: &App,
    payload: &DragPayload,
    section: Section,
    name: &str,
    active: bool,
) -> Option<String> {
    match payload {
        DragPayload::Items(items) => app.move_problem_in(items, section, name, Some(active)),
        DragPayload::Group {
            section: source_section,
            name: source,
        } => app.merge_problem(*source_section, source, section, name),
    }
}

fn apply(app: &mut App, action: Action, now: f64) {
    match action {
        Action::Select(place, active) => app.select_place_in(place, active),
        Action::NewGroup(section) => app.create_empty_group(section),
        Action::StartRename(section, name) => app.start_rename_group(section, &name),
        Action::CommitRename => app.commit_rename(now),
        Action::CancelRename => app.rename = None,
        Action::Drop(payload, section, name, active) => {
            app.drop_on_group(&payload, section, &name, active, now)
        }
        Action::DropNew(items, section) => app.drop_on_new_group(&items, section, now),
        Action::Delete(section, name, active) => {
            app.request_delete_group_in(section, &name, active, now)
        }
        Action::SetActive(section, name, active) => {
            app.set_group_active(section, &name, active, now)
        }
    }
}
