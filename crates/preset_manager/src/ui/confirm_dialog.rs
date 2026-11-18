//! Rückfragen vor dem Löschen und Zusammenführen.
//!
//! Die Bestätigung nennt Anzahl und Namen und sagt ausdrücklich, dass die
//! Dateien in den Papierkorb kommen (`Planung/04-Fehlerbehandlung.md`).

use eframe::egui::{self, Frame, Margin, RichText};
use egui_phosphor::regular as icon;

use crate::app::{App, Confirm};
use crate::texts;
use crate::ui::{theme, widgets};

/// So viele Namen werden höchstens aufgezählt.
const MAX_NAMES: usize = 8;

pub fn show(ctx: &egui::Context, app: &mut App, now: f64) {
    let Some(confirm) = &app.confirm else {
        return;
    };
    let (title, lines, note, button, danger) = describe(confirm);
    let mut accepted = false;
    let mut cancelled = false;

    let response = egui::Modal::new(egui::Id::new("confirm-dialog"))
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .stroke(egui::Stroke::new(1.0, theme::LINE))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(Margin::same(16)),
        )
        .show(ctx, |ui| {
            ui.set_width(460.0);
            ui.label(RichText::new(title).size(16.0).strong());
            ui.add_space(8.0);
            if !note.is_empty() {
                ui.label(RichText::new(note).color(theme::MUTED));
                ui.add_space(6.0);
            }
            for line in &lines {
                ui.label(format!("•  {line}"));
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let confirm_button = if danger {
                        ui.add(
                            egui::Button::new(
                                RichText::new(format!("{}  {button}", icon::TRASH))
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .fill(theme::ERROR),
                        )
                    } else {
                        widgets::primary_button(ui, button, true)
                    };
                    if confirm_button.clicked() {
                        accepted = true;
                    }
                    if ui.button(texts::CANCEL).clicked() {
                        cancelled = true;
                    }
                });
            });
        });

    if accepted {
        app.confirm_accepted(now);
    } else if cancelled || response.should_close() {
        app.confirm = None;
    }
}

/// Titel, aufgezählte Namen, Hinweis, Beschriftung der Schaltfläche, und ob
/// es eine Lösch-Rückfrage ist.
fn describe(confirm: &Confirm) -> (String, Vec<String>, String, String, bool) {
    match confirm {
        Confirm::DeletePresets(presets) => {
            let presets_count = presets
                .iter()
                .filter(|p| p.section() == preset_core::preset::Section::Presets)
                .count();
            let what = texts::count_label(presets_count, presets.len() - presets_count);
            let mut lines: Vec<String> = presets
                .iter()
                .take(MAX_NAMES)
                .map(|p| p.name.clone())
                .collect();
            if presets.len() > MAX_NAMES {
                lines.push(format!("… und {} weitere", presets.len() - MAX_NAMES));
            }
            (
                format!("{what} in den Papierkorb legen?"),
                lines,
                texts::TRASH_NOTE.to_owned(),
                texts::CONFIRM_TRASH.to_owned(),
                true,
            )
        }
        Confirm::DeleteGroup { name, count, .. } => (
            format!("Gruppe „{name}“ in den Papierkorb legen?"),
            vec![format!(
                "{} in dieser Gruppe",
                texts::plural(*count, "Eintrag", "Einträge")
            )],
            texts::TRASH_NOTE.to_owned(),
            texts::CONFIRM_TRASH.to_owned(),
            true,
        ),
        Confirm::Merge {
            source,
            target,
            presets,
            clashes,
            ..
        } => {
            let mut lines = vec![format!(
                "{} kommen von „{source}“ nach „{target}“",
                texts::plural(presets.len(), "Eintrag", "Einträge")
            )];
            if *clashes > 0 {
                lines.push(format!(
                    "{} mit gleichem Namen bleiben in „{source}“",
                    texts::plural(*clashes, "Eintrag", "Einträge")
                ));
            }
            (
                "Gruppen zusammenführen?".to_owned(),
                lines,
                String::new(),
                texts::CONFIRM_MERGE.to_owned(),
                false,
            )
        }
    }
}
