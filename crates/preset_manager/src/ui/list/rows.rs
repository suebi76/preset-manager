//! Die Zeilen der Liste: Symbol, Name, Auge zum An- und Abschalten,
//! Umbenennen an Ort und Stelle, Ziehen und Ablegen zwischen den Zeilen.

use eframe::egui::{self, DragAndDrop, RichText, ScrollArea, Sense, Ui};
use egui_phosphor::regular as icon;
use preset_core::preset::{Preset, Section};

use crate::app::{is_adobe_origin, App, DragPayload, RenameTarget};
use crate::texts::{self, tr};
use crate::ui::{theme, widgets};

use super::menu::context_menu;
use super::Action;

pub(super) fn rows(
    ui: &mut Ui,
    app: &mut App,
    listed: &[Preset],
    searching: bool,
    actions: &mut Vec<Action>,
) {
    let dragging = DragAndDrop::has_payload_of_type::<DragPayload>(ui.ctx());
    let modifiers = click_modifiers(ui);
    let reorder_allowed = !searching;

    widgets::scroll_rows(
        ui,
        ScrollArea::vertical().auto_shrink(false),
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
