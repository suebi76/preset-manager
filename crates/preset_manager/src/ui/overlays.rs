//! Alles, was über oder unter den drei Spalten liegt: Hinweisleiste,
//! Ablegefläche, „wird gelesen“-Anzeige, Einblendungen und die Fehlerseite.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Frame, Id, LayerId, Margin, Order, RichText,
    Stroke, Ui,
};
use egui_phosphor::regular as icon;

use crate::app::{App, DragPayload, FolderSource, Place, ToastKind, CUSTOM_DIR_VARIABLE};
use crate::texts;
use crate::ui::{theme, widgets};
use preset_core::preset::Section;

/// Hinweis „Photoshop läuft …“ am unteren Rand (F-41).
pub fn hint_bar(ui: &mut Ui, app: &mut App) {
    if !app.show_running_hint() {
        return;
    }
    let names: Vec<&str> = app
        .adobe_apps
        .iter()
        .map(|app| app.display_name())
        .collect();
    let text = texts::running_hint(&names);
    let mut close = false;
    egui::Panel::bottom("hint")
        .frame(
            Frame::new()
                .fill(theme::WARN_BG)
                .inner_margin(Margin::symmetric(12, 6)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon::WARNING).color(theme::WARN_TEXT));
                ui.label(RichText::new(text).color(theme::WARN_TEXT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new(icon::X).color(theme::WARN_TEXT))
                                .frame(false),
                        )
                        .on_hover_text("Hinweis ausblenden")
                        .clicked()
                    {
                        close = true;
                    }
                });
            });
        });
    if close {
        app.hint_dismissed = true;
    }
}

/// Rahmen und Beschriftung, solange Dateien über das Fenster gezogen werden.
pub fn drop_overlay(ctx: &egui::Context, app: &App) {
    let hovering = ctx.input(|input| !input.raw.hovered_files.is_empty());
    if !hovering {
        return;
    }
    let pointer = ctx.input(|input| input.pointer.latest_pos());
    let text = match pointer.and_then(|pos| app.group_under(pos)) {
        Some(Place::Group { name, .. }) => format!("In „{name}“ importieren"),
        _ => texts::DROP_HERE.to_owned(),
    };
    let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("drop-overlay")));
    let screen = ctx.content_rect();
    painter.rect(
        screen.shrink(2.0),
        CornerRadius::same(8),
        Color32::from_rgba_unmultiplied(38, 128, 235, 26),
        Stroke::new(2.0, theme::ACCENT),
        egui::StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(text, FontId::proportional(15.0), Color32::WHITE);
    let pill = egui::Rect::from_center_size(
        egui::pos2(screen.center().x, screen.bottom() - 60.0),
        galley.size() + egui::vec2(32.0, 16.0),
    );
    painter.rect_filled(pill, CornerRadius::same(255), theme::ACCENT);
    painter.galley(pill.min + egui::vec2(16.0, 8.0), galley, Color32::WHITE);
}

/// Während Einträge oder Gruppen gezogen werden: ein Etikett am Mauszeiger
/// („3 Presets“) und – über einem ungültigen Ziel – die Begründung.
pub fn drag_feedback(ctx: &egui::Context, app: &App) {
    let Some(payload) = egui::DragAndDrop::payload::<DragPayload>(ctx) else {
        return;
    };
    let Some(pointer) = ctx.pointer_latest_pos() else {
        return;
    };
    let (glyph, color, text) = match payload.as_ref() {
        DragPayload::Items(items) => {
            let presets = items
                .iter()
                .filter(|p| p.section() == Section::Presets)
                .count();
            let (glyph, color) = items
                .first()
                .map_or((icon::SLIDERS_HORIZONTAL, theme::PRESET), |p| {
                    widgets::type_icon(&p.preset_type)
                });
            (
                glyph,
                color,
                texts::count_label(presets, items.len() - presets),
            )
        }
        DragPayload::Group { section, name } => {
            let (glyph, color) = widgets::section_icon(*section);
            (glyph, color, format!("Gruppe „{name}“"))
        }
    };
    egui::Area::new(Id::new("drag-label"))
        .fixed_pos(pointer + egui::vec2(14.0, 10.0))
        .order(Order::Tooltip)
        .interactable(false)
        .show(ctx, |ui| {
            Frame::new()
                .fill(theme::RAISED)
                .stroke(Stroke::new(1.0, theme::ACCENT))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(Margin::symmetric(10, 3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        widgets::glyph(ui, glyph, color);
                        ui.label(RichText::new(text).strong());
                    });
                });
            if let Some(hint) = &app.drag_hint {
                ui.add_space(4.0);
                Frame::new()
                    .fill(Color32::from_rgb(0x11, 0x11, 0x11))
                    .stroke(Stroke::new(1.0, theme::LINE))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.set_max_width(300.0);
                        ui.label(RichText::new(format!("⦸ {hint}")).color(theme::WARN_TEXT));
                    });
            }
        });
    if app.drag_hint.is_some() {
        ctx.set_cursor_icon(egui::CursorIcon::NotAllowed);
    }
}

