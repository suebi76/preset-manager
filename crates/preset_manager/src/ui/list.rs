//! Mitte: die Einträge der gewählten Gruppe oder die Suchtreffer.
//!
//! Einträge lassen sich auswählen (Strg/Umschalt für mehrere), ziehen (auf
//! eine Gruppe links oder an eine andere Stelle der Liste), umbenennen (F2,
//! Doppelklick) und über das Kontextmenü bearbeiten.

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, DragAndDrop, Frame, Margin, RichText, ScrollArea, Sense, Ui};
use egui_phosphor::regular as icon;
use preset_core::preset::{Preset, Section};

use crate::app::{
    is_adobe_origin, show_in_explorer, App, DragPayload, Filter, Place, RenameTarget, ToastKind,
};
use crate::texts;
use crate::ui::{theme, widgets};

pub fn show(ui: &mut Ui, app: &mut App, now: f64) {
    egui::CentralPanel::no_frame()
        .frame(Frame::new().fill(theme::BG))
        .show(ui, |ui| {
            header(ui, app);
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(8, 0))
                .show(ui, |ui| body(ui, app, now));
        });
}

fn header(ui: &mut Ui, app: &mut App) {
    let listed = app.listed();
    let presets = listed
        .iter()
        .filter(|p| p.section() == Section::Presets)
        .count();
    let profiles = listed.len() - presets;
    let searching = app.searching();
    let title = if searching {
        format!("Suche „{}“", app.search.trim())
    } else {
        match &app.place {
            Some(Place::Group { name, .. }) if name.is_empty() => texts::NO_GROUP.to_owned(),
            Some(Place::Group { name, .. }) => name.clone(),
            Some(Place::Problems) => texts::PROBLEMS_GROUP.to_owned(),
            None => String::new(),
        }
    };
    let mut subtitle = if searching {
        texts::plural(listed.len(), "Treffer", "Treffer")
    } else if app.place == Some(Place::Problems) {
        texts::plural(app.problems.len(), "Datei", "Dateien")
    } else {
        texts::count_label(presets, profiles)
    };
    if app.selection.len() > 1 {
        subtitle.push_str(&format!(" · {} ausgewählt", app.selection.len()));
    }

    let mut new_filter = None;
    Frame::new()
        .inner_margin(Margin::symmetric(16, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Segmentschalter Alle · Presets · Profile (rechts → links).
                    for (filter, label, glyph, shortcut) in [
                        (
                            Filter::Profiles,
                            texts::FILTER_PROFILES,
                            Some(icon::APERTURE),
                            "Strg+3",
                        ),
                        (
                            Filter::Presets,
                            texts::FILTER_PRESETS,
                            Some(icon::SLIDERS_HORIZONTAL),
                            "Strg+2",
                        ),
                        (Filter::All, texts::FILTER_ALL, None, "Strg+1"),
                    ] {
                        let text = match glyph {
                            Some(glyph) => format!("{glyph} {label}"),
                            None => label.to_owned(),
                        };
                        let button = egui::Button::selectable(app.filter == filter, text);
                        if ui.add(button).on_hover_text(shortcut).clicked() {
                            new_filter = Some(filter);
                        }
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        widgets::truncated(ui, RichText::new(title).size(15.0).strong());
                        ui.label(RichText::new(subtitle).color(theme::MUTED));
                    });
                });
            });
        });
    if let Some(filter) = new_filter {
        app.set_filter(filter);
    }
}

/// Was in der Liste ausgelöst wurde; ausgeführt nach dem Zeichnen.
enum Action {
    Click(PathBuf, egui::Modifiers),
    StartRename(Preset),
    CommitRename,
    CancelRename,
    /// Ziehen beginnt auf einem nicht ausgewählten Eintrag: nur ihn wählen.
    SelectOnly(PathBuf),
    Reorder(Arc<DragPayload>, Option<PathBuf>),
    DropOnEmptyGroup(Arc<DragPayload>),
    GroupFromSelection,
    Explorer(PathBuf),
    Delete,
}

fn body(ui: &mut Ui, app: &mut App, now: f64) {
    if app.library.is_none() {
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new(texts::LOADING).color(theme::MUTED));
        });
        return;
    }
    let searching = app.searching();
    if !searching && app.place == Some(Place::Problems) {
        problems(ui, app);
        return;
    }

    let mut actions = Vec::new();
    let listed: Vec<Preset> = app.listed().into_iter().cloned().collect();

    if listed.is_empty() {
        empty(ui, app, searching, &mut actions);
    } else {
        rows(ui, app, &listed, searching, &mut actions);
    }

    for action in actions {
        apply(app, action, now);
    }
}

