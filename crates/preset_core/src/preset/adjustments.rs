//! Was ein Preset einstellt – geordnet wie in Camera Raw und in Worten.
//!
//! [`super::settings`] liefert die Rohwerte. Hier werden sie gedeutet:
//!
//! - [`PresetSettings::sliders`]: die Regler eines Bereichs, die vom Standard
//!   abweichen
//! - [`PresetSettings::point_curves`], [`PresetSettings::mixer`],
//!   [`PresetSettings::color_grading`], [`PresetSettings::white_balance`]:
//!   was sich nicht als einzelner Regler zeigen lässt
//! - [`PresetSettings::facts`]: Aussagen wie „heller“ oder „Grün entsättigt“,
//!   aus denen die Oberfläche einen Satz baut
//!
//! Alles bleibt sprachneutral; die Texte stehen in der Oberfläche.

use super::settings::{CurvePoint, PresetSettings};

/// Die Bereiche von Camera Raw, in der Reihenfolge der Bedienfelder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Panel {
    /// Grundeinstellungen: Licht, Präsenz.
    Basic,
    /// Gradationskurve (Regler der parametrischen Kurve).
    Curve,
    /// Details: Schärfen, Rauschreduzierung.
    Detail,
    /// Effekte: Vignette, Körnung.
    Effects,
    /// Kalibrierung.
    Calibration,
    /// Optik: Vignettierung des Objektivs.
    Optics,
}

impl Panel {
    /// Alle Bereiche, die nur aus Reglern bestehen.
    pub const ALL: [Panel; 6] = [
        Panel::Basic,
        Panel::Curve,
        Panel::Detail,
        Panel::Effects,
        Panel::Calibration,
        Panel::Optics,
    ];
}

/// Wie ein Regler aufgebaut ist.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scale {
    /// Mit Mittelpunkt, z. B. −100 … +100.
    Bipolar {
        /// Linkes Ende.
        min: f32,
        /// Rechtes Ende.
        max: f32,
    },
    /// Von 0 an, z. B. 0 … 100.
    Unipolar {
        /// Rechtes Ende.
        max: f32,
    },
}

/// Ein Regler, wie Camera Raw ihn kennt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderSpec {
    /// Der Schlüssel in der Datei ohne `crs:`, z. B. `Exposure2012`.
    pub key: &'static str,
    /// Der Bereich in Camera Raw.
    pub panel: Panel,
    /// Aufbau des Reglers.
    pub scale: Scale,
    /// Der Wert, bei dem der Regler nichts bewirkt.
    pub neutral: f32,
    /// Nachkommastellen bei der Anzeige.
    pub decimals: u8,
    /// Nur zeigen, wenn dieser Regler vom Standard abweicht (z. B. die
    /// Größe der Körnung nur bei Körnung).
    pub requires: Option<&'static str>,
}

const fn bipolar(key: &'static str, panel: Panel) -> SliderSpec {
    SliderSpec {
        key,
        panel,
        scale: Scale::Bipolar {
            min: -100.0,
            max: 100.0,
        },
        neutral: 0.0,
        decimals: 0,
        requires: None,
    }
}

const fn unipolar(key: &'static str, panel: Panel, neutral: f32) -> SliderSpec {
    SliderSpec {
        key,
        panel,
        scale: Scale::Unipolar { max: 100.0 },
        neutral,
        decimals: 0,
        requires: None,
    }
}

const fn requiring(spec: SliderSpec, key: &'static str) -> SliderSpec {
    SliderSpec {
        requires: Some(key),
        ..spec
    }
}

