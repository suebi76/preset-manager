//! Farbe: Farbmischer (HSL), Color Grading und Weißabgleich.

use crate::preset::settings::PresetSettings;

/// Die acht Farben des Farbmischers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HslColor {
    /// Rot.
    Red,
    /// Orange.
    Orange,
    /// Gelb.
    Yellow,
    /// Grün.
    Green,
    /// Aquamarin.
    Aqua,
    /// Blau.
    Blue,
    /// Lila.
    Purple,
    /// Magenta.
    Magenta,
}

impl HslColor {
    /// Alle Farben in der Reihenfolge von Camera Raw.
    pub const ALL: [HslColor; 8] = [
        HslColor::Red,
        HslColor::Orange,
        HslColor::Yellow,
        HslColor::Green,
        HslColor::Aqua,
        HslColor::Blue,
        HslColor::Purple,
        HslColor::Magenta,
    ];

    /// Der Teil des Schlüssels, z. B. `Red` in `HueAdjustmentRed`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Aqua => "Aqua",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Magenta => "Magenta",
        }
    }

    /// Ungefährer Farbton in Grad (0 = Rot), zum Einfärben der Anzeige.
    pub fn hue_degrees(self) -> f32 {
        match self {
            Self::Red => 0.0,
            Self::Orange => 30.0,
            Self::Yellow => 55.0,
            Self::Green => 120.0,
            Self::Aqua => 180.0,
            Self::Blue => 225.0,
            Self::Purple => 270.0,
            Self::Magenta => 310.0,
        }
    }
}

/// Eine Zeile des Farbmischers: Farbton, Sättigung, Luminanz (je −100 … +100).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MixerRow {
    /// Die Farbe.
    pub color: HslColor,
    /// Verschiebung des Farbtons.
    pub hue: f32,
    /// Sättigung.
    pub saturation: f32,
    /// Luminanz (Helligkeit der Farbe).
    pub luminance: f32,
}

impl MixerRow {
    pub(super) fn strength(&self) -> f32 {
        self.hue
            .abs()
            .max(self.saturation.abs())
            .max(self.luminance.abs())
    }
}

/// Ein Rad des Color Gradings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wheel {
    /// Farbton in Grad, 0–359.
    pub hue: f32,
    /// Sättigung, 0–100.
    pub saturation: f32,
    /// Luminanz, −100 … +100.
    pub luminance: f32,
}

impl Wheel {
    pub(super) fn is_neutral(&self) -> bool {
        self.saturation.abs() < 0.5 && self.luminance.abs() < 0.5
    }
}

/// Das Color Grading (früher Teiltonung).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorGrading {
    /// Tiefen.
    pub shadows: Wheel,
    /// Mitteltöne.
    pub midtones: Wheel,
    /// Lichter.
    pub highlights: Wheel,
    /// Global.
    pub global: Wheel,
    /// Überblendung, 0–100 (Standard 50).
    pub blending: f32,
    /// Abgleich, −100 … +100.
    pub balance: f32,
}

/// Der Weißabgleich des Presets.
#[derive(Debug, Clone, PartialEq)]
pub enum WhiteBalance {
    /// Nicht gesetzt oder „Wie Aufnahme“ – das Preset ändert ihn nicht.
    AsShot,
    /// Eine Vorgabe wie „Auto“ oder „Tageslicht“.
    Named(String),
    /// Feste Werte – gelten nur für Rohdaten.
    Custom {
        /// Farbtemperatur in Kelvin.
        temperature: Option<f32>,
        /// Tönung, −150 … +150.
        tint: Option<f32>,
    },
}

impl PresetSettings {
    /// Die geänderten Zeilen des Farbmischers (bei Farbe).
    pub fn mixer(&self) -> Vec<MixerRow> {
        if self.is_black_and_white() {
            return Vec::new();
        }
        HslColor::ALL
            .into_iter()
            .map(|color| MixerRow {
                color,
                hue: self.value(&format!("HueAdjustment{}", color.key())),
                saturation: self.value(&format!("SaturationAdjustment{}", color.key())),
                luminance: self.value(&format!("LuminanceAdjustment{}", color.key())),
            })
            .filter(|row| row.strength() >= 0.5)
            .collect()
    }

    /// Der Schwarzweiß-Mischer: wie hell jede Farbe in Grau wird.
    pub fn gray_mixer(&self) -> Vec<(HslColor, f32)> {
        if !self.is_black_and_white() {
            return Vec::new();
        }
        HslColor::ALL
            .into_iter()
            .filter_map(|color| {
                let value = self.number(&format!("GrayMixer{}", color.key()))?;
                (value.abs() >= 0.5).then_some((color, value))
            })
            .collect()
    }

    /// Das Color Grading, falls eines der Räder etwas bewirkt.
    pub fn color_grading(&self) -> Option<ColorGrading> {
        let wheel = |hue: &str, saturation: &str, luminance: &str| Wheel {
            hue: self.value(hue),
            saturation: self.value(saturation),
            luminance: self.value(luminance),
        };
        let grading = ColorGrading {
            shadows: wheel(
                "SplitToningShadowHue",
                "SplitToningShadowSaturation",
                "ColorGradeShadowLum",
            ),
            midtones: wheel(
                "ColorGradeMidtoneHue",
                "ColorGradeMidtoneSat",
                "ColorGradeMidtoneLum",
            ),
            highlights: wheel(
                "SplitToningHighlightHue",
                "SplitToningHighlightSaturation",
                "ColorGradeHighlightLum",
            ),
            global: wheel(
                "ColorGradeGlobalHue",
                "ColorGradeGlobalSat",
                "ColorGradeGlobalLum",
            ),
            blending: self.number("ColorGradeBlending").unwrap_or(50.0),
            balance: self.value("SplitToningBalance"),
        };
        let wheels = [
            grading.shadows,
            grading.midtones,
            grading.highlights,
            grading.global,
        ];
        (!wheels.iter().all(Wheel::is_neutral)).then_some(grading)
    }

    /// Der Weißabgleich.
    pub fn white_balance(&self) -> WhiteBalance {
        match self.text("WhiteBalance").map(str::trim) {
            None | Some("") | Some("As Shot") => WhiteBalance::AsShot,
            Some("Custom") => WhiteBalance::Custom {
                temperature: self.number("Temperature"),
                tint: self.number("Tint"),
            },
            Some(named) => WhiteBalance::Named(named.to_owned()),
        }
    }
}
