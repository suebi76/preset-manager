//! Suchwörter für die Wirkung eines Eintrags (F-49) – auf Deutsch und
//! Englisch zugleich, unabhängig von der Sprache der Oberfläche: Wer
//! „warm“ oder „grain“ sucht, findet beides.
//!
//! Die Wörter kommen aus den Aussagen von
//! [`preset_core::preset::PresetSettings::facts`] und dem Namen des Profils,
//! auf dem der Eintrag aufbaut.

use preset_core::preset::adjustments::{ColorChange, Fact, HslColor};

use crate::worker::PresetInfo;

/// Alle Suchwörter eines Eintrags, kleingeschrieben und durch Leerzeichen
/// getrennt.
pub fn effect_words(info: &PresetInfo) -> String {
    let mut words: Vec<&str> = Vec::new();
    let mut extra: Vec<String> = Vec::new();
    for fact in &info.summary.facts {
        match fact {
            Fact::Color { color, change } => {
                words.extend(color_words(*color));
                words.extend(change_words(*change));
            }
            Fact::Toned { hue, .. } => {
                words.extend(["getönt", "tönung", "toned", "toning", "color grading"]);
                words.extend(color_words(hue_color(*hue)));
            }
            Fact::Profile(name) => {
                words.extend(["profil", "profile"]);
                extra.push(name.to_lowercase());
            }
            other => words.extend(fact_words(other)),
        }
    }
    if let Some((profile, _)) = &info.profile {
        extra.push(profile.name.to_lowercase());
    }
    let mut text = words.join(" ");
    for name in extra {
        text.push(' ');
        text.push_str(&name);
    }
    text
}

/// Trifft die Suche den Eintrag? Jedes Wort der Suche muss im Namen oder in
/// den Suchwörtern vorkommen.
pub fn matches(query: &str, name: &str, words: &str) -> bool {
    let name = name.to_lowercase();
    let query = query.trim().to_lowercase();
    if name.contains(&query) {
        return true;
    }
    let mut terms = query.split_whitespace().peekable();
    terms.peek().is_some()
        && terms
            .all(|term| name.contains(term) || (term.chars().count() >= 3 && words.contains(term)))
}

fn fact_words(fact: &Fact) -> &'static [&'static str] {
    match fact {
        Fact::BlackAndWhite => &[
            "schwarzweiß",
            "schwarz-weiß",
            "schwarzweiss",
            "s/w",
            "monochrom",
            "black & white",
            "black and white",
            "b&w",
            "monochrome",
            "grayscale",
        ],
        Fact::Brighter => &["heller", "hell", "brighter", "bright", "light"],
        Fact::Darker => &["dunkler", "dunkel", "darker", "dark", "moody"],
        Fact::MoreContrast => &["kontrast", "kontrastreich", "contrast", "punchy"],
        Fact::LessContrast => &["weniger kontrast", "flach", "low contrast", "flat"],
        Fact::FadedBlacks => &[
            "matt",
            "matte",
            "faded",
            "ausgeblichen",
            "verblasst",
            "film",
        ],
        Fact::MutedWhites => &["gedämpftes weiß", "muted whites"],
        Fact::BrighterShadows => &["tiefen aufgehellt", "lifted shadows"],
        Fact::DarkerShadows => &["tiefe schatten", "deep shadows"],
        Fact::SofterHighlights => &["lichter", "highlights"],
        Fact::Warmer => &["warm", "wärmer", "warmer", "golden"],
        Fact::Cooler => &["kühl", "kühler", "kalt", "cool", "cooler", "cold"],
        Fact::Magenta => &["magenta", "pink"],
        Fact::Greener => &["grünstich", "grün", "green tint", "green"],
        Fact::MoreColorful => &[
            "kräftig",
            "bunt",
            "satt",
            "vivid",
            "vibrant",
            "saturated",
            "colorful",
        ],
        Fact::LessColorful => &[
            "gedämpft",
            "entsättigt",
            "pastell",
            "muted",
            "desaturated",
            "pastel",
        ],
        Fact::MoreTexture => &[
            "struktur", "klarheit", "scharf", "texture", "clarity", "crisp",
        ],
        Fact::Softer => &["weich", "soft", "softer"],
        Fact::LessHaze => &["dunst entfernt", "klar", "dehaze", "clear"],
        Fact::MoreHaze => &["dunst", "dunstig", "verträumt", "haze", "hazy", "dreamy"],
        Fact::DarkVignette => &["vignette", "dunkle ränder", "dark edges"],
        Fact::LightVignette => &["vignette", "helle ränder", "light edges"],
        Fact::Grain => &["filmkorn", "korn", "körnung", "grain", "film grain", "film"],
        Fact::ColorTable => &["lut", "farbtabelle", "color table"],
        Fact::LocalAdjustments(_) => &["maske", "masken", "lokal", "mask", "masks", "local"],
        Fact::Color { .. } | Fact::Toned { .. } | Fact::Profile(_) => &[],
    }
}

