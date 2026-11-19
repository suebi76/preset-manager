//! Die Werte des Presets, für die Rechnung aufbereitet, und die Schritte
//! 2 bis 6 je Bildpunkt (siehe Modulbeschreibung).

use crate::preset::adjustments::{CurveChannel, WhiteBalance};
use crate::preset::PresetSettings;

use super::color_math::{hsv_to_rgb, hue_rgb, hue_weight, luma, rgb_to_hsv, smoothstep};

/// Die Werte des Presets, auf −1 … +1 bzw. passende Einheiten gebracht.
pub(super) struct Params {
    pub(super) gains: [f32; 3],
    pub(super) exposure: f32,
    contrast: f32,
    highlights: f32,
    shadows: f32,
    whites: f32,
    blacks: f32,
    parametric: [(f32, f32, f32); 4],
    pub(super) clarity: f32,
    pub(super) texture: f32,
    pub(super) dehaze: f32,
    pub(super) curves: Vec<(CurveChannel, Vec<(f32, f32)>)>,
    saturation: f32,
    vibrance: f32,
    mixer: Vec<(f32, f32, f32, f32)>,
    gray_mixer: Vec<(f32, f32)>,
    pub(super) black_and_white: bool,
    grading: Vec<(Zone, [f32; 3], f32, f32)>,
    pub(super) vignette: f32,
    pub(super) vignette_midpoint: f32,
    pub(super) grain: f32,
}

#[derive(Clone, Copy)]
enum Zone {
    Shadows,
    Midtones,
    Highlights,
    Global,
}

impl Params {
    pub(super) fn new(settings: &PresetSettings) -> Self {
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
    pub(super) fn tone(&self, l: f32) -> f32 {
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
    pub(super) fn dehaze_tone(&self, l: f32) -> f32 {
        if self.dehaze > 0.0 {
            let floor = self.dehaze * 0.08;
            (l - floor) / (1.0 - floor)
        } else {
            l + (-self.dehaze) * 0.22 * (0.72 - l)
        }
    }

    pub(super) fn color(&self, rgb: [f32; 3]) -> [f32; 3] {
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

    pub(super) fn black_and_white(&self, rgb: [f32; 3]) -> [f32; 3] {
        let (hue, sat, _) = rgb_to_hsv(rgb);
        let mut gray = luma(rgb);
        for &(center, amount) in &self.gray_mixer {
            gray += hue_weight(hue, center) * amount * 0.35 * sat;
        }
        [gray; 3]
    }

    pub(super) fn grade(&self, rgb: [f32; 3]) -> [f32; 3] {
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
