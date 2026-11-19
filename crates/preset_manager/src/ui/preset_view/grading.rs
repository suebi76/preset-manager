//! Color Grading als Farbräder: Tiefen, Mitteltöne, Lichter, global.

use eframe::egui::{self, Color32, Mesh, RichText, Sense, Shape, Stroke, Ui, Vec2};
use preset_core::preset::adjustments::{ColorGrading, Wheel};

use crate::texts::{tr, tr_format};
use crate::ui::theme;

use super::wording::number;

pub(super) fn grading_wheels(ui: &mut Ui, grading: &ColorGrading) {
    let wheels = [
        (tr("Tiefen", "Shadows"), grading.shadows),
        (tr("Mitteltöne", "Midtones"), grading.midtones),
        (tr("Lichter", "Highlights"), grading.highlights),
        (tr("Global", "Global"), grading.global),
    ];
    let spacing = ui.spacing().item_spacing.x;
    let size = ((ui.available_width() - 3.0 * spacing) / 4.0).clamp(36.0, 80.0);
    ui.horizontal(|ui| {
        for (title, wheel) in wheels {
            ui.vertical(|ui| {
                ui.set_width(size);
                color_wheel(ui, size, &wheel);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(title).small().color(theme::MUTED));
                    if wheel.luminance.abs() >= 0.5 {
                        ui.label(
                            RichText::new(number(wheel.luminance, 0, true))
                                .small()
                                .color(theme::FAINT),
                        )
                        .on_hover_text(tr("Luminanz", "Luminance"));
                    }
                });
            });
        }
    });
    ui.add_space(4.0);
}

fn color_wheel(ui: &mut Ui, size: f32, wheel: &Wheel) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = size / 2.0 - 2.0;

    // Farbkreis: außen volle Farbe, innen grau.
    let mut mesh = Mesh::default();
    let segments = 48;
    mesh.colored_vertex(center, Color32::from_gray(0x55));
    for i in 0..=segments {
        let degrees = i as f32 * 360.0 / segments as f32;
        mesh.colored_vertex(
            center + wheel_offset(degrees, radius),
            hue_color(degrees, 0.65, 0.75),
        );
    }
    for i in 1..=segments as u32 {
        mesh.add_triangle(0, i, i + 1);
    }
    painter.add(Shape::mesh(mesh));
    painter.circle_stroke(center, radius, Stroke::new(1.0, theme::LINE));

    let active = wheel.saturation >= 0.5;
    let dot = center
        + wheel_offset(
            wheel.hue,
            radius * (wheel.saturation / 100.0).clamp(0.0, 1.0),
        );
    painter.circle_filled(
        dot,
        4.0,
        if active {
            hue_color(wheel.hue, 0.8, 1.0)
        } else {
            Color32::from_gray(0x99)
        },
    );
    painter.circle_stroke(dot, 4.0, Stroke::new(1.5, Color32::WHITE));
    if active {
        response.on_hover_text(tr_format!(
            "Farbton {}°, Sättigung {}",
            "Hue {}°, saturation {}",
            wheel.hue.round(),
            wheel.saturation.round()
        ));
    }
}

/// Camera Raw zeichnet 0° (Rot) rechts, gegen den Uhrzeigersinn steigend.
fn wheel_offset(degrees: f32, radius: f32) -> Vec2 {
    let radians = degrees.to_radians();
    Vec2::new(radians.cos() * radius, -radians.sin() * radius)
}

pub(super) fn hue_color(degrees: f32, saturation: f32, value: f32) -> Color32 {
    egui::ecolor::Hsva::new(degrees.rem_euclid(360.0) / 360.0, saturation, value, 1.0).into()
}
