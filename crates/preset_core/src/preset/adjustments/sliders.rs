//! Die Regler von Camera Raw: Bereiche, Wertebereiche und die Tabelle
//! [`SLIDERS`] – dazu, welche Regler ein Preset vom Standard abweichend
//! setzt.

use crate::preset::settings::PresetSettings;

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
    pub(super) fn changed(&self, key: &str) -> Option<f32> {
        let spec = SLIDERS.iter().find(|spec| spec.key == key)?;
        let value = self.number(key)?;
        ((value - spec.neutral).abs() > 0.004).then_some(value)
    }

    /// Ein Wert oder 0.
    pub(super) fn value(&self, key: &str) -> f32 {
        self.number(key).unwrap_or(0.0)
    }

    /// `true`, wenn das Preset in Schwarzweiß umwandelt.
    pub fn is_black_and_white(&self) -> bool {
        self.flag("ConvertToGrayscale") == Some(true)
    }
}
