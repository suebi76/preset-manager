//! Die Bereiche wie in Camera Raw: Profil, Grundeinstellungen,
//! Gradationskurve, Farbmischer, Color Grading, Masken. Jeder zeigt nur, was
//! vom Standard abweicht, und gibt zurück, ob er etwas gezeigt hat.

use eframe::egui::{RichText, Ui};
use preset_core::preset::adjustments::{Panel, WhiteBalance};
use preset_core::preset::settings::LocalAdjustment;
use preset_core::preset::PresetSettings;

use crate::texts::{tr, tr_format};
use crate::ui::theme;

use super::wording::{
    local_label, mask_kind, number, panel_title, slider_label, white_balance_name,
};

use super::curve::curve_graph;
use super::grading::grading_wheels;
use super::mixer::{mixer_header, mixer_line, MixerPart};
use super::section;
use super::sliders::{note, slider_row, value_row, Track};

pub(super) fn profile_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let look = settings.look.as_ref().filter(|look| !look.name.is_empty());
    let profile = settings
        .text("CameraProfile")
        .filter(|name| !name.is_empty() && *name != "Embedded");
    let black_and_white = settings.is_black_and_white();
    if look.is_none() && profile.is_none() && !black_and_white && !settings.uses_color_table {
        return false;
    }
    section(ui, tr("Profil", "Profile"), |ui| {
        if let Some(look) = look {
            let amount = look
                .amount
                .filter(|amount| (amount - 1.0).abs() > 0.005)
                .map(|amount| format!(" · {} %", (amount * 100.0).round()))
                .unwrap_or_default();
            value_row(
                ui,
                tr("Profil", "Profile"),
                &format!("{}{amount}", look.name),
            );
        } else if let Some(profile) = profile {
            value_row(ui, tr("Kameraprofil", "Camera profile"), profile);
        }
        if black_and_white {
            value_row(
                ui,
                tr("Behandlung", "Treatment"),
                tr("Schwarzweiß", "Black & white"),
            );
        }
        if settings.uses_color_table {
            note(
                ui,
                tr(
                    "Enthält eine Farbtabelle (LUT). Ihr Look lässt sich nicht als Regler zeigen.",
                    "Contains a color table (LUT). Its look cannot be shown as sliders.",
                ),
            );
        }
    });
    true
}

pub(super) fn basic_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let sliders = settings.sliders(Panel::Basic);
    let white_balance = settings.white_balance();
    if sliders.is_empty() && white_balance == WhiteBalance::AsShot {
        return false;
    }
    section(ui, panel_title(Panel::Basic), |ui| {
        match &white_balance {
            WhiteBalance::AsShot => {}
            WhiteBalance::Named(name) => {
                value_row(
                    ui,
                    tr("Weißabgleich", "White balance"),
                    white_balance_name(name),
                );
            }
            WhiteBalance::Custom { temperature, tint } => {
                let mut parts = Vec::new();
                if let Some(kelvin) = temperature {
                    parts.push(format!("{} K", kelvin.round()));
                }
                if let Some(tint) = tint {
                    parts.push(format!(
                        "{} {}",
                        tr("Tönung", "Tint"),
                        number(*tint, 0, true)
                    ));
                }
                value_row(ui, tr("Weißabgleich", "White balance"), &parts.join(" · "));
                note(
                    ui,
                    tr(
                        "Feste Werte – wirken so nur bei Rohdaten.",
                        "Fixed values – work like this on raw files only.",
                    ),
                );
            }
        }
        for slider in &sliders {
            let track = match slider.spec.key {
                "IncrementalTemperature" => Track::Temperature,
                "IncrementalTint" => Track::Tint,
                _ => Track::Plain,
            };
            slider_row(ui, slider_label(slider.spec.key), slider, track);
        }
    });
    true
}

pub(super) fn curve_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let curves = settings.point_curves();
    let sliders = settings.sliders(Panel::Curve);
    if curves.is_empty() && sliders.is_empty() {
        return false;
    }
    section(ui, panel_title(Panel::Curve), |ui| {
        if !curves.is_empty() {
            curve_graph(ui, &curves);
            ui.add_space(6.0);
        }
        for slider in &sliders {
            slider_row(ui, slider_label(slider.spec.key), slider, Track::Plain);
        }
    });
    true
}

pub(super) fn mixer_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let rows = settings.mixer();
    let gray = settings.gray_mixer();
    if rows.is_empty() && gray.is_empty() {
        return false;
    }
    if rows.is_empty() {
        section(ui, tr("Schwarzweiß-Mischer", "B&W mixer"), |ui| {
            for (color, value) in &gray {
                mixer_line(ui, *color, &[(*value, MixerPart::Luminance)]);
            }
        });
        return true;
    }
    section(ui, tr("Farbmischer", "Color mixer"), |ui| {
        mixer_header(ui);
        for row in &rows {
            mixer_line(
                ui,
                row.color,
                &[
                    (row.hue, MixerPart::Hue),
                    (row.saturation, MixerPart::Saturation),
                    (row.luminance, MixerPart::Luminance),
                ],
            );
        }
    });
    true
}

pub(super) fn grading_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let Some(grading) = settings.color_grading() else {
        return false;
    };
    section(ui, "Color Grading", |ui| {
        grading_wheels(ui, &grading);
        if (grading.blending - 50.0).abs() > 0.5 {
            value_row(
                ui,
                tr("Überblendung", "Blending"),
                &number(grading.blending, 0, false),
            );
        }
        if grading.balance.abs() > 0.5 {
            value_row(
                ui,
                tr("Abgleich", "Balance"),
                &number(grading.balance, 0, true),
            );
        }
    });
    true
}

pub(super) fn local_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let adjustments: Vec<&LocalAdjustment> = settings
        .local_adjustments
        .iter()
        .filter(|adjustment| adjustment.has_effect())
        .collect();
    if adjustments.is_empty() {
        return false;
    }
    section(ui, tr("Masken", "Masking"), |ui| {
        for (index, adjustment) in adjustments.iter().enumerate() {
            let name = if adjustment.name.is_empty() {
                tr_format!("Maske {}", "Mask {}", index + 1)
            } else {
                adjustment.name.clone()
            };
            let kinds: Vec<&str> = adjustment
                .masks
                .iter()
                .map(|kind| mask_kind(*kind))
                .collect();
            ui.label(RichText::new(name).strong());
            if !kinds.is_empty() {
                ui.label(RichText::new(kinds.join(" + ")).small().color(theme::MUTED));
            }
            let changes: Vec<String> = adjustment
                .changes
                .iter()
                .map(|(key, value)| {
                    let direction = if *value > 0.0 { "+" } else { "−" };
                    format!("{} {direction}", local_label(key))
                })
                .collect();
            ui.label(
                RichText::new(changes.join(" · "))
                    .small()
                    .color(theme::TEXT),
            );
            ui.add_space(4.0);
        }
        note(
            ui,
            tr(
                "Masken passen sich in Camera Raw dem Foto an; ihre Lage lässt sich hier nicht zeigen.",
                "Masks adapt to the photo in Camera Raw; their position cannot be shown here.",
            ),
        );
    });
    true
}
