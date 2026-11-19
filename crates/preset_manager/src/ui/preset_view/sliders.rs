//! Regler wie in Camera Raw: Name, Spur mit Mittelpunkt, Wert – in einer
//! Zeile. Dazu Zeilen für Texte und Hinweise.

use eframe::egui::{
    self, Color32, CornerRadius, Mesh, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, Vec2,
};
use preset_core::preset::adjustments::{Scale, Slider};

use crate::ui::theme;

use super::wording::number;

/// Wie die Spur eines Reglers gefärbt ist.
#[derive(Clone, Copy)]
pub(super) enum Track {
    Plain,
    Temperature,
    Tint,
}

/// Eine Zeile: Bezeichnung links, Regler in der Mitte, Wert rechts.
pub(super) fn slider_row(ui: &mut Ui, label: &str, slider: &Slider, track: Track) {
    let signed = matches!(slider.spec.scale, Scale::Bipolar { .. });
    let value_text = number(slider.value, slider.spec.decimals, signed);
    let (min, max) = match slider.spec.scale {
        Scale::Bipolar { min, max } => (min, max),
        Scale::Unipolar { max } => (0.0, max),
    };
    let width = ui.available_width();
    let (row, response) = ui.allocate_exact_size(Vec2::new(width, 20.0), Sense::hover());
    let painter = ui.painter_at(row);
    let label_width = (width * 0.42).min(150.0);
    let value_width = 44.0;
    painter.galley(
        Pos2::new(row.left(), row.center().y - 8.0),
        one_line(ui, label, label_width - 6.0, theme::MUTED),
        theme::MUTED,
    );
    painter.text(
        Pos2::new(row.right(), row.center().y),
        egui::Align2::RIGHT_CENTER,
        &value_text,
        egui::FontId::monospace(12.0),
        theme::TEXT,
    );
    let rect = Rect::from_x_y_ranges(
        (row.left() + label_width)..=(row.right() - value_width),
        (row.center().y - 4.0)..=(row.center().y + 4.0),
    );
    response.on_hover_text(format!("{label}: {value_text}"));
    let bar = Rect::from_center_size(rect.center(), Vec2::new(rect.width(), 3.0));
    match track {
        Track::Plain => {
            painter.rect_filled(bar, CornerRadius::same(1), theme::LINE);
        }
        Track::Temperature => gradient_bar(
            &painter,
            bar,
            Color32::from_rgb(0x4a, 0x7b, 0xd8),
            Color32::from_rgb(0xe0, 0xc0, 0x3a),
        ),
        Track::Tint => gradient_bar(
            &painter,
            bar,
            Color32::from_rgb(0x4c, 0xb0, 0x5a),
            Color32::from_rgb(0xc8, 0x4c, 0xc0),
        ),
    }
    let position = |value: f32| {
        let t = ((value - min) / (max - min)).clamp(0.0, 1.0);
        bar.left() + t * bar.width()
    };
    let neutral = position(slider.spec.neutral.clamp(min, max));
    let value = position(slider.value);
    if matches!(track, Track::Plain) {
        let fill = Rect::from_x_y_ranges(neutral.min(value)..=neutral.max(value), bar.y_range());
        painter.rect_filled(fill, CornerRadius::same(1), theme::ACCENT);
    }
    if signed {
        painter.line_segment(
            [
                Pos2::new(neutral, rect.top()),
                Pos2::new(neutral, rect.bottom()),
            ],
            Stroke::new(1.0, theme::FAINT),
        );
    }
    painter.circle_filled(Pos2::new(value, bar.center().y), 3.5, theme::TEXT);
}

/// Text in einer Zeile, bei Platzmangel mit „…“ gekürzt.
fn one_line(ui: &Ui, text: &str, width: f32, color: Color32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(
        text.to_owned(),
        egui::TextStyle::Body.resolve(ui.style()),
        color,
    );
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(10.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

fn gradient_bar(painter: &egui::Painter, rect: Rect, left: Color32, right: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), left);
    mesh.colored_vertex(rect.right_top(), right);
    mesh.colored_vertex(rect.right_bottom(), right);
    mesh.colored_vertex(rect.left_bottom(), left);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

/// Bezeichnung links, Wert rechts.
pub(super) fn value_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(theme::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(RichText::new(value).color(theme::TEXT)).truncate());
        });
    });
}

pub(super) fn note(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::FAINT));
}
