//! Texte: Zahlen, Namen der Regler und Farben, Sätze für die
//! Zusammenfassung – auf Deutsch und Englisch.

use preset_core::preset::adjustments::{ColorChange, Fact, HslColor, Panel, ToneZone};
use preset_core::preset::settings::MaskKind;

use crate::texts::{self, tr, tr_format, Language};

/// Eine Zahl wie in Camera Raw: mit Vorzeichen bei Reglern mit Mittelpunkt,
/// im Deutschen mit Komma.
pub(super) fn number(value: f32, decimals: u8, signed: bool) -> String {
    let rounded = if decimals == 0 { value.round() } else { value };
    let mut text = format!("{:.*}", usize::from(decimals), rounded.abs());
    if texts::language() == Language::German {
        text = text.replace('.', ",");
    }
    let zero = rounded.abs() < 0.5 * 10f32.powi(-i32::from(decimals));
    match (signed, zero, rounded < 0.0) {
        (_, true, _) => text,
        (_, false, true) => format!("−{text}"),
        (true, false, false) => format!("+{text}"),
        (false, false, false) => text,
    }
}

pub(super) fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

pub(super) fn phrase(fact: &Fact) -> String {
    let fixed = match fact {
        Fact::BlackAndWhite => tr("Schwarzweiß", "black & white"),
        Fact::Brighter => tr("heller", "brighter"),
        Fact::Darker => tr("dunkler", "darker"),
        Fact::MoreContrast => tr("mehr Kontrast", "more contrast"),
        Fact::LessContrast => tr("weniger Kontrast", "less contrast"),
        Fact::FadedBlacks => tr("matte, angehobene Schwarztöne", "faded, lifted blacks"),
        Fact::MutedWhites => tr("gedämpftes Weiß", "muted whites"),
        Fact::BrighterShadows => tr("aufgehellte Tiefen", "lifted shadows"),
        Fact::DarkerShadows => tr("tiefere Schatten", "deeper shadows"),
        Fact::SofterHighlights => tr("zurückgenommene Lichter", "softened highlights"),
        Fact::Warmer => tr("wärmer", "warmer"),
        Fact::Cooler => tr("kühler", "cooler"),
        Fact::Magenta => tr("Stich ins Magenta", "a magenta tint"),
        Fact::Greener => tr("Stich ins Grüne", "a green tint"),
        Fact::MoreColorful => tr("kräftigere Farben", "richer colors"),
        Fact::LessColorful => tr("gedämpfte Farben", "muted colors"),
        Fact::MoreTexture => tr("mehr Struktur und Klarheit", "more texture and clarity"),
        Fact::Softer => tr("weicher", "softer"),
        Fact::LessHaze => tr("Dunst entfernt", "haze removed"),
        Fact::MoreHaze => tr("dunstiger, verträumter", "hazier, dreamier"),
        Fact::DarkVignette => tr("dunkle Ränder (Vignette)", "darkened edges (vignette)"),
        Fact::LightVignette => tr("aufgehellte Ränder", "lightened edges"),
        Fact::Grain => tr("Filmkorn", "film grain"),
        Fact::ColorTable => tr(
            "Der Look steckt in einer Farbtabelle (LUT).",
            "The look lives in a color table (LUT).",
        ),
        Fact::Color { color, change } => {
            let name = color_name(*color);
            return match change {
                ColorChange::Saturated => tr_format!("{name} kräftiger", "stronger {name}"),
                ColorChange::Desaturated => tr_format!("{name} entsättigt", "{name} desaturated"),
                ColorChange::Lighter => tr_format!("{name} heller", "lighter {name}"),
                ColorChange::Darker => tr_format!("{name} dunkler", "darker {name}"),
                ColorChange::Shifted => {
                    tr_format!("{name} im Farbton verschoben", "{name} hue shifted")
                }
            };
        }
        Fact::Toned { zone, hue } => {
            let color = hue_name(*hue);
            return match zone {
                ToneZone::Shadows => tr_format!("Tiefen {color} getönt", "shadows tinted {color}"),
                ToneZone::Midtones => {
                    tr_format!("Mitteltöne {color} getönt", "midtones tinted {color}")
                }
                ToneZone::Highlights => {
                    tr_format!("Lichter {color} getönt", "highlights tinted {color}")
                }
                ToneZone::Overall => {
                    tr_format!("insgesamt {color} getönt", "tinted {color} overall")
                }
            };
        }
        Fact::Profile(name) => {
            return tr_format!(
                "Baut auf dem Profil „{name}“ auf.",
                "Built on the profile “{name}”."
            )
        }
        Fact::LocalAdjustments(count) => {
            return if *count == 1 {
                tr(
                    "Eine lokale Anpassung mit Maske.",
                    "One local adjustment with a mask.",
                )
                .to_owned()
            } else {
                tr_format!(
                    "{count} lokale Anpassungen mit Masken.",
                    "{count} local adjustments with masks."
                )
            }
        }
    };
    fixed.to_owned()
}