/// Alle Regler, die die App zeigt.
pub const SLIDERS: &[SliderSpec] = &[
    SliderSpec {
        key: "Exposure2012",
        panel: Panel::Basic,
        scale: Scale::Bipolar {
            min: -5.0,
            max: 5.0,
        },
        neutral: 0.0,
        decimals: 2,
        requires: None,
    },
    bipolar("IncrementalTemperature", Panel::Basic),
    bipolar("IncrementalTint", Panel::Basic),
    bipolar("Contrast2012", Panel::Basic),
    bipolar("Highlights2012", Panel::Basic),
    bipolar("Shadows2012", Panel::Basic),
    bipolar("Whites2012", Panel::Basic),
    bipolar("Blacks2012", Panel::Basic),
    bipolar("Texture", Panel::Basic),
    bipolar("Clarity2012", Panel::Basic),
    bipolar("Dehaze", Panel::Basic),
    bipolar("Vibrance", Panel::Basic),
    bipolar("Saturation", Panel::Basic),
    bipolar("ParametricHighlights", Panel::Curve),
    bipolar("ParametricLights", Panel::Curve),
    bipolar("ParametricDarks", Panel::Curve),
    bipolar("ParametricShadows", Panel::Curve),
    SliderSpec {
        key: "Sharpness",
        panel: Panel::Detail,
        scale: Scale::Unipolar { max: 150.0 },
        neutral: 40.0,
        decimals: 0,
        requires: None,
    },
    SliderSpec {
        key: "SharpenRadius",
        panel: Panel::Detail,
        scale: Scale::Unipolar { max: 3.0 },
        neutral: 1.0,
        decimals: 1,
        requires: None,
    },
    unipolar("SharpenDetail", Panel::Detail, 25.0),
    unipolar("SharpenEdgeMasking", Panel::Detail, 0.0),
    unipolar("LuminanceSmoothing", Panel::Detail, 0.0),
    unipolar("ColorNoiseReduction", Panel::Detail, 25.0),
    bipolar("PostCropVignetteAmount", Panel::Effects),
    requiring(
        unipolar("PostCropVignetteMidpoint", Panel::Effects, 50.0),
        "PostCropVignetteAmount",
    ),
    requiring(
        bipolar("PostCropVignetteRoundness", Panel::Effects),
        "PostCropVignetteAmount",
    ),
    requiring(
        unipolar("PostCropVignetteFeather", Panel::Effects, 50.0),
        "PostCropVignetteAmount",
    ),
    requiring(
        unipolar("PostCropVignetteHighlightContrast", Panel::Effects, 0.0),
        "PostCropVignetteAmount",
    ),
    unipolar("GrainAmount", Panel::Effects, 0.0),
    requiring(unipolar("GrainSize", Panel::Effects, 25.0), "GrainAmount"),
    requiring(
        unipolar("GrainFrequency", Panel::Effects, 50.0),
        "GrainAmount",
    ),
    bipolar("ShadowTint", Panel::Calibration),
    bipolar("RedHue", Panel::Calibration),
    bipolar("RedSaturation", Panel::Calibration),
    bipolar("GreenHue", Panel::Calibration),
    bipolar("GreenSaturation", Panel::Calibration),
    bipolar("BlueHue", Panel::Calibration),
    bipolar("BlueSaturation", Panel::Calibration),
    bipolar("VignetteAmount", Panel::Optics),
];

/// Ein Regler mit dem Wert aus der Datei.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slider {
    /// Wie der Regler aufgebaut ist.
    pub spec: &'static SliderSpec,
    /// Der Wert aus der Datei.
    pub value: f32,
}

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
    fn strength(&self) -> f32 {
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
    fn is_neutral(&self) -> bool {
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

/// Ein Kanal der Gradationskurve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveChannel {
    /// Alle Kanäle (Helligkeit).
    Master,
    /// Rot.
    Red,
    /// Grün.
    Green,
    /// Blau.
    Blue,
}

impl CurveChannel {
    /// Alle Kanäle.
    pub const ALL: [CurveChannel; 4] = [
        CurveChannel::Master,
        CurveChannel::Red,
        CurveChannel::Green,
        CurveChannel::Blue,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Master => "ToneCurvePV2012",
            Self::Red => "ToneCurvePV2012Red",
            Self::Green => "ToneCurvePV2012Green",
            Self::Blue => "ToneCurvePV2012Blue",
        }
    }
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

/// Eine Aussage darüber, was das Preset bewirkt.
#[derive(Debug, Clone, PartialEq)]
pub enum Fact {
    /// Schwarzweiß.
    BlackAndWhite,
    /// Heller.
    Brighter,
    /// Dunkler.
    Darker,
    /// Mehr Kontrast.
    MoreContrast,
    /// Weniger Kontrast.
    LessContrast,
    /// Schwarz angehoben – matter, „ausgeblichener“ Look.
    FadedBlacks,
    /// Weiß abgesenkt.
    MutedWhites,
    /// Tiefen aufgehellt.
    BrighterShadows,
    /// Tiefen abgedunkelt.
    DarkerShadows,
    /// Lichter abgeschwächt.
    SofterHighlights,
    /// Wärmer.
    Warmer,
    /// Kühler.
    Cooler,
    /// Richtung Magenta getönt.
    Magenta,
    /// Richtung Grün getönt.
    Greener,
    /// Kräftigere Farben.
    MoreColorful,
    /// Gedämpfte Farben.
    LessColorful,
    /// Eine Farbe des Farbmischers deutlich verändert.
    Color {
        /// Die Farbe.
        color: HslColor,
        /// Was mit ihr geschieht.
        change: ColorChange,
    },
    /// Farbig getönt (Color Grading).
    Toned {
        /// Wo.
        zone: ToneZone,
        /// Farbton in Grad.
        hue: f32,
    },
    /// Mehr Struktur und Klarheit.
    MoreTexture,
    /// Weicher.
    Softer,
    /// Dunst entfernt.
    LessHaze,
    /// Dunstiger, verträumter.
    MoreHaze,
    /// Dunkle Ränder.
    DarkVignette,
    /// Helle Ränder.
    LightVignette,
    /// Filmkorn.
    Grain,
    /// Baut auf einem Profil auf.
    Profile(String),
    /// Der Look steckt in einer Farbtabelle.
    ColorTable,
    /// Lokale Anpassungen (Anzahl).
    LocalAdjustments(usize),
}

/// Was der Farbmischer mit einer Farbe macht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChange {
    /// Kräftiger.
    Saturated,
    /// Entsättigt.
    Desaturated,
    /// Heller.
    Lighter,
    /// Dunkler.
    Darker,
    /// Farbton verschoben.
    Shifted,
}

