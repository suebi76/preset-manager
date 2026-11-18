//! Die Ordner, in denen Adobe Presets ablegt.
//!
//! Photoshop, Photoshop (Beta) und Lightroom Classic teilen sich einen einzigen
//! Camera-Raw-Ordner. Hintergrund: `Planung/02-Adobe-Grundlagen.md`.
//!
//! Dies ist die einzige Stelle im Projekt, die Adobe-Pfade kennt. Alle anderen
//! Module bekommen ein [`AdobeLocations`] übergeben. Dadurch können Tests mit
//! einem temporären Ordner arbeiten und fassen nie echte Presets an.

use std::path::{Component, Path, PathBuf, Prefix};

use crate::error::CoreError;

/// Die Preset-Ordner einer Camera-Raw-Installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdobeLocations {
    /// Presets, die der Nutzer selbst angelegt oder hineinkopiert hat.
    /// Hierhin installiert die App.
    pub settings: PathBuf,
    /// Presets, die über Adobes eigenen Importdialog kamen.
    /// Die App liest und verwaltet sie, installiert aber nicht hierhin.
    pub imported_settings: PathBuf,
    /// Kameraprofile (`.dcp`).
    pub camera_profiles: PathBuf,
}

impl AdobeLocations {
    /// Leitet die Unterordner aus einem Camera-Raw-Ordner ab.
    ///
    /// Prüft nicht, ob die Ordner existieren. Gedacht für Tests; im Programm
    /// [`AdobeLocations::resolve`] verwenden.
    pub fn from_camera_raw_dir(camera_raw_dir: &Path) -> Self {
        Self {
            settings: camera_raw_dir.join("Settings"),
            imported_settings: camera_raw_dir.join("ImportedSettings"),
            camera_profiles: camera_raw_dir.join("CameraProfiles"),
        }
    }

    /// Ermittelt die Ordner: den vom Nutzer gewählten Camera-Raw-Ordner, falls
    /// angegeben, sonst den automatisch gefundenen ([`AdobeLocations::detect`]).
    ///
    /// # Fehler
    ///
    /// - [`CoreError::AdobeFolderNotFound`], wenn der Ordner nicht existiert.
    /// - [`CoreError::NotACameraRawFolder`], wenn der gewählte Ordner keinen
    ///   der Unterordner `Settings`, `ImportedSettings` oder `CameraProfiles`
    ///   enthält – etwa weil `CameraRaw\Settings` statt `CameraRaw` gewählt
    ///   wurde.
    pub fn resolve(custom_camera_raw_dir: Option<&Path>) -> Result<Self, CoreError> {
        let Some(dir) = custom_camera_raw_dir else {
            return Self::detect();
        };

        if !dir.is_dir() {
            return Err(CoreError::AdobeFolderNotFound {
                searched: dir.to_path_buf(),
            });
        }

        let locations = Self::from_camera_raw_dir(dir);
        if locations
            .preset_folders()
            .iter()
            .any(|folder| folder.is_dir())
        {
            Ok(locations)
        } else {
            Err(CoreError::NotACameraRawFolder {
                path: dir.to_path_buf(),
            })
        }
    }

    /// Sucht den Camera-Raw-Ordner des angemeldeten Windows-Benutzers unter
    /// `%APPDATA%\Adobe\CameraRaw`.
    ///
    /// # Fehler
    ///
    /// [`CoreError::AdobeFolderNotFound`], wenn der Ordner nicht existiert –
    /// etwa weil Camera Raw noch nie gestartet wurde – oder `%APPDATA%` nicht
    /// gesetzt bzw. leer ist.
    pub fn detect() -> Result<Self, CoreError> {
        let app_data = std::env::var_os("APPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let Some(app_data) = app_data else {
            return Err(CoreError::AdobeFolderNotFound {
                searched: PathBuf::from(r"%APPDATA%\Adobe\CameraRaw"),
            });
        };
        let camera_raw_dir = app_data.join("Adobe").join("CameraRaw");

        if camera_raw_dir.is_dir() {
            Ok(Self::from_camera_raw_dir(&camera_raw_dir))
        } else {
            Err(CoreError::AdobeFolderNotFound {
                searched: camera_raw_dir,
            })
        }
    }

    /// Alle Preset-Ordner dieser Installation.
    pub fn preset_folders(&self) -> [&Path; 3] {
        [
            &self.settings,
            &self.imported_settings,
            &self.camera_profiles,
        ]
    }

    /// `true`, wenn `path` **innerhalb** eines der Preset-Ordner liegt – der
    /// Ordner selbst zählt nicht dazu.
    ///
    /// Schutz vor Datenverlust: verändernde Aktionen prüfen damit, dass sie
    /// nur Dateien und Unterordner in den Adobe-Ordnern anfassen, nie einen
    /// der Ordner selbst.
    ///
    /// Der Vergleich folgt den Windows-Regeln: Groß- und Kleinschreibung zählt
    /// nicht, und die Schreibweise `\\?\C:\…` (wie sie
    /// `std::fs::canonicalize` liefert) gilt als gleich mit `C:\…`. Pfade mit
    /// `..` werden immer abgelehnt.
    pub fn contains(&self, path: &Path) -> bool {
        let Some(candidate) = comparable_components(path) else {
            return false;
        };
        self.preset_folders().iter().any(|folder| {
            comparable_components(folder).is_some_and(|folder| {
                candidate.len() > folder.len() && candidate.starts_with(&folder)
            })
        })
    }