fn empty(ui: &mut Ui, app: &mut App, searching: bool, actions: &mut Vec<Action>) {
    if searching {
        widgets::empty_state(
            ui,
            icon::MAGNIFYING_GLASS,
            texts::NOTHING_FOUND,
            texts::NOTHING_FOUND_HINT,
        );
        return;
    }
    let Some(Place::Group { section, name }) = app.place.clone() else {
        widgets::empty_state(
            ui,
            icon::TRAY_ARROW_DOWN,
            texts::EMPTY_LIBRARY,
            texts::EMPTY_LIBRARY_HINT,
        );
        return;
    };
    if !app.is_empty_group(section, &name) {
        widgets::empty_state(
            ui,
            icon::TRAY_ARROW_DOWN,
            texts::EMPTY_LIBRARY,
            texts::EMPTY_LIBRARY_HINT,
        );
        return;
    }
    // Neue, leere Gruppe: große Ablagefläche.
    ui.add_space(16.0);
    let size = egui::vec2(ui.available_width(), 180.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(8),
        egui::Stroke::new(1.5, theme::FAINT),
        egui::StrokeKind::Inside,
    );
    if let Some(hovered) = response.dnd_hover_payload::<DragPayload>() {
        let problem = match hovered.as_ref() {
            DragPayload::Items(items) => app.move_problem(items, section, &name),
            DragPayload::Group { .. } => Some(String::new()),
        };
        match problem {
            None => widgets::paint_drop_ok(ui, rect),
            Some(problem) => {
                widgets::paint_drop_bad(ui, rect);
                if !problem.is_empty() {
                    app.drag_hint = Some(problem);
                }
            }
        }
    }
    if let Some(dropped) = response.dnd_release_payload::<DragPayload>() {
        actions.push(Action::DropOnEmptyGroup(dropped));
    }
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    widgets::empty_state(
        &mut child,
        icon::FOLDER_SIMPLE,
        texts::EMPTY_GROUP_TITLE,
        texts::EMPTY_GROUP_HINT,
    );
}

fn rows(ui: &mut Ui, app: &mut App, listed: &[Preset], searching: bool, actions: &mut Vec<Action>) {
    let dragging = DragAndDrop::has_payload_of_type::<DragPayload>(ui.ctx());
    let modifiers = click_modifiers(ui);
    let reorder_allowed = !searching;

    ScrollArea::vertical().auto_shrink(false).show_rows(
        ui,
        theme::ROW_HEIGHT,
        listed.len(),
        |ui, range| {
            ui.add_space(4.0);
            for (offset, preset) in listed[range.clone()].iter().enumerate() {
                let index = range.start + offset;
                let selected = app.selection.contains(&preset.path);
                let renaming = matches!(
                    &app.rename,
                    Some(rename) if rename.target == RenameTarget::Preset(preset.path.clone())
                );
                let (glyph, color) = widgets::type_icon(&preset.preset_type);
                let tooltip = format!(
                    "{} · {}",
                    widgets::type_label(&preset.preset_type),
                    match preset.section() {
                        Section::Presets => texts::WHERE_PRESETS,
                        Section::Profiles => texts::WHERE_PROFILES,
                    }
                );
                let sense = if renaming {
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
                        if is_adobe_origin(preset) {
                            ui.label(RichText::new(icon::DOWNLOAD_SIMPLE).color(theme::FAINT))
                                .on_hover_text(texts::ORIGIN_ADOBE);
                        }
                        if searching {
                            let group = if preset.group.is_empty() {
                                texts::NO_GROUP
                            } else {
                                &preset.group
                            };
                            ui.label(RichText::new(group).color(theme::MUTED));
                        }
                    },
                    |ui| {
                        ui.label(RichText::new(icon::DOTS_SIX_VERTICAL).color(theme::FAINT))
                            .on_hover_text(texts::DRAG_HINT_REORDER);
                        widgets::glyph(ui, glyph, color).on_hover_text(&tooltip);
                        match &mut app.rename {
                            Some(rename) if renaming => {
                                rename_outcome = Some(widgets::rename_field(
                                    ui,
                                    &mut rename.text,
                                    &mut rename.focus,
                                ));
                            }
                            _ => {
                                widgets::truncated(ui, preset.name.as_str());
                            }
                        }
                    },
                );
                match rename_outcome {
                    Some(widgets::RenameOutcome::Commit) => actions.push(Action::CommitRename),
                    Some(widgets::RenameOutcome::Cancel) => actions.push(Action::CancelRename),
                    _ => {}
                }
                if renaming {
                    continue;
                }

                // Ziehen beginnen: die Auswahl oder – falls dieser Eintrag
                // nicht dazugehört – nur ihn.
                if response.drag_started() {
                    let payload = if selected {
                        app.selected_presets()
                    } else {
                        actions.push(Action::SelectOnly(preset.path.clone()));
                        vec![preset.clone()]
                    };
                    response.dnd_set_drag_payload(DragPayload::Items(payload));
                }

                // Umsortieren innerhalb der Gruppe
                if dragging && reorder_allowed {
                    if let Some(hovered) = response.dnd_hover_payload::<DragPayload>() {
                        if let DragPayload::Items(items) = hovered.as_ref() {
                            let below = pointer_below_middle(ui, response.rect);
                            if items.iter().all(|item| {
                                item.group == preset.group && item.section() == preset.section()
                            }) {
                                widgets::paint_insert_line(ui, response.rect, below);
                            } else {
                                app.drag_hint =
                                    Some("Zum Verschieben auf eine Gruppe links ziehen".to_owned());
                            }
                        }
                    }
                    if let Some(dropped) = response.dnd_release_payload::<DragPayload>() {
                        let below = pointer_below_middle(ui, response.rect);
                        let before = if below {
                            listed.get(index + 1).map(|next| next.path.clone())
                        } else {
                            Some(preset.path.clone())
                        };
                        actions.push(Action::Reorder(dropped, before));
                    }
                }

                if response.clicked() {
                    actions.push(Action::Click(preset.path.clone(), modifiers));
                }
                if response.double_clicked() {
                    actions.push(Action::StartRename(preset.clone()));
                }
                context_menu(&response, app, preset, actions);
            }
        },
    );
}

