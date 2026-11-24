//! Welche Profile es gibt – für den Hinweis „Profil fehlt“ (F-50).
//!
//! Ein Preset kann auf ein Profil verweisen, ohne dessen Farbtabelle selbst
//! mitzubringen ([`crate::preset::PresetSummary::needs_profile`]). Camera Raw
//! findet das Profil dann über die Kennung – unter den Profilen des Nutzers
//! oder unter denen, die Adobe mitliefert:
//!
//! - `%ProgramData%\Adobe\CameraRaw\Settings` (Camera Raw in Photoshop)
//! - `%ProgramFiles%\Adobe\Adobe Lightroom Classic\Resources\Settings`
//!
//! Fehlt es überall, zeigen Photoshop und Lightroom das Preset ohne den
//! Look des Profils.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::preset::{read_metadata, PresetType, ProfileRef};

/// Kennungen und Namen bekannter Profile.
#[derive(Debug, Clone, Default)]
pub struct ProfileIndex {
    uuids: HashSet<String>,
    /// Kleingeschrieben – für Verweise ohne Kennung.
    names: HashSet<String>,
}

impl ProfileIndex {
    /// Nimmt ein Profil auf.
    pub fn add(&mut self, name: &str, uuid: Option<&str>) {
        if let Some(uuid) = uuid.filter(|uuid| !uuid.is_empty()) {
            self.uuids.insert(uuid.to_ascii_uppercase());
        }
        if !name.is_empty() {
            self.names.insert(name.to_lowercase());
        }
    }

    /// Ist das Profil bekannt? Mit Kennung zählt nur sie, ohne nur der Name.
    pub fn contains(&self, profile: &ProfileRef) -> bool {
        match profile.uuid.as_deref().filter(|uuid| !uuid.is_empty()) {
            Some(uuid) => self.uuids.contains(&uuid.to_ascii_uppercase()),
            None => self.names.contains(&profile.name.to_lowercase()),
        }
    }

    /// Wie viele Profile mit Kennung bekannt sind.
    pub fn len(&self) -> usize {
        self.uuids.len()
    }

    /// `true`, wenn kein Profil bekannt ist.
    pub fn is_empty(&self) -> bool {
        self.uuids.is_empty() && self.names.is_empty()
    }

    /// Liest alle Profile (`.xmp` mit `crs:PresetType="Look"`) unter den
    /// Ordnern. Unlesbare Dateien werden übergangen.
    pub fn scan(dirs: &[PathBuf]) -> Self {
        let mut index = Self::default();
        for dir in dirs {
            for entry in WalkDir::new(dir).into_iter().flatten() {
                let path = entry.path();
                let is_xmp = path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("xmp"));
                if !entry.file_type().is_file() || !is_xmp {
                    continue;
                }
                if let Ok(metadata) = read_metadata(path) {
                    if metadata.preset_type == PresetType::Profile {
                        index.add(&metadata.name, metadata.uuid.as_deref());
                    }
                }
            }
        }
        index
    }
}

/// Wo Adobe seine eigenen Profile ablegt – nur Ordner, die es gibt.
pub fn adobe_profile_dirs() -> Vec<PathBuf> {
    let variable = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    let mut dirs = Vec::new();
    if let Some(program_data) = variable("ProgramData") {
        dirs.push(Path::new(&program_data).join(r"Adobe\CameraRaw\Settings"));
    }
    if let Some(program_files) = variable("ProgramFiles") {
        dirs.push(
            Path::new(&program_files).join(r"Adobe\Adobe Lightroom Classic\Resources\Settings"),
        );
    }
    dirs.into_iter().filter(|dir| dir.is_dir()).collect()
}

/// Steht das Profil, auf das ein Preset verweist, zur Verfügung?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileStatus {
    /// Installiert – beim Nutzer oder von Adobe mitgeliefert.
    Installed,
    /// Nur deaktiviert vorhanden: Photoshop und Lightroom finden es nicht.
    Inactive,
    /// Nirgends gefunden.
    Missing,
}

/// Bestimmt den Stand eines Profils aus den bekannten Profilen.
pub fn profile_status(
    profile: &ProfileRef,
    available: &ProfileIndex,
    inactive: &ProfileIndex,
) -> ProfileStatus {
    if available.contains(profile) {
        ProfileStatus::Installed
    } else if inactive.contains(profile) {
        ProfileStatus::Inactive
    } else {
        ProfileStatus::Missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str, uuid: Option<&str>) -> ProfileRef {
        ProfileRef {
            name: name.to_owned(),
            uuid: uuid.map(str::to_owned),
        }
    }

    #[test]
    fn kennung_entscheidet_vor_dem_namen() {
        let mut index = ProfileIndex::default();
        index.add("Film warm", Some("abc"));
        assert!(index.contains(&profile("anderer Name", Some("ABC"))));
        assert!(!index.contains(&profile("Film warm", Some("XYZ"))));
        assert!(index.contains(&profile("FILM WARM", None)));
    }

    #[test]
    fn stand_eines_profils() {
        let mut available = ProfileIndex::default();
        available.add("Da", Some("1"));
        let mut inactive = ProfileIndex::default();
        inactive.add("Aus", Some("2"));
        assert_eq!(
            profile_status(&profile("Da", Some("1")), &available, &inactive),
            ProfileStatus::Installed
        );
        assert_eq!(
            profile_status(&profile("Aus", Some("2")), &available, &inactive),
            ProfileStatus::Inactive
        );
        assert_eq!(
            profile_status(&profile("Weg", Some("3")), &available, &inactive),
            ProfileStatus::Missing
        );
    }

    #[test]
    fn liest_profile_aus_ordnern() {
        let dir = tempfile::tempdir().unwrap();
        let file = |name: &str, kind: &str, uuid: &str| {
            std::fs::write(
                dir.path().join(format!("{name}.xmp")),
                format!(
                    r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:PresetType="{kind}" crs:UUID="{uuid}" crs:Name="{name}"/></rdf:RDF></x:xmpmeta>"#
                ),
            )
            .unwrap();
        };
        file("Profil", "Look", "P1");
        file("Preset", "Normal", "N1");
        let index = ProfileIndex::scan(&[dir.path().to_path_buf()]);
        assert!(index.contains(&profile("", Some("P1"))));
        assert!(!index.contains(&profile("", Some("N1"))));
        assert_eq!(index.len(), 1);
    }
}
