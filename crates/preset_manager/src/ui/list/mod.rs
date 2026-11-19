//! Mitte: die Einträge der gewählten Gruppe oder die Suchtreffer.
//!
//! Einträge lassen sich auswählen (Strg/Umschalt für mehrere), ziehen (auf
//! eine Gruppe links oder an eine andere Stelle der Liste), umbenennen (F2,
//! Doppelklick) und über das Kontextmenü bearbeiten. Deaktivierte Einträge
//! (F-48) sind blasser und tragen ein durchgestrichenes Auge; das Auge am
//! Zeilenende schaltet um.
//!
//! | Datei | Inhalt |
//! |---|---|
//! | `rows` | die Zeilen, Ziehen und Ablegen |
//! | `menu` | Kontextmenü |

mod menu;
mod rows;

use rows::rows;

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, Frame, Margin, RichText, ScrollArea, Sense, Ui};
use egui_phosphor::regular as icon;
use preset_core::preset::{Preset, Section};

use crate::app::{show_in_explorer, App, DragPayload, Filter, Place, ToastKind};
use crate::texts::{self, tr, tr_format};
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
        tr_format!("Suche „{}“", "Search “{}”", app.search.trim())
    } else {
        match &app.place {
            Some(Place::Group { name, .. }) if name.is_empty() => texts::no_group().to_owned(),
            Some(Place::Group { name, .. }) => name.clone(),
            Some(Place::Problems) => texts::problems_group().to_owned(),
            None => String::new(),
        }
    };
    let mut subtitle = if searching {
        texts::hits(listed.len())
    } else if app.place == Some(Place::Problems) {
        texts::files(app.problems.len())
    } else {
        texts::count_label(presets, profiles)
    };
    if app.selection.len() > 1 {
        subtitle.push_str(&tr_format!(
            " · {} ausgewählt",
            " · {} selected",
            app.selection.len()
        ));
    }

    let viewing_inactive =
        !searching && !app.place_active && matches!(app.place, Some(Place::Group { .. }));
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
                            texts::filter_profiles(),
                            Some(icon::APERTURE),
                            tr("Strg+3", "Ctrl+3"),
                        ),
                        (
                            Filter::Presets,
                            texts::filter_presets(),
                            Some(icon::SLIDERS_HORIZONTAL),
                            tr("Strg+2", "Ctrl+2"),
                        ),
                        (
                            Filter::All,
                            texts::filter_all(),
                            None,
                            tr("Strg+1", "Ctrl+1"),
                        ),
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
                        if viewing_inactive {
                            widgets::chip(
                                ui,
                                &format!("{}  {}", icon::EYE_SLASH, texts::section_inactive()),
                                theme::WARN_TEXT,
                                theme::WARN_BG,
                            )
                            .on_hover_text(texts::inactive_hint());
                        }
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
    SetActive(Vec<Preset>, bool),
    Export(Vec<Preset>),
}

fn body(ui: &mut Ui, app: &mut App, now: f64) {
    if app.library.is_none() {
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new(texts::loading()).color(theme::MUTED));
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
            texts::nothing_found(),
            texts::nothing_found_hint(),
        );
        return;
    }
    let Some(Place::Group { section, name }) = app.place.clone() else {
        widgets::empty_state(
            ui,
            icon::TRAY_ARROW_DOWN,
            texts::empty_library(),
            texts::empty_library_hint(),
        );
        return;
    };
    if !app.is_empty_group(section, &name) {
        widgets::empty_state(
            ui,
            icon::TRAY_ARROW_DOWN,
            texts::empty_library(),
            texts::empty_library_hint(),
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
            DragPayload::Items(items) => {
                app.move_problem_in(items, section, &name, Some(app.place_active))
            }
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
        texts::empty_group_title(),
        texts::empty_group_hint(),
    );
}

fn apply(app: &mut App, action: Action, now: f64) {
    match action {
        Action::Click(path, modifiers) => {
            app.click_item(&path, modifiers);
            if app.searching() {
                // Die Gruppe des Treffers links markieren, die Suche bleibt.
                let found = app.all_presets().find(|p| p.path == path).map(|preset| {
                    (
                        Place::Group {
                            section: preset.section(),
                            name: preset.group.clone(),
                        },
                        preset.active,
                    )
                });
                if let Some((place, active)) = found {
                    app.place = Some(place);
                    app.place_active = active;
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
                let active = app.place_active;
                app.drop_on_group(&payload, section, &name, active, now);
            }
        }
        Action::GroupFromSelection => app.group_from_selection(now),
        Action::Explorer(path) => {
            if let Err(message) = show_in_explorer(&path) {
                app.toast(message, ToastKind::Error, now);
            }
        }
        Action::Delete => app.request_delete_selection(),
        Action::SetActive(presets, active) => app.set_presets_active(presets, active, now),
        Action::Export(presets) => {
            let name = crate::actions::suggested_name(&presets);
            app.export_presets(presets, &name, now);
        }
    }
}

fn problems(ui: &mut Ui, app: &App) {
    ui.add_space(8.0);
    ui.label(RichText::new(texts::problems_hint()).color(theme::MUTED));
    ui.add_space(6.0);
    widgets::scroll(ui, ScrollArea::vertical().auto_shrink(false), |ui| {
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