/// Wo das Color Grading tönt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneZone {
    /// Tiefen.
    Shadows,
    /// Mitteltöne.
    Midtones,
    /// Lichter.
    Highlights,
    /// Das ganze Bild.
    Overall,
}

impl PresetSettings {
    /// Die Regler eines Bereichs, die vom Standard abweichen.
    pub fn sliders(&self, panel: Panel) -> Vec<Slider> {
        SLIDERS
            .iter()
            .filter(|spec| spec.panel == panel)
            .filter(|spec| spec.requires.is_none_or(|key| self.changed(key).is_some()))
            .filter_map(|spec| {
                let value = self.number(spec.key)?;
                ((value - spec.neutral).abs() > 0.004).then_some(Slider { spec, value })
            })
            .collect()
    }

    /// Wert eines bekannten Reglers, wenn er vom Standard abweicht.
    fn changed(&self, key: &str) -> Option<f32> {
        let spec = SLIDERS.iter().find(|spec| spec.key == key)?;
        let value = self.number(key)?;
        ((value - spec.neutral).abs() > 0.004).then_some(value)
    }

    /// Ein Wert oder 0.
    fn value(&self, key: &str) -> f32 {
        self.number(key).unwrap_or(0.0)
    }

    /// `true`, wenn das Preset in Schwarzweiß umwandelt.
    pub fn is_black_and_white(&self) -> bool {
        self.flag("ConvertToGrayscale") == Some(true)
    }

    /// Die Punktkurven, die nicht gerade sind.
    pub fn point_curves(&self) -> Vec<(CurveChannel, &[CurvePoint])> {
        CurveChannel::ALL
            .into_iter()
            .filter_map(|channel| {
                let points = self.curve(channel.key())?;
                (!is_linear(points)).then_some((channel, points))
            })
            .collect()
    }

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

    /// Wie viele lokale Anpassungen etwas bewirken.
    pub fn effective_local_adjustments(&self) -> usize {
        self.local_adjustments
            .iter()
            .filter(|adjustment| adjustment.has_effect())
            .count()
    }

