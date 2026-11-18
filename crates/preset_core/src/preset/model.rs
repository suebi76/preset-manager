//! Datentypen für Presets.

use std::path::{Path, PathBuf};

/// Die Dateiarten, die die App kennt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresetKind {
    /// Camera-Raw-Preset (`.xmp`). Wird von Photoshop und Lightroom Classic gelesen.
    CameraRawPreset,
    /// Kameraprofil (`.dcp`).
    CameraProfile,
    /// Altes Lightroom-Preset (`.lrtemplate`).
    LegacyLightroomPreset,
}

impl PresetKind {
    /// Bestimmt die Art anhand der Dateiendung. Groß-/Kleinschreibung ist egal.
    ///
    /// Gibt `None` zurück, wenn die Endung unbekannt ist. Eine `.xmp`-Datei
    /// kann auch ein Foto-Sidecar sein; das entscheidet erst der Blick in die
    /// Datei.
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "xmp" => Some(Self::CameraRawPreset),
            "dcp" => Some(Self::CameraProfile),
            "lrtemplate" => Some(Self::LegacyLightroomPreset),
            _ => None,
        }
    }
}

/// Was eine `.xmp`-Datei laut `crs:PresetType` ist.
///
/// Camera Raw zeigt beide Arten an verschiedenen Stellen: Presets in der
/// Preset-Liste, Profile im Profilbrowser.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PresetType {
    /// `Normal`: ein gewöhnliches Preset mit Reglereinstellungen.
    Preset,
    /// `Look`: ein kreatives Profil.
    Profile,
    /// Ein anderer Wert, den die App nicht kennt. Wird unverändert angezeigt.
    Other(String),
}

impl PresetType {
    /// Übersetzt den Wert von `crs:PresetType`.
    pub fn from_xmp_value(value: &str) -> Self {
        match value {
            "Normal" => Self::Preset,
            "Look" => Self::Profile,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl PresetType {
    /// Der Bereich, in dem Camera Raw diese Art zeigt. Unbekannte Arten
    /// zählen zu den Presets.
    pub fn section(&self) -> Section {
        match self {
            Self::Profile => Section::Profiles,
            Self::Preset | Self::Other(_) => Section::Presets,
        }
    }
}

/// Wo Camera Raw einen Eintrag zeigt. Presets und Profile haben getrennte
/// Gruppen: Eine Preset-Gruppe „Sommer“ und eine Profil-Gruppe „Sommer“ sind
/// zwei verschiedene Gruppen an zwei verschiedenen Stellen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Section {
    /// Die Vorgabenliste („Vorgaben“).
    Presets,
    /// Der Profilbrowser („Bearbeiten → Profil → Durchsuchen“).
    Profiles,
}

/// Aus welchem Adobe-Ordner ein installiertes Preset stammt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresetOrigin {
    /// `CameraRaw\Settings` – selbst angelegt oder mit dieser App installiert.
    Settings,
    /// `CameraRaw\ImportedSettings` – über Adobes Importdialog installiert.
    ImportedSettings,
}

/// Ein installiertes Preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    /// Wo die Datei liegt.
    pub path: PathBuf,
    /// Aus welchem Adobe-Ordner die Datei stammt.
    pub origin: PresetOrigin,
    /// Ob die Datei ein Preset oder ein Profil ist.
    pub preset_type: PresetType,
    /// Der Name, den Camera Raw anzeigt (`crs:Name`).
    pub name: String,
    /// Die Gruppe, unter der Camera Raw das Preset einsortiert (`crs:Group`).
    /// Leer, wenn das Preset keiner Gruppe angehört.
    pub group: String,
    /// Eindeutige Kennung (`crs:UUID`), falls vorhanden.
    pub uuid: Option<String>,
    /// Sortierschlüssel (`crs:SortName`). Leer bedeutet: nach Name sortieren.
    pub sort_name: String,
}

impl Preset {
    /// Der Bereich, in dem Camera Raw den Eintrag zeigt.
    pub fn section(&self) -> Section {
        self.preset_type.section()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erkennt_art_an_der_endung() {
        assert_eq!(
            PresetKind::from_path(Path::new("a.xmp")),
            Some(PresetKind::CameraRawPreset)
        );
        assert_eq!(
            PresetKind::from_path(Path::new("A.XMP")),
            Some(PresetKind::CameraRawPreset)
        );
        assert_eq!(
            PresetKind::from_path(Path::new("b.dcp")),
            Some(PresetKind::CameraProfile)
        );
        assert_eq!(
            PresetKind::from_path(Path::new("c.lrtemplate")),
            Some(PresetKind::LegacyLightroomPreset)
        );
    }

    #[test]
    fn unbekannte_endung_ergibt_none() {
        assert_eq!(PresetKind::from_path(Path::new("bild.jpg")), None);
        assert_eq!(PresetKind::from_path(Path::new("ohne_endung")), None);
    }

    #[test]
    fn profile_haben_einen_eigenen_bereich() {
        assert_eq!(PresetType::Preset.section(), Section::Presets);
        assert_eq!(PresetType::Profile.section(), Section::Profiles);
        assert_eq!(
            PresetType::Other("Neu".to_owned()).section(),
            Section::Presets
        );
    }

    #[test]
    fn uebersetzt_preset_typ() {
        assert_eq!(PresetType::from_xmp_value("Normal"), PresetType::Preset);
        assert_eq!(PresetType::from_xmp_value("Look"), PresetType::Profile);
        assert_eq!(
            PresetType::from_xmp_value("Neu"),
            PresetType::Other("Neu".to_owned())
        );
    }
}
