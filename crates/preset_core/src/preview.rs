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

use crate::preset::adjustments::{CurveChannel, HslColor, WhiteBalance};
use crate::preset::color_table::{linear_to_srgb, srgb_to_linear, ColorTable};
use crate::preset::PresetSettings;

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

/// Die Werte des Presets, auf −1 … +1 bzw. passende Einheiten gebracht.
struct Params {
    gains: [f32; 3],
    exposure: f32,
    contrast: f32,
    highlights: f32,
    shadows: f32,
    whites: f32,
    blacks: f32,
    parametric: [(f32, f32, f32); 4],
    clarity: f32,
    texture: f32,
    dehaze: f32,
    curves: Vec<(CurveChannel, Vec<(f32, f32)>)>,
    saturation: f32,
    vibrance: f32,
    mixer: Vec<(f32, f32, f32, f32)>,
    gray_mixer: Vec<(f32, f32)>,
    black_and_white: bool,
    grading: Vec<(Zone, [f32; 3], f32, f32)>,
    vignette: f32,
    vignette_midpoint: f32,
    grain: f32,
}

#[derive(Clone, Copy)]
enum Zone {
    Shadows,
    Midtones,
    Highlights,
    Global,
}

impl Params {
    fn new(settings: &PresetSettings) -> Self {
        let value = |key: &str| settings.number(key).unwrap_or(0.0);
        let unit = |key: &str| value(key) / 100.0;

        // Weißabgleich: Verschiebung wie bei JPEGs (−100 … +100), ein fester
        // Wert in Kelvin wird gegenüber Tageslicht umgerechnet.
        let (mut temperature, mut tint) = (unit("IncrementalTemperature"), unit("IncrementalTint"));
        if let WhiteBalance::Custom {
            temperature: kelvin,
            tint: fixed_tint,
        } = settings.white_balance()
        {
            if let Some(kelvin) = kelvin.filter(|kelvin| *kelvin > 1000.0) {
                temperature += (1.0e6 / 5500.0 - 1.0e6 / kelvin) / 100.0;
            }
            if let Some(fixed_tint) = fixed_tint {
                tint += (fixed_tint - 10.0) / 100.0;
            }
        }
        let gains = [
            2f32.powf(temperature * 0.45),
            2f32.powf(-tint * 0.35),
            2f32.powf(-temperature * 0.55),
        ];

        let split = |key: &str, default: f32| settings.number(key).unwrap_or(default) / 100.0;
        let (s1, s2, s3) = (
            split("ParametricShadowSplit", 25.0),
            split("ParametricMidtoneSplit", 50.0),
            split("ParametricHighlightSplit", 75.0),
        );
        let parametric = [
            (0.0, s1, unit("ParametricShadows")),
            (s1, s2, unit("ParametricDarks")),
            (s2, s3, unit("ParametricLights")),
            (s3, 1.0, unit("ParametricHighlights")),
        ];

        let curves = settings
            .point_curves()
            .into_iter()
            .map(|(channel, points)| {
                let mut points: Vec<(f32, f32)> =
                    points.iter().map(|(x, y)| (x / 255.0, y / 255.0)).collect();
                points.sort_by(|a, b| a.0.total_cmp(&b.0));
                (channel, points)
            })
            .collect();

        let mixer = settings
            .mixer()
            .into_iter()
            .map(|row| {
                (
                    row.color.hue_degrees(),
                    row.hue / 100.0,
                    row.saturation / 100.0,
                    row.luminance / 100.0,
                )
            })
            .collect();
        let gray_mixer = settings
            .gray_mixer()
            .into_iter()
            .map(|(color, amount)| (color.hue_degrees(), amount / 100.0))
            .collect();

        let mut grading = Vec::new();
        if let Some(cg) = settings.color_grading() {
            for (zone, wheel) in [
                (Zone::Shadows, cg.shadows),
                (Zone::Midtones, cg.midtones),
                (Zone::Highlights, cg.highlights),
                (Zone::Global, cg.global),
            ] {
                grading.push((
                    zone,
                    hue_rgb(wheel.hue),
                    wheel.saturation / 100.0,
                    wheel.luminance / 100.0,
                ));
            }
        }

        Self {
            gains,
            exposure: 2f32.powf(value("Exposure2012")),
            contrast: unit("Contrast2012"),
            highlights: unit("Highlights2012"),
            shadows: unit("Shadows2012"),
            whites: unit("Whites2012"),
            blacks: unit("Blacks2012"),
            parametric,
            clarity: unit("Clarity2012"),
            texture: unit("Texture"),
            dehaze: unit("Dehaze"),
            curves,
            saturation: unit("Saturation"),
            vibrance: unit("Vibrance"),
            mixer,
            gray_mixer,
            black_and_white: settings.is_black_and_white(),
            grading,
            vignette: unit("PostCropVignetteAmount"),
            vignette_midpoint: settings.number("PostCropVignetteMidpoint").unwrap_or(50.0) / 100.0,
            grain: unit("GrainAmount"),
        }
    }