pub(super) fn color_name(color: HslColor) -> &'static str {
    match color {
        HslColor::Red => tr("Rot", "red"),
        HslColor::Orange => tr("Orange", "orange"),
        HslColor::Yellow => tr("Gelb", "yellow"),
        HslColor::Green => tr("Grün", "green"),
        HslColor::Aqua => tr("Aquamarin", "aqua"),
        HslColor::Blue => tr("Blau", "blue"),
        HslColor::Purple => tr("Lila", "purple"),
        HslColor::Magenta => tr("Magenta", "magenta"),
    }
}

/// Ein Wort für einen Farbton in Grad.
fn hue_name(degrees: f32) -> &'static str {
    match degrees.rem_euclid(360.0) {
        d if d < 15.0 => tr("rot", "red"),
        d if d < 45.0 => tr("orange", "orange"),
        d if d < 70.0 => tr("gelb", "yellow"),
        d if d < 160.0 => tr("grün", "green"),
        d if d < 200.0 => tr("türkis", "teal"),
        d if d < 250.0 => tr("blau", "blue"),
        d if d < 290.0 => tr("violett", "violet"),
        d if d < 340.0 => tr("magenta", "magenta"),
        _ => tr("rot", "red"),
    }
}

pub(super) fn panel_title(panel: Panel) -> &'static str {
    match panel {
        Panel::Basic => tr("Grundeinstellungen", "Basic"),
        Panel::Curve => tr("Gradationskurve", "Tone curve"),
        Panel::Detail => tr("Details", "Detail"),
        Panel::Effects => tr("Effekte", "Effects"),
        Panel::Calibration => tr("Kalibrierung", "Calibration"),
        Panel::Optics => tr("Optik", "Optics"),
    }
}

pub(super) fn white_balance_name(name: &str) -> &str {
    match name {
        "Auto" => tr("Automatisch", "Auto"),
        "Daylight" => tr("Tageslicht", "Daylight"),
        "Cloudy" => tr("Bewölkt", "Cloudy"),
        "Shade" => tr("Schatten", "Shade"),
        "Tungsten" => tr("Kunstlicht", "Tungsten"),
        "Fluorescent" => tr("Leuchtstoffröhre", "Fluorescent"),
        "Flash" => tr("Blitz", "Flash"),
        other => other,
    }
}

pub(super) fn mask_kind(kind: MaskKind) -> &'static str {
    match kind {
        MaskKind::Linear => tr("Linearer Verlauf", "Linear gradient"),
        MaskKind::Radial => tr("Radialverlauf", "Radial gradient"),
        MaskKind::Brush => tr("Pinsel", "Brush"),
        MaskKind::Ai => tr(
            "KI-Maske (z. B. Motiv, Himmel)",
            "AI mask (e.g. subject, sky)",
        ),
        MaskKind::Range => tr("Bereich", "Range"),
        MaskKind::Other => tr("Maske", "Mask"),
    }
}