    /// Was das Preset bewirkt, als Liste von Aussagen – die wichtigsten
    /// zuerst. Die Schwellen sind so gewählt, dass kleine Korrekturen nicht
    /// erwähnt werden.
    pub fn facts(&self) -> Vec<Fact> {
        let mut facts = Vec::new();
        let black_and_white = self.is_black_and_white();
        if black_and_white {
            facts.push(Fact::BlackAndWhite);
        }

        let master = self
            .curve(CurveChannel::Master.key())
            .filter(|points| !is_linear(points));
        let at = |x: f32| master.map_or(x, |points| curve_value(points, x));

        let brightness = self.value("Exposure2012") * 40.0 + (at(128.0) - 128.0) / 2.0;
        if brightness >= 6.0 {
            facts.push(Fact::Brighter);
        } else if brightness <= -6.0 {
            facts.push(Fact::Darker);
        }

        let contrast = self.value("Contrast2012")
            + (at(191.0) - at(64.0) - 127.0)
            + (self.value("ParametricLights") - self.value("ParametricDarks")) / 2.0
            + (self.value("ParametricHighlights") - self.value("ParametricShadows")) / 4.0;
        if contrast >= 10.0 {
            facts.push(Fact::MoreContrast);
        } else if contrast <= -10.0 {
            facts.push(Fact::LessContrast);
        }
        if at(0.0) >= 12.0 {
            facts.push(Fact::FadedBlacks);
        }
        if at(255.0) <= 243.0 {
            facts.push(Fact::MutedWhites);
        }
        let shadows = self.value("Shadows2012");
        if shadows >= 25.0 {
            facts.push(Fact::BrighterShadows);
        } else if shadows <= -25.0 {
            facts.push(Fact::DarkerShadows);
        }
        if self.value("Highlights2012") <= -25.0 {
            facts.push(Fact::SofterHighlights);
        }

        if !black_and_white {
            self.color_facts(&mut facts);
        }
        if let Some(grading) = self.color_grading() {
            let zones = [
                (ToneZone::Shadows, grading.shadows, 8.0),
                (ToneZone::Midtones, grading.midtones, 8.0),
                (ToneZone::Highlights, grading.highlights, 8.0),
                (ToneZone::Overall, grading.global, 5.0),
            ];
            for (zone, wheel, threshold) in zones {
                if wheel.saturation >= threshold {
                    facts.push(Fact::Toned {
                        zone,
                        hue: wheel.hue,
                    });
                }
            }
        }

        let texture = self.value("Texture") + self.value("Clarity2012");
        if texture >= 15.0 {
            facts.push(Fact::MoreTexture);
        } else if texture <= -15.0 {
            facts.push(Fact::Softer);
        }
        let dehaze = self.value("Dehaze");
        if dehaze >= 8.0 {
            facts.push(Fact::LessHaze);
        } else if dehaze <= -8.0 {
            facts.push(Fact::MoreHaze);
        }
        let vignette = self.value("PostCropVignetteAmount");
        if vignette <= -10.0 {
            facts.push(Fact::DarkVignette);
        } else if vignette >= 10.0 {
            facts.push(Fact::LightVignette);
        }
        if self.value("GrainAmount") >= 5.0 {
            facts.push(Fact::Grain);
        }

        if let Some(look) = self.look.as_ref().filter(|look| !look.name.is_empty()) {
            facts.push(Fact::Profile(look.name.clone()));
        } else if self.uses_color_table {
            facts.push(Fact::ColorTable);
        }
        let local = self.effective_local_adjustments();
        if local > 0 {
            facts.push(Fact::LocalAdjustments(local));
        }
        facts
    }

    fn color_facts(&self, facts: &mut Vec<Fact>) {
        let (mut temperature, mut tint) = (
            self.value("IncrementalTemperature"),
            self.value("IncrementalTint"),
        );
        if let WhiteBalance::Custom {
            temperature: kelvin,
            tint: fixed_tint,
        } = self.white_balance()
        {
            // Gegenüber Tageslicht (etwa 5500 K).
            if let Some(kelvin) = kelvin {
                temperature += (kelvin - 5500.0) / 100.0;
            }
            if let Some(fixed_tint) = fixed_tint {
                tint += fixed_tint - 10.0;
            }
        }
        if temperature >= 5.0 {
            facts.push(Fact::Warmer);
        } else if temperature <= -5.0 {
            facts.push(Fact::Cooler);
        }
        if tint >= 8.0 {
            facts.push(Fact::Magenta);
        } else if tint <= -8.0 {
            facts.push(Fact::Greener);
        }

        let colorfulness = self.value("Saturation") + self.value("Vibrance") / 2.0;
        if colorfulness >= 10.0 {
            facts.push(Fact::MoreColorful);
        } else if colorfulness <= -10.0 {
            facts.push(Fact::LessColorful);
        }

        let mut rows: Vec<(f32, Fact)> = self
            .mixer()
            .into_iter()
            .filter_map(|row| {
                let change = if row.saturation <= -25.0 {
                    ColorChange::Desaturated
                } else if row.saturation >= 25.0 {
                    ColorChange::Saturated
                } else if row.luminance >= 25.0 {
                    ColorChange::Lighter
                } else if row.luminance <= -25.0 {
                    ColorChange::Darker
                } else if row.hue.abs() >= 25.0 {
                    ColorChange::Shifted
                } else {
                    return None;
                };
                Some((
                    row.strength(),
                    Fact::Color {
                        color: row.color,
                        change,
                    },
                ))
            })
            .collect();
        rows.sort_by(|a, b| b.0.total_cmp(&a.0));
        facts.extend(rows.into_iter().take(3).map(|(_, fact)| fact));
    }
}

