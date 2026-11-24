//! Fenster „Was ist neu?“: die Notizen aus dem CHANGELOG.
//!
//! - GitHub-Fassung: vor dem Installieren, aus der Update-Leiste geöffnet –
//!   mit „Jetzt installieren“.
//! - Store-Fassung: einmal nach einem Update, beim ersten Start der neuen
//!   Version.
//!
//! Der CHANGELOG ist schlichtes Markdown: Absätze, Aufzählungen mit `- `
//! (auch eingerückt), `**fett**` und `` `Code` ``. Mehr braucht es nicht.

use eframe::egui::{self, text::LayoutJob, Frame, Margin, RichText, ScrollArea, TextFormat};

use crate::app::App;
use crate::release_notes::VersionNotes;
use crate::texts::{self, tr, tr_format};
use crate::ui::{theme, widgets};
use crate::updates::{Status, UpdateEvent};

/// Was das Fenster gerade zeigt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhatsNew {
    /// Die gefundene, noch nicht installierte Version (GitHub-Fassung).
    Upcoming,
    /// Was seit dem letzten Start neu ist (Store-Fassung).
    Installed(Vec<VersionNotes>),
}

pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(whats_new) = app.whats_new.clone() else {
        return;
    };
    let (title, notes, offer_install) = match &whats_new {
        WhatsNew::Upcoming => {
            let notes = app.updater.notes();
            let version = app.updater.found_version().unwrap_or_default().to_owned();
            let can_install = matches!(app.updater.status, Status::Available(_) | Status::Ready(_));
            (
                tr_format!(
                    "Was ist neu in Version {version}?",
                    "What's new in version {version}?"
                ),
                notes,
                can_install,
            )
        }
        WhatsNew::Installed(notes) => (
            tr_format!(
                "Neu in Version {}",
                "New in version {}",
                env!("CARGO_PKG_VERSION")
            ),
            notes.clone(),
            false,
        ),
    };
    let mut close = false;
    let mut install = false;
    let response = egui::Modal::new(egui::Id::new("whats-new"))
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .stroke(egui::Stroke::new(1.0, theme::LINE))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(Margin::same(16)),
        )
        .show(ctx, |ui| {
            ui.set_width(520.0);
            ui.label(RichText::new(title).size(16.0).strong());
            ui.add_space(8.0);
            ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                if notes.is_empty() {
                    ui.label(
                        RichText::new(tr(
                            "Zu dieser Version gibt es keine Notizen.",
                            "There are no notes for this version.",
                        ))
                        .color(theme::MUTED),
                    );
                }
                let several = notes.len() > 1;
                for entry in &notes {
                    if several {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(tr_format!("Version {}", "Version {}", entry.version))
                                .strong()
                                .color(theme::MUTED),
                        );
                    }
                    markdown(ui, &entry.text);
                    ui.add_space(6.0);
                }
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if offer_install {
                        if widgets::primary_button(
                            ui,
                            texts::update_install(),
                            !app.restart_pending,
                        )
                        .clicked()
                        {
                            install = true;
                        }
                        if ui.button(tr("Später", "Later")).clicked() {
                            close = true;
                        }
                    } else if widgets::primary_button(ui, "OK", true).clicked() {
                        close = true;
                    }
                });
            });
        });
    if install {
        app.whats_new = None;
        if let Some(UpdateEvent::RestartNow) = app.updater.install_now(ctx) {
            app.restart_pending = true;
        }
    } else if close || response.should_close() {
        app.whats_new = None;
    }
}

/// Absätze und Aufzählungen; Folgezeilen (eingerückt) gehören zum Punkt
/// davor.
pub fn markdown(ui: &mut egui::Ui, text: &str) {
    let mut blocks: Vec<(usize, String)> = Vec::new(); // (Einrückung der Aufzählung, Text)
    for line in text.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if trimmed.is_empty() {
            blocks.push((usize::MAX, String::new()));
        } else if let Some(item) = trimmed.strip_prefix("- ") {
            blocks.push((indent / 2, item.to_owned()));
        } else if let Some((_, last)) = blocks
            .last_mut()
            .filter(|(level, text)| *level != usize::MAX && !text.is_empty())
        {
            last.push(' ');
            last.push_str(trimmed);
        } else {
            blocks.push((usize::MAX - 1, trimmed.to_owned()));
        }
    }
    for (level, text) in blocks {
        if text.is_empty() {
            continue;
        }
        let bullet = level < usize::MAX - 1;
        // Der Text bekommt eine eigene Spalte: So bricht er neben dem Punkt
        // um, nicht darunter.
        ui.horizontal_top(|ui| {
            if bullet {
                ui.add_space(level as f32 * 16.0);
                ui.label("•");
            }
            ui.vertical(|ui| {
                let job = styled(ui, &text);
                ui.add(egui::Label::new(job).wrap());
            });
        });
    }
}

/// `**fett**` und `` `Code` `` als Textformat.
fn styled(ui: &egui::Ui, text: &str) -> LayoutJob {
    let style = ui.style();
    let font = egui::TextStyle::Body.resolve(style);
    let mono = egui::TextStyle::Monospace.resolve(style);
    let mut job = LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    let mut bold = false;
    for (index, part) in text.split("**").enumerate() {
        if index > 0 {
            bold = !bold;
        }
        for (code_index, piece) in part.split('`').enumerate() {
            if piece.is_empty() {
                continue;
            }
            let code = code_index % 2 == 1;
            // Die Schrift kennt kein Fett – hervorgehoben wird mit Weiß.
            let format = TextFormat {
                font_id: if code { mono.clone() } else { font.clone() },
                color: if bold {
                    egui::Color32::WHITE
                } else {
                    theme::TEXT
                },
                ..Default::default()
            };
            job.append(piece, 0.0, format);
        }
    }
    job
}