    /// Die Tonwertkorrekturen auf der Helligkeit (0–1, sRGB-kodiert).
    fn tone(&self, l: f32) -> f32 {
        let mut l = l;
        l += self.contrast * 0.55 * (l - 0.5) * (1.0 - (2.0 * l - 1.0).powi(2));
        l += self.highlights * 0.22 * smoothstep(0.45, 1.0, l) * (0.4 + 0.6 * l);
        l += self.shadows * 0.22 * (1.0 - smoothstep(0.0, 0.55, l)) * (0.35 + l);
        l += self.whites * 0.13 * smoothstep(0.55, 1.0, l);
        l += self.blacks * 0.13 * (1.0 - smoothstep(0.0, 0.45, l));
        for (from, to, amount) in self.parametric {
            if amount.abs() > 0.001 && to > from {
                let center = (from + to) / 2.0;
                let half = (to - from) / 2.0 + 0.08;
                let t = ((l - center) / half).clamp(-1.0, 1.0);
                l += amount * 0.14 * (1.0 - t * t);
            }
        }
        l
    }

    /// Dunst entfernen: Schwarz anheben bzw. absenken.
    fn dehaze_tone(&self, l: f32) -> f32 {
        if self.dehaze > 0.0 {
            let floor = self.dehaze * 0.08;
            (l - floor) / (1.0 - floor)
        } else {
            l + (-self.dehaze) * 0.22 * (0.72 - l)
        }
    }

    fn color(&self, rgb: [f32; 3]) -> [f32; 3] {
        let (mut hue, mut sat, mut value) = rgb_to_hsv(rgb);
        let original_sat = sat;
        let original_hue = hue;
        let mut hue_shift = 0.0;
        let mut sat_factor = 1.0;
        let mut light = 0.0;
        for &(center, shift, saturation, luminance) in &self.mixer {
            let weight = hue_weight(original_hue, center);
            hue_shift += weight * shift * 30.0;
            sat_factor += weight * saturation;
            light += weight * luminance * 0.3 * original_sat;
        }
        hue = (hue + hue_shift).rem_euclid(360.0);
        sat *= sat_factor.max(0.0);
        value = (value + light * value.max(0.2)).max(0.0);
        sat *= 1.0 + self.saturation + self.dehaze.max(0.0) * 0.2 + self.dehaze.min(0.0) * 0.3;
        if self.vibrance >= 0.0 {
            sat += self.vibrance * (1.0 - sat).max(0.0) * sat.min(1.0) * 1.2;
        } else {
            sat *= 1.0 + self.vibrance * 0.7;
        }
        hsv_to_rgb(hue, sat.clamp(0.0, 1.0), value)
    }

    fn black_and_white(&self, rgb: [f32; 3]) -> [f32; 3] {
        let (hue, sat, _) = rgb_to_hsv(rgb);
        let mut gray = luma(rgb);
        for &(center, amount) in &self.gray_mixer {
            gray += hue_weight(hue, center) * amount * 0.35 * sat;
        }
        [gray; 3]
    }

    fn grade(&self, rgb: [f32; 3]) -> [f32; 3] {
        if self.grading.is_empty() {
            return rgb;
        }
        let l = luma(rgb);
        let mut result = rgb;
        for &(zone, tint, saturation, luminance) in &self.grading {
            let weight = match zone {
                Zone::Shadows => 1.0 - smoothstep(0.0, 0.55, l),
                Zone::Midtones => 1.0 - (2.0 * l - 1.0).abs(),
                Zone::Highlights => smoothstep(0.45, 1.0, l),
                Zone::Global => 1.0,
            };
            let mean = (tint[0] + tint[1] + tint[2]) / 3.0;
            for channel in 0..3 {
                result[channel] += (tint[channel] - mean) * saturation * 0.32 * weight;
                result[channel] += luminance * 0.18 * weight;
            }
        }
        result
    }
}