/// `true`, wenn die Kurve die Gerade von (0, 0) nach (255, 255) ist.
fn is_linear(points: &[CurvePoint]) -> bool {
    points.iter().all(|(x, y)| (x - y).abs() < 0.5)
}

/// Der Wert einer Punktkurve an der Stelle `x` (0–255), wie Camera Raw sie
/// zeichnet: eine glatte Kurve (natürlicher kubischer Spline) durch die
/// Punkte, auf 0–255 begrenzt. Vor dem ersten und nach dem letzten Punkt
/// bleibt sie waagerecht.
pub fn curve_value(points: &[CurvePoint], x: f32) -> f32 {
    let mut sorted: Vec<CurvePoint> = points.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    sorted.dedup_by(|a, b| (a.0 - b.0).abs() < f32::EPSILON);
    let (Some(first), Some(last)) = (sorted.first().copied(), sorted.last().copied()) else {
        return x;
    };
    if sorted.len() == 1 || x <= first.0 {
        return first.1.clamp(0.0, 255.0);
    }
    if x >= last.0 {
        return last.1.clamp(0.0, 255.0);
    }

    let second_derivatives = spline_second_derivatives(&sorted);
    let index = sorted
        .windows(2)
        .position(|pair| x <= pair[1].0)
        .unwrap_or(sorted.len() - 2);
    let (x0, y0) = sorted[index];
    let (x1, y1) = sorted[index + 1];
    let (m0, m1) = (second_derivatives[index], second_derivatives[index + 1]);
    let h = x1 - x0;
    let a = (x1 - x) / h;
    let b = (x - x0) / h;
    let y = a * y0 + b * y1 + ((a * a * a - a) * m0 + (b * b * b - b) * m1) * h * h / 6.0;
    y.clamp(0.0, 255.0)
}