fn color_words(color: HslColor) -> &'static [&'static str] {
    match color {
        HslColor::Red => &["rot", "red"],
        HslColor::Orange => &["orange", "hauttöne", "skin"],
        HslColor::Yellow => &["gelb", "yellow"],
        HslColor::Green => &["grün", "green"],
        HslColor::Aqua => &["aquamarin", "türkis", "aqua", "teal"],
        HslColor::Blue => &["blau", "blue"],
        HslColor::Purple => &["lila", "violett", "purple"],
        HslColor::Magenta => &["magenta", "pink"],
    }
}

fn change_words(change: ColorChange) -> &'static [&'static str] {
    match change {
        ColorChange::Saturated => &["kräftiger", "saturated"],
        ColorChange::Desaturated => &["entsättigt", "desaturated"],
        ColorChange::Lighter => &["heller", "lighter"],
        ColorChange::Darker => &["dunkler", "darker"],
        ColorChange::Shifted => &["verschoben", "shifted"],
    }
}

/// Die Farbe des Mischers, die einem Farbton am nächsten liegt.
fn hue_color(hue: f32) -> HslColor {
    let hue = hue.rem_euclid(360.0);
    HslColor::ALL
        .into_iter()
        .min_by(|a, b| {
            let distance = |color: &HslColor| {
                let d = (color.hue_degrees() - hue).abs();
                d.min(360.0 - d)
            };
            distance(a).total_cmp(&distance(b))
        })
        .unwrap_or(HslColor::Red)
}

#[cfg(test)]
mod tests {
    use preset_core::library::ProfileStatus;
    use preset_core::preset::{PresetSummary, ProfileRef};

    use super::*;

    fn info(facts: Vec<Fact>) -> PresetInfo {
        PresetInfo {
            summary: PresetSummary {
                facts,
                needs_profile: None,
            },
            profile: None,
        }
    }

    #[test]
    fn findet_wirkung_in_beiden_sprachen() {
        let words = effect_words(&info(vec![Fact::Warmer, Fact::Grain]));
        assert!(matches("warm", "Sommer", &words));
        assert!(matches("wärmer", "Sommer", &words));
        assert!(matches("grain", "Sommer", &words));
        assert!(matches("Filmkorn", "Sommer", &words));
        assert!(!matches("schwarzweiß", "Sommer", &words));
    }

    #[test]
    fn mehrere_woerter_muessen_alle_passen() {
        let words = effect_words(&info(vec![Fact::BlackAndWhite, Fact::Grain]));
        assert!(matches("schwarzweiß korn", "Klassik", &words));
        assert!(!matches("schwarzweiß warm", "Klassik", &words));
        // Name und Wirkung gemischt.
        assert!(matches("klassik korn", "Klassik 2", &words));
    }

    #[test]
    fn findet_masken_und_profil() {
        let mut found = info(vec![Fact::LocalAdjustments(2)]);
        found.profile = Some((
            ProfileRef {
                name: "Kodak Gold".to_owned(),
                uuid: None,
            },
            ProfileStatus::Missing,
        ));
        let words = effect_words(&found);
        assert!(matches("maske", "X", &words));
        assert!(matches("mask", "X", &words));
        assert!(matches("kodak", "X", &words));
    }

    #[test]
    fn kurze_woerter_nur_im_namen() {
        let words = effect_words(&info(vec![Fact::Warmer]));
        assert!(!matches("wa", "Sommer", &words));
        assert!(matches("so", "Sommer", &words));
    }

    #[test]
    fn farbton_wird_zur_naechsten_farbe() {
        assert_eq!(hue_color(220.0), HslColor::Blue);
        assert_eq!(hue_color(355.0), HslColor::Red);
    }
}
