//! Vorher/Nachher: ein Foto mit den Werten eines Presets nachrechnen (F-39,
//! Teil 2).
//!
//! Das ist eine **Annäherung**. Camera Raw rechnet auf Rohdaten mit eigenen,
//! nicht veröffentlichten Formeln. Hier wird ein fertiges JPEG mit einfachen,
//! nachvollziehbaren Kurven bearbeitet, damit man die Richtung eines Presets
//! sieht: heller oder dunkler, wärmer oder kühler, matter, kräftiger … Was gar
//! nicht nachgebildet wird, nennt [`missing`].
//!
//! Reihenfolge:
//!
//! 1. Farbtabelle des Profils (exakt, siehe [`crate::preset::color_table`])
//! 2. Weißabgleich und Belichtung (linear)
//! 3. Licht: Kontrast, Lichter, Tiefen, Weiß, Schwarz, parametrische Kurve,
//!    Klarheit, Struktur, Dunst – auf der Helligkeit, Farben bleiben
//! 4. Gradationskurven je Kanal
//! 5. Farbe: Sättigung, Dynamik, Farbmischer oder Schwarzweiß-Mischer,
//!    Color Grading
//! 6. Vignette und Körnung
//!
//! | Datei | Inhalt |
//! |---|---|
//! | `params` | Werte aufbereiten, Schritte 2–6 |
//! | `color_math` | Rechenbausteine |
//! | `chart` | Testtafel |

mod chart;
mod color_math;
mod params;
#[cfg(test)]
mod tests;

pub use chart::test_chart;

use crate::preset::adjustments::{CurveChannel, WhiteBalance};
use crate::preset::color_table::{linear_to_srgb, srgb_to_linear, ColorTable};
use crate::preset::PresetSettings;
use color_math::box_blur;
use color_math::curve;
use color_math::hash_noise;
use color_math::luma;
use color_math::scale_light;
use color_math::smoothstep;
use params::Params;

/// Ein Bild in sRGB, Werte 0–1, Zeile für Zeile.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// Breite in Pixeln.
    pub width: usize,
    /// Höhe in Pixeln.
    pub height: usize,
    /// `width × height` Farben.
    pub pixels: Vec<[f32; 3]>,
}

/// Das Profil, auf dem ein Preset aufbaut: seine Farbtabellen und Stärke.
#[derive(Debug, Clone, Copy)]
pub struct LookTable<'a> {
    /// Die Tabellen, in dieser Reihenfolge angewendet.
    pub tables: &'a [ColorTable],
    /// Stärke, 1 = 100 %.
    pub amount: f32,
}

/// Was die Vorschau nicht nachbildet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// Lokale Anpassungen (Masken).
    Masks,
    /// Kalibrierung.
    Calibration,
    /// Schärfen und Rauschreduzierung.
    Detail,
    /// Objektivkorrektur und Objektiv-Vignettierung.
    Optics,
    /// Ein Kameraprofil wie „Adobe Farbe“ – wirkt auf Rohdaten.
    CameraProfile,
    /// Das Profil hat eine Farbtabelle, die nicht gefunden oder gelesen wurde.
    ProfileTable,
    /// Fester Weißabgleich in Kelvin – nur ungefähr, weil er für Rohdaten gilt.
    FixedWhiteBalance,
}

/// Was in der Vorschau fehlt oder nur grob stimmt.
pub fn missing(settings: &PresetSettings, look: Option<LookTable<'_>>) -> Vec<Missing> {
    let mut missing = Vec::new();
    if settings.effective_local_adjustments() > 0 {
        missing.push(Missing::Masks);
    }
    let changed = |keys: &[&str]| {
        keys.iter()
            .any(|key| settings.number(key).is_some_and(|v| v.abs() > 0.5))
    };
    if changed(&[
        "ShadowTint",
        "RedHue",
        "RedSaturation",
        "GreenHue",
        "GreenSaturation",
        "BlueHue",
        "BlueSaturation",
    ]) {
        missing.push(Missing::Calibration);
    }
    let detail = settings
        .number("LuminanceSmoothing")
        .is_some_and(|v| v > 0.5)
        || settings
            .number("Sharpness")
            .is_some_and(|v| (v - 40.0).abs() > 0.5);
    if detail {
        missing.push(Missing::Detail);
    }
    if changed(&["VignetteAmount"]) || settings.flag("LensProfileEnable") == Some(true) {
        missing.push(Missing::Optics);
    }
    let profile = settings.text("CameraProfile").unwrap_or_default();
    if !profile.is_empty() && profile != "Embedded" && settings.look.is_none() {
        missing.push(Missing::CameraProfile);
    }
    if (settings.look.is_some() || settings.uses_color_table) && look.is_none() {
        missing.push(Missing::ProfileTable);
    }
    if matches!(
        settings.white_balance(),
        WhiteBalance::Custom { .. } | WhiteBalance::Named(_)
    ) {
        missing.push(Missing::FixedWhiteBalance);
    }
    missing
}