/// Zweite Ableitungen eines natürlichen kubischen Splines (Thomas-Verfahren).
fn spline_second_derivatives(points: &[CurvePoint]) -> Vec<f32> {
    let n = points.len();
    let mut result = vec![0.0; n];
    if n < 3 {
        return result;
    }
    let mut upper = vec![0.0; n];
    let mut rhs = vec![0.0; n];
    for i in 1..n - 1 {
        let (x_prev, y_prev) = points[i - 1];
        let (x, y) = points[i];
        let (x_next, y_next) = points[i + 1];
        let sigma = (x - x_prev) / (x_next - x_prev);
        let p = sigma * upper[i - 1] + 2.0;
        upper[i] = (sigma - 1.0) / p;
        let slope = (y_next - y) / (x_next - x) - (y - y_prev) / (x - x_prev);
        rhs[i] = (6.0 * slope / (x_next - x_prev) - sigma * rhs[i - 1]) / p;
    }
    for i in (1..n - 1).rev() {
        result[i] = upper[i] * result[i + 1] + rhs[i];
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::settings::parse_settings;
    use super::*;

    fn settings(attributes: &str, body: &str) -> PresetSettings {
        parse_settings(&format!(
            r#"<x:xmpmeta><rdf:RDF><rdf:Description crs:PresetType="Normal" {attributes}>{body}</rdf:Description></rdf:RDF></x:xmpmeta>"#
        ))
        .unwrap()
    }

    fn curve(points: &[(u8, u8)]) -> String {
        let items: String = points
            .iter()
            .map(|(x, y)| format!("<rdf:li>{x}, {y}</rdf:li>"))
            .collect();
        format!("<crs:ToneCurvePV2012><rdf:Seq>{items}</rdf:Seq></crs:ToneCurvePV2012>")
    }

    #[test]
    fn zeigt_nur_geaenderte_regler() {
        let settings = settings(
            r#"crs:Exposure2012="+0.50" crs:Contrast2012="0" crs:Sharpness="40" crs:SharpenRadius="+1.0" crs:LuminanceSmoothing="20""#,
            "",
        );

        let basic: Vec<_> = settings
            .sliders(Panel::Basic)
            .iter()
            .map(|s| (s.spec.key, s.value))
            .collect();
        let detail: Vec<_> = settings
            .sliders(Panel::Detail)
            .iter()
            .map(|s| s.spec.key)
            .collect();

        assert_eq!(basic, vec![("Exposure2012", 0.5)]);
        assert_eq!(detail, vec!["LuminanceSmoothing"]);
    }

    #[test]
    fn abhaengige_regler_nur_mit_ihrem_hauptregler() {
        let without = settings(r#"crs:GrainAmount="0" crs:GrainSize="40""#, "");
        let with = settings(r#"crs:GrainAmount="20" crs:GrainSize="40""#, "");

        assert!(without.sliders(Panel::Effects).is_empty());
        assert_eq!(with.sliders(Panel::Effects).len(), 2);
    }

    #[test]
    fn gerade_kurven_zaehlen_nicht() {
        let linear = settings("", &curve(&[(0, 0), (255, 255)]));
        let bent = settings("", &curve(&[(0, 20), (128, 128), (255, 255)]));

        assert!(linear.point_curves().is_empty());
        assert_eq!(bent.point_curves().len(), 1);
    }

    #[test]
    fn kurve_geht_durch_ihre_punkte() {
        let points = [(0.0, 20.0), (64.0, 50.0), (192.0, 210.0), (255.0, 240.0)];

        for (x, y) in points {
            assert!((curve_value(&points, x) - y).abs() < 0.01, "{x}");
        }
        assert!(curve_value(&points, 128.0) > 100.0 && curve_value(&points, 128.0) < 160.0);
        assert_eq!(curve_value(&[(0.0, 0.0), (255.0, 255.0)], 100.0), 100.0);
    }

    #[test]
    fn beschreibt_einen_matten_warmen_look() {
        let settings = settings(
            r#"crs:Exposure2012="+0.30" crs:IncrementalTemperature="+15" crs:Saturation="-20"
               crs:SaturationAdjustmentGreen="-60" crs:GrainAmount="25" crs:PostCropVignetteAmount="-20"
               crs:SplitToningShadowHue="200" crs:SplitToningShadowSaturation="20""#,
            &curve(&[(0, 30), (128, 128), (255, 235)]),
        );

        let facts = settings.facts();

        for expected in [
            Fact::Brighter,
            Fact::FadedBlacks,
            Fact::MutedWhites,
            Fact::Warmer,
            Fact::LessColorful,
            Fact::Color {
                color: HslColor::Green,
                change: ColorChange::Desaturated,
            },
            Fact::Toned {
                zone: ToneZone::Shadows,
                hue: 200.0,
            },
            Fact::DarkVignette,
            Fact::Grain,
        ] {
            assert!(facts.contains(&expected), "{expected:?} fehlt in {facts:?}");
        }
        assert!(!facts.contains(&Fact::MoreContrast));
    }

    #[test]
    fn schwarzweiss_ohne_farbaussagen() {
        let settings = settings(
            r#"crs:ConvertToGrayscale="True" crs:Saturation="+40" crs:GrayMixerRed="-20" crs:SaturationAdjustmentRed="+50""#,
            "",
        );

        assert_eq!(settings.facts(), vec![Fact::BlackAndWhite]);
        assert!(settings.mixer().is_empty());
        assert_eq!(settings.gray_mixer(), vec![(HslColor::Red, -20.0)]);
    }

    #[test]
    fn weissabgleich() {
        assert_eq!(settings("", "").white_balance(), WhiteBalance::AsShot);
        assert_eq!(
            settings(r#"crs:WhiteBalance="Auto""#, "").white_balance(),
            WhiteBalance::Named("Auto".to_owned())
        );
        let custom = settings(
            r#"crs:WhiteBalance="Custom" crs:Temperature="7500" crs:Tint="+10""#,
            "",
        );
        assert_eq!(
            custom.white_balance(),
            WhiteBalance::Custom {
                temperature: Some(7500.0),
                tint: Some(10.0)
            }
        );
        assert!(custom.facts().contains(&Fact::Warmer));
    }

    #[test]
    fn neutrales_preset_hat_keine_aussagen() {
        let settings = settings(
            r#"crs:Exposure2012="0.00" crs:Contrast2012="0" crs:ColorGradeBlending="50""#,
            &curve(&[(0, 0), (255, 255)]),
        );

        assert!(settings.facts().is_empty());
        assert_eq!(settings.color_grading(), None);
    }
}