/// Bezeichnung eines Reglers in Camera Raw.
pub(super) fn slider_label(key: &str) -> &'static str {
    match key {
        "Exposure2012" => tr("Belichtung", "Exposure"),
        "IncrementalTemperature" => tr("Temperatur", "Temperature"),
        "IncrementalTint" => tr("Tönung", "Tint"),
        "Contrast2012" => tr("Kontrast", "Contrast"),
        "Highlights2012" | "ParametricHighlights" => tr("Lichter", "Highlights"),
        "Shadows2012" | "ParametricShadows" => tr("Tiefen", "Shadows"),
        "Whites2012" => tr("Weiß", "Whites"),
        "Blacks2012" => tr("Schwarz", "Blacks"),
        "Texture" => tr("Struktur", "Texture"),
        "Clarity2012" => tr("Klarheit", "Clarity"),
        "Dehaze" => tr("Dunst entfernen", "Dehaze"),
        "Vibrance" => tr("Dynamik", "Vibrance"),
        "Saturation" => tr("Sättigung", "Saturation"),
        "ParametricLights" => tr("Helle Mitteltöne", "Lights"),
        "ParametricDarks" => tr("Dunkle Mitteltöne", "Darks"),
        "Sharpness" => tr("Schärfen", "Sharpening"),
        "SharpenRadius" => tr("Radius", "Radius"),
        "SharpenDetail" => tr("Details", "Detail"),
        "SharpenEdgeMasking" => tr("Maskieren", "Masking"),
        "LuminanceSmoothing" => tr("Rauschreduzierung", "Noise reduction"),
        "ColorNoiseReduction" => tr("Farbrauschen", "Color noise reduction"),
        "PostCropVignetteAmount" => tr("Vignette", "Vignette"),
        "PostCropVignetteMidpoint" => tr("Mittelpunkt", "Midpoint"),
        "PostCropVignetteRoundness" => tr("Rundheit", "Roundness"),
        "PostCropVignetteFeather" => tr("Weiche Kante", "Feather"),
        "PostCropVignetteHighlightContrast" => tr("Lichter", "Highlights"),
        "GrainAmount" => tr("Körnung", "Grain"),
        "GrainSize" => tr("Größe", "Size"),
        "GrainFrequency" => tr("Unregelmäßigkeit", "Roughness"),
        "ShadowTint" => tr("Tiefen – Tönung", "Shadows tint"),
        "RedHue" => tr("Rot – Farbton", "Red hue"),
        "RedSaturation" => tr("Rot – Sättigung", "Red saturation"),
        "GreenHue" => tr("Grün – Farbton", "Green hue"),
        "GreenSaturation" => tr("Grün – Sättigung", "Green saturation"),
        "BlueHue" => tr("Blau – Farbton", "Blue hue"),
        "BlueSaturation" => tr("Blau – Sättigung", "Blue saturation"),
        "VignetteAmount" => tr("Objektiv-Vignettierung", "Lens vignetting"),
        _ => tr("Regler", "Slider"),
    }
}

/// Bezeichnung eines lokalen Reglers (Schlüssel ohne `Local`).
pub(super) fn local_label(key: &str) -> &'static str {
    match key {
        "Exposure2012" | "Exposure" => tr("Belichtung", "Exposure"),
        "Contrast2012" | "Contrast" => tr("Kontrast", "Contrast"),
        "Highlights2012" => tr("Lichter", "Highlights"),
        "Shadows2012" => tr("Tiefen", "Shadows"),
        "Whites2012" => tr("Weiß", "Whites"),
        "Blacks2012" => tr("Schwarz", "Blacks"),
        "Clarity2012" | "Clarity" => tr("Klarheit", "Clarity"),
        "Texture" => tr("Struktur", "Texture"),
        "Dehaze" => tr("Dunst", "Dehaze"),
        "Saturation" => tr("Sättigung", "Saturation"),
        "Temperature" => tr("Temperatur", "Temperature"),
        "Tint" => tr("Tönung", "Tint"),
        "Sharpness" => tr("Schärfe", "Sharpness"),
        "LuminanceNoise" => tr("Rauschen", "Noise"),
        "Moire" => tr("Moiré", "Moiré"),
        "Defringe" => tr("Rand entfernen", "Defringe"),
        "Brightness" => tr("Helligkeit", "Brightness"),
        "ToningSaturation" => tr("Farbe", "Color"),
        "Hue" => tr("Farbton", "Hue"),
        "Grain" => tr("Körnung", "Grain"),
        _ => tr("weitere Regler", "other sliders"),
    }
}
