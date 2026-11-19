//! Die Gradationskurve als Zeichnung, je Kanal in seiner Farbe.

use eframe::egui::{Color32, CornerRadius, Pos2, RichText, Sense, Shape, Stroke, Ui, Vec2};
use preset_core::preset::adjustments::{curve_value, CurveChannel};
use preset_core::preset::settings::CurvePoint;

use crate::texts::tr;
use crate::ui::theme;

pub(super) fn curve_graph(ui: &mut Ui, curves: &[(CurveChannel, &[CurvePoint])]) {
    let size = ui.available_width().min(260.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(4), theme::BG);
    let inner = rect.shrink(6.0);
    let grid = Stroke::new(1.0, theme::LINE);
    for i in 1..4 {
        let t = i as f32 / 4.0;
        let x = inner.left() + t * inner.width();
        let y = inner.top() + t * inner.height();
        painter.line_segment(
            [Pos2::new(x, inner.top()), Pos2::new(x, inner.bottom())],
            grid,
        );
        painter.line_segment(
            [Pos2::new(inner.left(), y), Pos2::new(inner.right(), y)],
            grid,
        );
    }
    painter.line_segment(
        [inner.left_bottom(), inner.right_top()],
        Stroke::new(1.0, theme::FAINT),
    );
    let to_screen = |x: f32, y: f32| {
        Pos2::new(
            inner.left() + x / 255.0 * inner.width(),
            inner.bottom() - y / 255.0 * inner.height(),
        )
    };
    // Erst die Farbkanäle, dann die Helligkeit obenauf.
    let mut ordered: Vec<_> = curves.to_vec();
    ordered.sort_by_key(|(channel, _)| *channel == CurveChannel::Master);
    for (channel, points) in ordered {
        let color = curve_color(channel);
        let line: Vec<Pos2> = (0..=64)
            .map(|i| {
                let x = i as f32 * 255.0 / 64.0;
                to_screen(x, curve_value(points, x))
            })
            .collect();
        painter.add(Shape::line(line, Stroke::new(1.6, color)));
        for (x, y) in points {
            painter.circle_filled(to_screen(*x, *y), 2.5, color);
        }
    }
    if curves
        .iter()
        .any(|(channel, _)| *channel != CurveChannel::Master)
    {
        ui.horizontal_wrapped(|ui| {
            for (channel, _) in curves {
                ui.label(
                    RichText::new(format!("● {}", channel_name(*channel)))
                        .small()
                        .color(curve_color(*channel)),
                );
            }
        });
    }
}

fn curve_color(channel: CurveChannel) -> Color32 {
    match channel {
        CurveChannel::Master => theme::TEXT,
        CurveChannel::Red => Color32::from_rgb(0xe0, 0x55, 0x55),
        CurveChannel::Green => Color32::from_rgb(0x55, 0xc0, 0x6a),
        CurveChannel::Blue => Color32::from_rgb(0x5a, 0x8c, 0xe8),
    }
}

fn channel_name(channel: CurveChannel) -> &'static str {
    match channel {
        CurveChannel::Master => tr("Helligkeit", "Luminance"),
        CurveChannel::Red => tr("Rot", "Red"),
        CurveChannel::Green => tr("Grün", "Green"),
        CurveChannel::Blue => tr("Blau", "Blue"),
    }
}
