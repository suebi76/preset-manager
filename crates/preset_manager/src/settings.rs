//! Einstellungen der App unter `%APPDATA%\PresetManager`.
//!
//! Zwei Dateien, beide bewusst schlicht, damit man sie zur Not von Hand lesen
//! und ändern kann:
//!
//! - `einstellungen.txt` – was der Nutzer im Einstellungsdialog festlegt
//!   ([`Settings`]), eine Zeile `schlüssel=wert` je Einstellung.
//! - `leere-gruppen.txt` – die **leeren Gruppen** (F-33). Camera Raw kennt
//!   keine leeren Gruppen – eine Gruppe ist nur ein Eintrag in den
//!   Preset-Dateien. Legt der Nutzer eine neue Gruppe an, merkt sich die App
//!   sie hier, bis etwas hineingezogen wurde. Eine Zeile je Gruppe: `P`
//!   (Presets) oder `R` (Profile), ein Tabulator, der Name.
//!
//! Fehlt eine Datei oder ist sie kaputt, gelten die Voreinstellungen – die App
//! startet immer.

use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use preset_core::preset::Section;
use preset_core::safe_fs::{write_atomic, WriteMode};

const EMPTY_GROUPS_FILE: &str = "leere-gruppen.txt";
const SETTINGS_FILE: &str = "einstellungen.txt";

/// Erlaubte Vergrößerung der Oberfläche (50 % bis 200 %) – auch für Werte,
/// die mit Strg+Plus/Minus entstanden sind.
pub const ZOOM_RANGE: RangeInclusive<f32> = 0.5..=2.0;
/// Die Stufen im Einstellungsdialog.
pub const ZOOM_STEPS: [f32; 8] = [0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];

/// Was der Nutzer im Einstellungsdialog festlegt.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Selbst gewählter Camera-Raw-Ordner; `None` heißt „automatisch suchen“.
    pub camera_raw_dir: Option<PathBuf>,
    /// Vergrößerung der Oberfläche, 1.0 = 100 %.
    pub zoom: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            camera_raw_dir: None,
            zoom: 1.0,
        }
    }
}

/// Der Ordner für Einstellungen; `None`, wenn `%APPDATA%` fehlt.
fn settings_dir() -> Option<PathBuf> {
    let app_data = std::env::var_os("APPDATA").filter(|value| !value.is_empty())?;
    Some(PathBuf::from(app_data).join("PresetManager"))
}

fn read_file(name: &str) -> Option<String> {
    settings_dir().and_then(|dir| std::fs::read_to_string(dir.join(name)).ok())
}

/// Schreibt eine Einstellungsdatei atomar. Scheitert das, geht nur diese
/// Einstellung verloren – deshalb wird der Fehler als Text zurückgegeben.
fn write_file(name: &str, contents: &str) -> Result<(), String> {
    let dir = settings_dir().ok_or("%APPDATA% ist nicht gesetzt")?;
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    write_atomic(&dir.join(name), contents.as_bytes(), WriteMode::Replace)
        .map_err(|error| error.to_string())
}

/// Liest die Einstellungen. Fehlt die Datei, gelten die Voreinstellungen.
pub fn load_settings() -> Settings {
    read_file(SETTINGS_FILE)
        .map(|text| parse_settings(&text))
        .unwrap_or_default()
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    write_file(SETTINGS_FILE, &format_settings(settings))
}

/// Liest die gemerkten leeren Gruppen. Fehlt die Datei, ist die Liste leer.
pub fn load_empty_groups() -> Vec<(Section, String)> {
    read_file(EMPTY_GROUPS_FILE)
        .map(|text| parse(&text))
        .unwrap_or_default()
}

pub fn save_empty_groups(groups: &[(Section, String)]) -> Result<(), String> {
    write_file(EMPTY_GROUPS_FILE, &format(groups))
}

/// Unbekannte Schlüssel und unlesbare Werte werden übergangen, damit eine
/// Datei einer neueren Version die App nicht am Start hindert.
fn parse_settings(text: &str) -> Settings {
    let mut settings = Settings::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "camera_raw_dir" if !value.is_empty() => {
                settings.camera_raw_dir = Some(PathBuf::from(value));
            }
            "zoom" => {
                if let Ok(zoom) = value.parse::<f32>() {
                    if zoom.is_finite() {
                        settings.zoom = zoom.clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
                    }
                }
            }
            _ => {}
        }
    }
    settings
}

fn format_settings(settings: &Settings) -> String {
    let dir = settings
        .camera_raw_dir
        .as_deref()
        .map(Path::to_string_lossy)
        .unwrap_or_default();
    format!(
        "# Preset Manager – Einstellungen\n\
         # camera_raw_dir leer = automatisch suchen\n\
         camera_raw_dir={dir}\n\
         zoom={:.2}\n",
        settings.zoom
    )
}

fn parse(text: &str) -> Vec<(Section, String)> {
    text.lines()
        .filter_map(|line| {
            let (kind, name) = line.split_once('\t')?;
            let section = match kind {
                "P" => Section::Presets,
                "R" => Section::Profiles,
                _ => return None,
            };
            let name = name.trim();
            (!name.is_empty()).then(|| (section, name.to_owned()))
        })
        .collect()
}

fn format(groups: &[(Section, String)]) -> String {
    groups
        .iter()
        .map(|(section, name)| {
            let kind = match section {
                Section::Presets => "P",
                Section::Profiles => "R",
            };
            format!("{kind}\t{name}\n")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schreibt_und_liest_dieselben_gruppen() {
        let groups = vec![
            (Section::Presets, "Sommer 2026".to_owned()),
            (Section::Profiles, "Film & Korn".to_owned()),
        ];

        assert_eq!(parse(&format(&groups)), groups);
    }

    #[test]
    fn einstellungen_ueberstehen_speichern_und_lesen() {
        let settings = Settings {
            camera_raw_dir: Some(PathBuf::from(r"D:\Adobe Test\CameraRaw")),
            zoom: 1.25,
        };

        let text = format_settings(&settings);
        assert!(text.lines().all(|line| !line.starts_with(' ')), "{text}");
        assert_eq!(parse_settings(&text), settings);
        assert_eq!(
            parse_settings(&format_settings(&Settings::default())),
            Settings::default()
        );
    }

    #[test]
    fn kaputte_einstellungen_ergeben_voreinstellungen() {
        assert_eq!(parse_settings(""), Settings::default());
        assert_eq!(
            parse_settings("zoom=riesig\nunbekannt=1\nohne Gleichheitszeichen\nzoom=NaN"),
            Settings::default()
        );
        assert_eq!(parse_settings("zoom=9").zoom, *ZOOM_RANGE.end());
    }

    #[test]
    fn uebergeht_kaputte_zeilen() {
        let text = "P\tGut\nX\tUnbekannt\nohne Tabulator\nR\t   \nR\tAuch gut\n";

        assert_eq!(
            parse(text),
            [
                (Section::Presets, "Gut".to_owned()),
                (Section::Profiles, "Auch gut".to_owned())
            ]
        );
    }
}
