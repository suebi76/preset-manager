//! Aussagen über die Wirkung („heller“, „Grün entsättigt“), aus denen die
//! Oberfläche einen Satz baut.

use crate::preset::settings::PresetSettings;

use super::color::{HslColor, WhiteBalance};
use super::curve::{curve_value, is_linear, CurveChannel};

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

    pub(super) fn color_facts(&self, facts: &mut Vec<Fact>) {
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