fn context_menu(response: &egui::Response, app: &App, preset: &Preset, actions: &mut Vec<Action>) {
    let single = app.selection.len() <= 1 || !app.selection.contains(&preset.path);
    response.context_menu(|ui| {
        if !app.selection.contains(&preset.path) {
            actions.push(Action::SelectOnly(preset.path.clone()));
        }
        if ui
            .add_enabled(
                single,
                egui::Button::new(format!("{}  {}  (F2)", icon::PENCIL_SIMPLE, texts::RENAME)),
            )
            .clicked()
        {
            actions.push(Action::StartRename(preset.clone()));
            ui.close();
        }
        if ui
            .button(format!(
                "{}  {}  (Strg+G)",
                icon::FOLDER_SIMPLE,
                texts::NEW_GROUP_FROM_SELECTION
            ))
            .clicked()
        {
            actions.push(Action::GroupFromSelection);
            ui.close();
        }
        if ui
            .add_enabled(
                single,
                egui::Button::new(format!(
                    "{}  {}",
                    icon::FOLDER_OPEN,
                    texts::SHOW_IN_EXPLORER
                )),
            )
            .clicked()
        {
            actions.push(Action::Explorer(preset.path.clone()));
            ui.close();
        }
        ui.separator();
        if ui
            .button(format!("{}  {}  (Entf)", icon::TRASH, texts::TO_TRASH))
            .clicked()
        {
            actions.push(Action::Delete);
            ui.close();
        }
    });
}

/// Die Zusatztasten (Strg, Umschalt) zum Zeitpunkt des Klicks.
///
/// Gelesen aus dem Klick-Ereignis selbst, nicht aus dem Tastenzustand am
/// Ende des Bildes: Wird Strg sehr schnell nach dem Klick losgelassen,
/// zählt es trotzdem.
fn click_modifiers(ui: &Ui) -> egui::Modifiers {
    ui.input(|input| {
        input
            .events
            .iter()
            .rev()
            .find_map(|event| match event {
                egui::Event::PointerButton {
                    pressed: false,
                    modifiers,
                    ..
                } => Some(*modifiers),
                _ => None,
            })
            .unwrap_or(input.modifiers)
    })
}

fn pointer_below_middle(ui: &Ui, rect: egui::Rect) -> bool {
    ui.ctx()
        .pointer_latest_pos()
        .is_some_and(|pos| pos.y > rect.center().y)
}

fn apply(app: &mut App, action: Action, now: f64) {
    match action {
        Action::Click(path, modifiers) => {
            app.click_item(&path, modifiers);
            if app.searching() {
                // Die Gruppe des Treffers links markieren, die Suche bleibt.
                let place = app
                    .all_presets()
                    .find(|p| p.path == path)
                    .map(|preset| Place::Group {
                        section: preset.section(),
                        name: preset.group.clone(),
                    });
                if place.is_some() {
                    app.place = place;
                }
            }
        }
        Action::StartRename(preset) => app.start_rename_preset(&preset),
        Action::CommitRename => app.commit_rename(now),
        Action::CancelRename => app.rename = None,
        Action::SelectOnly(path) => {
            app.selection.clear();
            app.selection.insert(path.clone());
            app.anchor = Some(path);
        }
        Action::Reorder(payload, before) => {
            if let DragPayload::Items(items) = payload.as_ref() {
                app.drop_reorder(items, before.as_deref(), now);
            }
        }
        Action::DropOnEmptyGroup(payload) => {
            if let Some(Place::Group { section, name }) = app.place.clone() {
                app.drop_on_group(&payload, section, &name, now);
            }
        }
        Action::GroupFromSelection => app.group_from_selection(now),
        Action::Explorer(path) => {
            if let Err(message) = show_in_explorer(&path) {
                app.toast(message, ToastKind::Error, now);
            }
        }
        Action::Delete => app.request_delete_selection(),
    }
}

fn problems(ui: &mut Ui, app: &App) {
    ui.add_space(8.0);
    ui.label(RichText::new(texts::PROBLEMS_HINT).color(theme::MUTED));
    ui.add_space(6.0);
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for (path, reason) in &app.problems {
            ui.horizontal(|ui| {
                widgets::glyph(ui, icon::WARNING, theme::WARN);
                ui.vertical(|ui| {
                    ui.label(path.display().to_string());
                    ui.label(RichText::new(reason).small().color(theme::MUTED));
                });
            });
            ui.add_space(4.0);
        }
    });
}