/// Kleine Anzeige, solange eine Quelle durchsucht oder eine Änderung
/// gespeichert wird.
pub fn busy(ctx: &egui::Context, app: &App) {
    let text = if app.scanning {
        texts::SCANNING
    } else if app.managing {
        texts::BUSY_MANAGING
    } else {
        return;
    };
    egui::Area::new(Id::new("busy"))
        .anchor(Align2::CENTER_TOP, egui::vec2(0.0, 56.0))
        .order(Order::Foreground)
        .show(ctx, |ui| {
            Frame::popup(ui.style())
                .inner_margin(Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(text);
                    });
                });
        });
}

/// Einblendungen unten rechts. Erfolge verschwinden nach 5 s, Fehler
/// bleiben länger stehen und lassen sich wegklicken.
pub fn toasts(ctx: &egui::Context, app: &mut App, now: f64) {
    app.toasts.retain(|toast| toast.until > now);
    if app.toasts.is_empty() {
        return;
    }
    let bottom_offset = if app.show_running_hint() {
        -48.0
    } else {
        -16.0
    };
    let mut dismissed = None;
    egui::Area::new(Id::new("toasts"))
        .anchor(Align2::RIGHT_BOTTOM, egui::vec2(-16.0, bottom_offset))
        .order(Order::Foreground)
        .show(ctx, |ui| {
            ui.set_max_width(400.0);
            for (index, toast) in app.toasts.iter().enumerate() {
                let (glyph, color) = match toast.kind {
                    ToastKind::Success => (icon::CHECK, theme::OK),
                    ToastKind::Info => (icon::INFO, theme::ACCENT),
                    ToastKind::Error => (icon::WARNING, theme::ERROR),
                };
                Frame::new()
                    .fill(Color32::from_rgb(0x2b, 0x2b, 0x2b))
                    .stroke(Stroke::new(1.0, theme::LINE))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            widgets::glyph(ui, glyph, color);
                            ui.add(egui::Label::new(toast.text.as_str()).wrap());
                            if toast.kind == ToastKind::Error
                                && ui.add(egui::Button::new(icon::X).frame(false)).clicked()
                            {
                                dismissed = Some(index);
                            }
                        });
                    });
                ui.add_space(6.0);
            }
        });
    if let Some(index) = dismissed {
        app.toasts.remove(index);
    }
    let next = app
        .toasts
        .iter()
        .map(|toast| toast.until - now)
        .fold(f64::INFINITY, f64::min);
    if next.is_finite() {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(next.max(0.05)));
    }
}

/// Fehlerseite, wenn Camera Raw nicht gefunden wurde.
pub fn fatal(ui: &mut Ui, app: &mut App, message: &str, now: f64) {
    let ctx = ui.ctx().clone();
    egui::CentralPanel::no_frame()
        .frame(Frame::new().fill(theme::BG).inner_margin(Margin::same(32)))
        .show(ui, |ui| {
            let hint = match app.folder_source {
                FolderSource::Automatic => {
                    "Die App braucht Photoshop oder Lightroom Classic mit Camera Raw auf diesem Rechner."
                }
                _ => "Der gewählte Ordner ist nicht erreichbar – vielleicht ein getrenntes Laufwerk.",
            };
            widgets::empty_state(
                ui,
                icon::WARNING,
                "Camera Raw wurde nicht gefunden",
                hint,
            );
            ui.add_space(12.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(message).color(theme::MUTED));
                ui.add_space(16.0);
                if app.folder_source == FolderSource::Environment {
                    ui.label(
                        RichText::new(format!(
                            "Der Ordner ist über die Umgebungsvariable {CUSTOM_DIR_VARIABLE} vorgegeben."
                        ))
                        .color(theme::MUTED),
                    );
                    return;
                }
                if widgets::primary_button(ui, texts::CHOOSE_CAMERA_RAW, true).clicked() {
                    if let Some(dir) = rfd::FileDialog::new()
                        .set_title(texts::CHOOSE_CAMERA_RAW)
                        .pick_folder()
                    {
                        app.choose_camera_raw_dir(&ctx, Some(dir), now);
                    }
                }
                if app.folder_source == FolderSource::Chosen {
                    ui.add_space(6.0);
                    if ui.button("Wieder automatisch suchen").clicked() {
                        app.choose_camera_raw_dir(&ctx, None, now);
                    }
                }
            });
        });
}