/// Helligkeit nach Rec. 709 auf sRGB-kodierten Werten (für Bildbearbeitung
/// genau genug).
fn luma(rgb: [f32; 3]) -> f32 {
    0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
}

/// Setzt die Helligkeit auf `to`, ohne die Farbverhältnisse zu ändern.
fn scale_light(rgb: [f32; 3], from: f32, to: f32) -> [f32; 3] {
    if from > 0.01 {
        rgb.map(|value| value * to / from)
    } else {
        rgb.map(|value| value + (to - from))
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Punktkurve (0–1) mit linearer Interpolation – Camera Raw glättet stärker,
/// für die Vorschau genügt das.
fn curve(points: &[(f32, f32)], x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let scaled: Vec<(f32, f32)> = points.iter().map(|(a, b)| (a * 255.0, b * 255.0)).collect();
    crate::preset::adjustments::curve_value(&scaled, x * 255.0) / 255.0
}

/// Wie stark ein Farbton zu einer Farbe des Mischers gehört (0–1).
fn hue_weight(hue: f32, center: f32) -> f32 {
    let distance = ((hue - center + 540.0).rem_euclid(360.0) - 180.0).abs();
    let width = if (20.0..=70.0).contains(&center) {
        28.0
    } else {
        40.0
    };
    (1.0 - distance / width).max(0.0)
}

fn hue_rgb(hue: f32) -> [f32; 3] {
    hsv_to_rgb(hue, 1.0, 1.0)
}

fn rgb_to_hsv([r, g, b]: [f32; 3]) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta < 1e-6 {
        0.0
    } else if (max - r).abs() < f32::EPSILON {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() < f32::EPSILON {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let sat = if max <= 1e-6 { 0.0 } else { delta / max };
    (hue, sat, max)
}

fn hsv_to_rgb(hue: f32, sat: f32, value: f32) -> [f32; 3] {
    let c = value * sat;
    let h = hue.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = value - c;
    [r + m, g + m, b + m]
}

/// Gleichmäßiges Rauschen −1 … +1, für jedes Pixel immer dasselbe.
fn hash_noise(index: usize) -> f32 {
    let mut x = index as u32 ^ 0x9E37_79B9;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x as f32 / u32::MAX as f32) * 2.0 - 1.0
}

/// Kastenfilter in zwei Durchgängen (waagerecht, senkrecht).
fn box_blur(values: &[f32], width: usize, height: usize, radius: usize) -> Vec<f32> {
    if radius == 0 || width == 0 || height == 0 {
        return values.to_vec();
    }
    let pass = |input: &[f32], horizontal: bool| -> Vec<f32> {
        let mut output = vec![0.0; input.len()];
        let (lines, length) = if horizontal {
            (height, width)
        } else {
            (width, height)
        };
        for line in 0..lines {
            let at = |i: usize| {
                if horizontal {
                    line * width + i
                } else {
                    i * width + line
                }
            };
            let mut sum = 0.0;
            let mut count = 0.0;
            for i in 0..=radius.min(length - 1) {
                sum += input[at(i)];
                count += 1.0;
            }
            for i in 0..length {
                output[at(i)] = sum / count;
                if i + radius + 1 < length {
                    sum += input[at(i + radius + 1)];
                    count += 1.0;
                }
                if i >= radius {
                    sum -= input[at(i - radius)];
                    count -= 1.0;
                }
            }
        }
        output
    };
    pass(&pass(values, true), false)
}

/// Eine Testtafel, wenn kein eigenes Foto gewählt ist: Grauverlauf,
/// Farbverlauf und typische Motivfarben (Haut, Himmel, Laub, Sand).
pub fn test_chart(width: usize, height: usize) -> Image {
    let mut pixels = Vec::with_capacity(width * height);
    let swatches: [[f32; 3]; 6] = [
        [0.85, 0.64, 0.52], // helle Haut
        [0.55, 0.36, 0.26], // dunkle Haut
        [0.38, 0.58, 0.85], // Himmel
        [0.28, 0.45, 0.18], // Laub
        [0.88, 0.80, 0.62], // Sand
        [0.70, 0.12, 0.15], // Rot
    ];
    for y in 0..height {
        let v = y as f32 / height.max(1) as f32;
        for x in 0..width {
            let u = x as f32 / width.max(1).saturating_sub(1).max(1) as f32;
            let pixel = if v < 0.3 {
                [u; 3]
            } else if v < 0.6 {
                let light = 1.0 - (v - 0.3) / 0.3 * 0.6;
                hsv_to_rgb(u * 360.0, 0.75, light)
            } else {
                let index = ((u * swatches.len() as f32) as usize).min(swatches.len() - 1);
                let shade = 0.55 + 0.45 * (1.0 - (v - 0.6) / 0.4);
                swatches[index].map(|c| c * shade)
            };
            pixels.push(pixel);
        }
    }
    Image {
        width,
        height,
        pixels,
    }
}

impl HslColor {
    /// Nur für Tests: alle Farben in Grad.
    #[cfg(test)]
    fn all_degrees() -> Vec<f32> {
        Self::ALL.iter().map(|color| color.hue_degrees()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::color_table::tests::encode_table;
    use crate::preset::settings::parse_settings_for_tests;

    fn settings(attributes: &str) -> PresetSettings {
        parse_settings_for_tests(&format!(
            r#"<x:xmpmeta><rdf:RDF><rdf:Description crs:PresetType="Normal" {attributes}/></rdf:RDF></x:xmpmeta>"#
        ))
    }

    fn mean(image: &Image) -> [f32; 3] {
        let n = image.pixels.len() as f32;
        image.pixels.iter().fold([0.0; 3], |sum, p| {
            [sum[0] + p[0] / n, sum[1] + p[1] / n, sum[2] + p[2] / n]
        })
    }

    #[test]
    fn ohne_einstellungen_bleibt_das_bild() {
        let chart = test_chart(60, 40);
        let result = develop(&chart, &settings(""), None);

        for (a, b) in chart.pixels.iter().zip(&result.pixels) {
            for channel in 0..3 {
                assert!((a[channel] - b[channel]).abs() < 0.002, "{a:?} → {b:?}");
            }
        }
    }

    #[test]
    fn belichtung_und_temperatur_wirken_in_die_richtige_richtung() {
        let chart = test_chart(60, 40);
        let base = mean(&chart);

        let brighter = mean(&develop(
            &chart,
            &settings(r#"crs:Exposure2012="+1.0""#),
            None,
        ));
        let warmer = mean(&develop(
            &chart,
            &settings(r#"crs:IncrementalTemperature="+40""#),
            None,
        ));

        assert!(brighter.iter().zip(&base).all(|(a, b)| a > b));
        assert!(warmer[0] - warmer[2] > base[0] - base[2] + 0.05);
    }

    #[test]
    fn schwarzweiss_hat_keine_farbe() {
        let chart = test_chart(40, 30);
        let result = develop(&chart, &settings(r#"crs:ConvertToGrayscale="True""#), None);

        assert!(result
            .pixels
            .iter()
            .all(|p| (p[0] - p[1]).abs() < 1e-5 && (p[1] - p[2]).abs() < 1e-5));
    }

    #[test]
    fn farbtabelle_des_profils_wird_angewendet() {
        let table = [ColorTable::decode(&encode_table(9, |[r, g, b]| [b, g, r])).unwrap()];
        let red = Image {
            width: 1,
            height: 1,
            pixels: vec![[1.0, 0.0, 0.0]],
        };

        let result = develop(
            &red,
            &settings(""),
            Some(LookTable {
                tables: &table,
                amount: 1.0,
            }),
        );

        assert!(
            result.pixels[0][2] > 0.9 && result.pixels[0][0] < 0.1,
            "{:?}",
            result.pixels
        );
    }

    #[test]
    fn nennt_was_fehlt() {
        let s = settings(r#"crs:WhiteBalance="Custom" crs:Temperature="6500" crs:RedHue="+20""#);

        let missing = missing(&s, None);

        assert!(missing.contains(&Missing::FixedWhiteBalance));
        assert!(missing.contains(&Missing::Calibration));
        assert!(!missing.contains(&Missing::Masks));
    }

    #[test]
    fn farbgewichte_decken_den_farbkreis_ab() {
        for degrees in (0..360).step_by(5) {
            let total: f32 = HslColor::all_degrees()
                .into_iter()
                .map(|center| hue_weight(degrees as f32, center))
                .sum();
            assert!(total > 0.0, "{degrees}° gehört zu keiner Farbe");
        }
    }
}