    /// Wie [`Self::contains`], aber als Fehler zum Weiterreichen.
    ///
    /// # Fehler
    ///
    /// [`CoreError::OutsideAdobeFolder`], wenn `path` nicht innerhalb der
    /// Preset-Ordner liegt.
    pub fn ensure_contains(&self, path: &Path) -> Result<(), CoreError> {
        if self.contains(path) {
            Ok(())
        } else {
            Err(CoreError::OutsideAdobeFolder {
                path: path.to_path_buf(),
            })
        }
    }

    /// Der Camera-Raw-Ordner selbst, in dem `Settings` usw. liegen.
    pub fn camera_raw_dir(&self) -> &Path {
        self.settings.parent().unwrap_or(&self.settings)
    }

    /// `path` relativ zum Camera-Raw-Ordner, z. B. `Settings\Paket\Warm.xmp` –
    /// so wird die Datei im Sicherungsordner abgelegt.
    ///
    /// # Fehler
    ///
    /// [`CoreError::OutsideAdobeFolder`], wenn `path` nicht darunter liegt.
    pub fn relative_path<'a>(&self, path: &'a Path) -> Result<&'a Path, CoreError> {
        path.strip_prefix(self.camera_raw_dir())
            .map_err(|_| CoreError::OutsideAdobeFolder {
                path: path.to_path_buf(),
            })
    }
}

/// `true`, wenn zwei Pfade nach Windows-Regeln dieselbe Datei bezeichnen:
/// Groß- und Kleinschreibung zählt nicht, `\\?\C:\…` ist gleich `C:\…`.
pub fn same_path(a: &Path, b: &Path) -> bool {
    match (comparable_components(a), comparable_components(b)) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}

/// Zerlegt einen Pfad in Bestandteile, die sich nach Windows-Regeln
/// vergleichen lassen. Gibt `None` zurück, wenn der Pfad `..` enthält.
fn comparable_components(path: &Path) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => parts.push(comparable_prefix(prefix.kind())),
            Component::RootDir => parts.push(String::from(r"\")),
            Component::CurDir => {}
            Component::ParentDir => return None,
            Component::Normal(part) => parts.push(part.to_string_lossy().to_lowercase()),
        }
    }
    Some(parts)
}

/// Bringt `C:` und `\\?\C:` (bzw. die UNC-Varianten) auf dieselbe Form.
fn comparable_prefix(prefix: Prefix) -> String {
    match prefix {
        Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
            format!("{}:", char::from(letter).to_ascii_lowercase())
        }
        Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => format!(
            r"\\{}\{}",
            server.to_string_lossy().to_lowercase(),
            share.to_string_lossy().to_lowercase()
        ),
        Prefix::Verbatim(part) | Prefix::DeviceNS(part) => part.to_string_lossy().to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leitet_unterordner_ab() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new("X:/CameraRaw"));

        assert_eq!(locations.settings, Path::new("X:/CameraRaw/Settings"));
        assert_eq!(
            locations.imported_settings,
            Path::new("X:/CameraRaw/ImportedSettings")
        );
        assert_eq!(
            locations.camera_profiles,
            Path::new("X:/CameraRaw/CameraProfiles")
        );
    }

    #[test]
    fn erkennt_pfade_innerhalb_und_ausserhalb() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new("X:/CameraRaw"));

        assert!(locations.contains(Path::new("X:/CameraRaw/Settings/Paket/a.xmp")));
        assert!(locations.contains(Path::new("X:/CameraRaw/Settings/Paket")));
        assert!(!locations.contains(Path::new("X:/CameraRaw/Logs/a.xmp")));
        assert!(!locations.contains(Path::new("X:/Dokumente/a.xmp")));
        assert!(!locations.contains(Path::new("X:/CameraRaw/SettingsAlt/a.xmp")));
    }

    #[test]
    fn preset_ordner_selbst_gehoeren_nicht_dazu() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new("X:/CameraRaw"));

        assert!(!locations.contains(Path::new("X:/CameraRaw/Settings")));
        assert!(!locations.contains(Path::new("X:/CameraRaw/Settings/")));
        assert!(!locations.contains(Path::new("X:/CameraRaw/ImportedSettings")));
        assert!(!locations.contains(Path::new("X:/CameraRaw")));
    }

    #[test]
    fn lehnt_pfade_mit_zurueckspruengen_ab() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new("X:/CameraRaw"));

        assert!(!locations.contains(Path::new("X:/CameraRaw/Settings/../../Windows/a.xmp")));
    }

    #[cfg(windows)]
    #[test]
    fn vergleicht_nach_windows_regeln() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new(r"C:\Users\A\CameraRaw"));

        assert!(locations.contains(Path::new(r"c:\users\a\cameraraw\settings\a.xmp")));
        assert!(locations.contains(Path::new(r"\\?\C:\Users\A\CameraRaw\Settings\a.xmp")));
    }

    #[test]
    fn gewaehlter_ordner_wird_verwendet() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Settings")).unwrap();

        let locations = AdobeLocations::resolve(Some(dir.path())).unwrap();

        assert_eq!(locations.settings, dir.path().join("Settings"));
    }

    #[test]
    fn fehlender_gewaehlter_ordner_ist_ein_fehler() {
        let result = AdobeLocations::resolve(Some(Path::new("X:/gibt/es/nicht")));

        assert!(matches!(result, Err(CoreError::AdobeFolderNotFound { .. })));
    }

    #[test]
    fn ordner_ohne_preset_unterordner_ist_kein_camera_raw_ordner() {
        let dir = tempfile::tempdir().unwrap();

        let result = AdobeLocations::resolve(Some(dir.path()));

        assert!(matches!(result, Err(CoreError::NotACameraRawFolder { .. })));
    }
}
