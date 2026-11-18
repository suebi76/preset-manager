//! Mitte: die Einträge der gewählten Gruppe oder die Suchtreffer.
//!
//! Einträge lassen sich auswählen (Strg/Umschalt für mehrere), ziehen (auf
//! eine Gruppe links oder an eine andere Stelle der Liste), umbenennen (F2,
//! Doppelklick) und über das Kontextmenü bearbeiten. Deaktivierte Einträge
//! (F-48) sind blasser und tragen ein durchgestrichenes Auge; das Auge am
//! Zeilenende schaltet um.

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, DragAndDrop, Frame, Margin, RichText, ScrollArea, Sense, Ui};
use egui_phosphor::regular as icon;
use preset_core::preset::{Preset, Section};

use crate::app::{
    is_adobe_origin, show_in_explorer, App, DragPayload, Filter, Place, RenameTarget, ToastKind,
};
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
                        Section::Presets => texts::where_presets(),
                        Section::Profiles => texts::where_profiles(),
                    }
                );
                let sense = if renaming {
                    Sense::click()
                } else {
                    Sense::click_and_drag()
                };

                let eye_targets = if selected {
                    app.selected_presets()
                } else {
                    vec![preset.clone()]
                };
                let mut rename_outcome = None;
                let response = widgets::list_row(
                    ui,
                    selected,
                    sense,
                    |ui| {
                        // Das Auge schaltet um – bei einer Mehrfachauswahl
                        // alle ausgewählten Einträge.
                        let (eye, eye_color, eye_tip) = if preset.active {
                            (icon::EYE, theme::FAINT, texts::deactivate())
                        } else {
                            (icon::EYE_SLASH, theme::WARN_TEXT, texts::activate())
                        };
                        let eye_button =
                            egui::Button::new(RichText::new(eye).color(eye_color)).frame(false);
                        if ui
                            .add(eye_button)
                            .on_hover_text(format!("{eye_tip} ({})", tr("Strg+D", "Ctrl+D")))
                            .clicked()
                        {
                            actions.push(Action::SetActive(eye_targets.clone(), !preset.active));
                        }
                        if is_adobe_origin(preset) {
                            ui.label(RichText::new(icon::DOWNLOAD_SIMPLE).color(theme::FAINT))
                                .on_hover_text(texts::origin_adobe());
                        }
                        if searching {
                            let group = if preset.group.is_empty() {
                                texts::no_group()
                            } else {
                                &preset.group
                            };
                            ui.label(RichText::new(group).color(theme::MUTED));
                        }
                    },
                    |ui| {
                        ui.label(RichText::new(icon::DOTS_SIX_VERTICAL).color(theme::FAINT))
                            .on_hover_text(texts::drag_hint_reorder());
                        if preset.active {
                            widgets::glyph(ui, glyph, color).on_hover_text(&tooltip);
                        } else {
                            widgets::glyph(ui, glyph, color.gamma_multiply(0.45))
                                .on_hover_text(format!("{tooltip} · {}", texts::inactive_hint()));
                        }
                        match &mut app.rename {
                            Some(rename) if renaming => {
                                rename_outcome = Some(widgets::rename_field(
                                    ui,
                                    &mut rename.text,
                                    &mut rename.focus,
                                ));
                            }
                            _ if preset.active => {
                                widgets::truncated(ui, preset.name.as_str());
                            }
                            _ => {
                                widgets::truncated(
                                    ui,
                                    RichText::new(preset.name.as_str()).color(theme::MUTED),
                                );
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
                                item.group == preset.group
                                    && item.section() == preset.section()
                                    && item.active == preset.active
                            }) {
                                widgets::paint_insert_line(ui, response.rect, below);
                            } else {
                                app.drag_hint = Some(
                                    tr(
                                        "Zum Verschieben auf eine Gruppe links ziehen",
                                        "To move, drag onto a group on the left",
                                    )
                                    .to_owned(),
                                );
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
                egui::Button::new(format!(
                    "{}  {}  (F2)",
                    icon::PENCIL_SIMPLE,
                    texts::rename()
                )),
            )
            .clicked()
        {
            actions.push(Action::StartRename(preset.clone()));
            ui.close();
        }
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
            ui.close();
        }
        let targets = if single {
            vec![preset.clone()]
        } else {
            app.selected_presets()
        };
        let deactivate = targets.iter().all(|target| target.active);
        let (glyph, text) = if deactivate {
            (icon::EYE_SLASH, texts::deactivate())
        } else {
            (icon::EYE, texts::activate())
        };
        if ui
            .button(format!("{glyph}  {text}  ({})", tr("Strg+D", "Ctrl+D")))
            .clicked()
        {
            actions.push(Action::SetActive(targets, !deactivate));
            ui.close();
        }
        if ui
            .add_enabled(
                single,
                egui::Button::new(format!(
                    "{}  {}",
                    icon::FOLDER_OPEN,
                    texts::show_in_explorer()
                )),
            )
            .clicked()
        {
            actions.push(Action::Explorer(preset.path.clone()));
            ui.close();
        }
        ui.separator();
        if ui
            .button(format!(
                "{}  {}  ({})",
                icon::TRASH,
                texts::to_trash(),
                tr("Entf", "Del")
            ))
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
    }
}

fn problems(ui: &mut Ui, app: &App) {
    ui.add_space(8.0);
    ui.label(RichText::new(texts::problems_hint()).color(theme::MUTED));
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