/// Rechnet das Bild mit den Werten des Presets nach.
pub fn develop(image: &Image, settings: &PresetSettings, look: Option<LookTable<'_>>) -> Image {
    let p = Params::new(settings);
    let luminance: Vec<f32> = image.pixels.iter().map(|&pixel| luma(pixel)).collect();
    // Für Klarheit und Struktur: weichgezeichnete Helligkeit.
    let short = image.width.min(image.height).max(1) as f32;
    let clarity_blur = (p.clarity.abs() > 0.001 || p.dehaze.abs() > 0.001).then(|| {
        box_blur(
            &luminance,
            image.width,
            image.height,
            (short * 0.03) as usize,
        )
    });
    let texture_blur = (p.texture.abs() > 0.001).then(|| {
        box_blur(
            &luminance,
            image.width,
            image.height,
            (short * 0.006).max(1.0) as usize,
        )
    });

    let center = (image.width as f32 / 2.0, image.height as f32 / 2.0);
    let corner = (center.0.powi(2) + center.1.powi(2)).sqrt().max(1.0);

    let pixels = image
        .pixels
        .iter()
        .enumerate()
        .map(|(index, &pixel)| {
            let mut rgb = pixel.map(srgb_to_linear);
            if let Some(look) = look {
                for table in look.tables {
                    rgb = table.apply(rgb, look.amount);
                }
            }
            rgb = [
                rgb[0] * p.gains[0],
                rgb[1] * p.gains[1],
                rgb[2] * p.gains[2],
            ];
            rgb = rgb.map(|value| value * p.exposure);
            let mut rgb = rgb.map(linear_to_srgb);

            // Licht – auf der Helligkeit, Farbverhältnisse bleiben.
            let before = luma(rgb);
            let mut light = p.tone(before);
            // Lokaler Kontrast; begrenzt, damit an harten Kanten keine
            // Lichthöfe entstehen (Camera Raw arbeitet kantenerhaltend).
            if let Some(blur) = &clarity_blur {
                let detail = (luminance[index] - blur[index]).clamp(-0.12, 0.12);
                light += p.clarity * 0.35 * detail;
                light += p.dehaze * 0.15 * detail;
            }
            if let Some(blur) = &texture_blur {
                let detail = (luminance[index] - blur[index]).clamp(-0.08, 0.08);
                light += p.texture * 0.6 * detail;
            }
            light = p.dehaze_tone(light);
            rgb = scale_light(rgb, before, light);

            // Gradationskurven
            for (channel, points) in &p.curves {
                rgb = match channel {
                    CurveChannel::Master => rgb.map(|value| curve(points, value)),
                    CurveChannel::Red => [curve(points, rgb[0]), rgb[1], rgb[2]],
                    CurveChannel::Green => [rgb[0], curve(points, rgb[1]), rgb[2]],
                    CurveChannel::Blue => [rgb[0], rgb[1], curve(points, rgb[2])],
                };
            }

            // Farbe
            rgb = if p.black_and_white {
                p.black_and_white(rgb)
            } else {
                p.color(rgb)
            };
            rgb = p.grade(rgb);

            // Effekte
            if p.vignette.abs() > 0.001 {
                let (x, y) = ((index % image.width) as f32, (index / image.width) as f32);
                let distance = ((x - center.0).powi(2) + (y - center.1).powi(2)).sqrt() / corner;
                let start = p.vignette_midpoint * 0.9;
                let weight = smoothstep(start, (start + 0.55).min(1.2), distance);
                rgb = if p.vignette < 0.0 {
                    rgb.map(|value| value * (1.0 + p.vignette * 0.85 * weight))
                } else {
                    rgb.map(|value| value + (1.0 - value) * p.vignette * 0.85 * weight)
                };
            }
            if p.grain > 0.001 {
                let noise = hash_noise(index) * p.grain * 0.14;
                let light = luma(rgb);
                let midtones = 4.0 * light * (1.0 - light);
                rgb = rgb.map(|value| value + noise * (0.35 + 0.65 * midtones));
            }
            rgb.map(|value| value.clamp(0.0, 1.0))
        })
        .collect();

    Image {
        width: image.width,
        height: image.height,
        pixels,
    }
}
