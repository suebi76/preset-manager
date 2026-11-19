//! Der Farbmischer als Tabelle farbiger Balken: Farbton, Sättigung,
//! Luminanz je Farbe.

use eframe::egui::{self, CornerRadius, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use preset_core::preset::adjustments::HslColor;

use crate::texts::tr;
use crate::ui::theme;

use super::grading::hue_color;
use super::wording::{color_name, number};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum MixerPart {
    Hue,
    Saturation,
    Luminance,
}

const MIXER_NAME_WIDTH: f32 = 74.0;

pub(super) fn mixer_header(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.add_space(MIXER_NAME_WIDTH);
        let width = mixer_cell_width(ui);
        for title in [
            tr("Farbton", "Hue"),
            tr("Sättigung", "Saturation"),
            tr("Luminanz", "Luminance"),
        ] {
            ui.add_sized(
                [width, 14.0],
                egui::Label::new(RichText::new(title).small().color(theme::FAINT)),
            );
        }
    });
}

fn mixer_cell_width(ui: &Ui) -> f32 {
    ((ui.available_width() - MIXER_NAME_WIDTH) / 3.0 - ui.spacing().item_spacing.x).max(24.0)
}

pub(super) fn mixer_line(ui: &mut Ui, color: HslColor, values: &[(f32, MixerPart)]) {
    ui.horizontal(|ui| {
        let (swatch, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter().circle_filled(
            swatch.center(),
            5.0,
            hue_color(color.hue_degrees(), 0.75, 0.9),
        );
        let name_width = MIXER_NAME_WIDTH - 10.0 - ui.spacing().item_spacing.x;
        let (name_rect, _) = ui.allocate_exact_size(Vec2::new(name_width, 16.0), Sense::hover());
        ui.painter_at(name_rect).text(
            name_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            color_name(color),
            egui::TextStyle::Body.resolve(ui.style()),
            theme::TEXT,
        );
        let columns = values.len().max(1) as f32;
        let width = (ui.available_width() / columns - ui.spacing().item_spacing.x).max(24.0);
        for (value, part) in values {
            mixer_cell(ui, color, *value, *part, width);
        }
    });
}

fn mixer_cell(ui: &mut Ui, color: HslColor, value: f32, part: MixerPart, width: f32) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 16.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let bar = Rect::from_center_size(rect.center(), Vec2::new(rect.width() - 4.0, 4.0));
    painter.rect_filled(bar, CornerRadius::same(1), theme::LINE);
    let center = bar.center().x;
    let end = center + (value / 100.0).clamp(-1.0, 1.0) * bar.width() / 2.0;
    let fill_color = match part {
        MixerPart::Hue => {
            let shifted = color.hue_degrees() + value.signum() * 30.0;
            hue_color(shifted, 0.75, 0.9)
        }
        MixerPart::Saturation => hue_color(color.hue_degrees(), 0.8, 0.9),
        MixerPart::Luminance => {
            if value >= 0.0 {
                hue_color(color.hue_degrees(), 0.35, 1.0)
            } else {
                hue_color(color.hue_degrees(), 0.8, 0.45)
            }
        }
    };
    if value.abs() >= 0.5 {
        let fill = Rect::from_x_y_ranges(center.min(end)..=center.max(end), bar.y_range());
        painter.rect_filled(fill, CornerRadius::same(1), fill_color);
    }
    painter.line_segment(
        [
            Pos2::new(center, rect.top() + 3.0),
            Pos2::new(center, rect.bottom() - 3.0),
        ],
        Stroke::new(1.0, theme::FAINT),
    );
    response.on_hover_text(number(value, 0, true));
}
