//! Installierte Presets einlesen und nach Gruppen ordnen.
//!
//! Camera Raw gruppiert Presets nach dem Feld `crs:Group` in der Datei, nicht
//! nach Ordnern. Zwei Presets aus verschiedenen Ordnern mit derselben Gruppe
//! erscheinen deshalb in derselben Gruppe – genau so ordnet auch
//! [`Library::load`] sie ein.
//!
//! Dieses Modul liest nur. Es verändert keine Datei.

use std::collections::BTreeMap;
use std::path::Path;

use walkdir::WalkDir;

use crate::adobe::AdobeLocations;
use crate::error::CoreError;
use crate::preset::model::{Preset, PresetKind, PresetOrigin, Section};
use crate::preset::xmp::read_metadata;
use crate::report::{ItemOutcome, OperationReport};

/// Eine Gruppe, wie Camera Raw sie anzeigt – in der Vorgabenliste oder im
/// Profilbrowser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetGroup {
    /// Wo die Gruppe erscheint: bei den Presets oder bei den Profilen.
    pub section: Section,
    /// Name der Gruppe (`crs:Group`). Leer für Presets ohne Gruppe.
    pub name: String,
    /// Die Presets der Gruppe in Anzeigereihenfolge.
    pub presets: Vec<Preset>,
}

/// Alle installierten Presets, nach Gruppen geordnet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Library {
    groups: Vec<PresetGroup>,
}

impl Library {
    /// Liest alle Presets aus `Settings` und `ImportedSettings`.
    ///
    /// Scheitert nie als Ganzes: Dateien, die nicht gelesen werden können,
    /// stehen mit Grund im zurückgegebenen [`OperationReport`], alle übrigen
    /// sind in der [`Library`]. Fehlt einer der Ordner, gilt er als leer –
    /// Camera Raw legt ihn erst an, wenn er gebraucht wird.
    pub fn load(locations: &AdobeLocations) -> (Self, OperationReport) {
        let mut report = OperationReport::new();
        let mut presets = Vec::new();

        let folders = [
            (&locations.settings, PresetOrigin::Settings),
            (&locations.imported_settings, PresetOrigin::ImportedSettings),
        ];
        for (folder, origin) in folders {
            presets.extend(load_folder(folder, origin, &mut report));
        }

        (Self::from_presets(presets), report)
    }

    /// Ordnet eine Liste von Presets nach Gruppen und sortiert beides.
    ///
    /// Erst kommen die Preset-Gruppen, dann die Profil-Gruppen; innerhalb
    /// davon alphabetisch nach [`collation_key`]. Presets innerhalb einer
    /// Gruppe stehen nach [`display_sort_key`].
    ///
    /// Gruppen, die sich nur in der Schreibweise unterscheiden („Leica“ und
    /// „LEICA“), bleiben getrennt – so zeigt es auch Camera Raw (Versuch P-5
    /// in `Planung/02-Adobe-Grundlagen.md`). Ebenso getrennt sind eine
    /// Preset- und eine Profil-Gruppe gleichen Namens.
    pub fn from_presets(presets: Vec<Preset>) -> Self {
        let mut by_group: BTreeMap<(Section, String), Vec<Preset>> = BTreeMap::new();
        for preset in presets {
            by_group
                .entry((preset.section(), preset.group.clone()))
                .or_default()
                .push(preset);
        }

        let mut groups: Vec<PresetGroup> = by_group
            .into_iter()
            .map(|((section, name), mut presets)| {
                presets.sort_by_cached_key(display_sort_key);
                PresetGroup {
                    section,
                    name,
                    presets,
                }
            })
            .collect();
        groups.sort_by_cached_key(|group| {
            (
                group.section,
                collation_key(&group.name),
                group.name.clone(),
            )
        });

        Self { groups }
    }

    /// Alle Gruppen in Anzeigereihenfolge.
    pub fn groups(&self) -> &[PresetGroup] {
        &self.groups
    }

    /// Die Gruppen eines Bereichs in Anzeigereihenfolge.
    pub fn groups_in(&self, section: Section) -> impl Iterator<Item = &PresetGroup> {
        self.groups
            .iter()
            .filter(move |group| group.section == section)
    }

    /// Sucht eine Gruppe über Bereich und genauen Namen.
    pub fn group(&self, section: Section, name: &str) -> Option<&PresetGroup> {
        self.groups
            .iter()
            .find(|group| group.section == section && group.name == name)
    }

    /// Gesamtzahl der Presets über alle Gruppen.
    pub fn preset_count(&self) -> usize {
        self.groups.iter().map(|group| group.presets.len()).sum()
    }
}

/// Der Schlüssel, nach dem Presets innerhalb einer Gruppe sortiert werden.
///
/// Ist `crs:SortName` gesetzt, zählt er, sonst der Name. Verglichen wird
/// über [`collation_key`]; bei Gleichstand entscheidet der Pfad, damit die
/// Reihenfolge bei jedem Einlesen dieselbe ist.
///
/// Camera Raw und Lightroom Classic sortieren ebenfalls nach `crs:SortName`
/// (Versuch P-2 in `Planung/02-Adobe-Grundlagen.md`). Ob Adobe dabei Groß-/Kleinschreibung und
/// Umlaute genauso behandelt, ist nicht geprüft.
pub fn display_sort_key(preset: &Preset) -> (String, String, std::path::PathBuf) {
    let primary = if preset.sort_name.is_empty() {
        &preset.name
    } else {
        &preset.sort_name
    };
    (
        collation_key(primary),
        collation_key(&preset.name),
        preset.path.clone(),
    )
}

/// Vergleichsschlüssel für die alphabetische Sortierung nach deutscher
/// Lexikon-Regel (DIN 5007-1): Groß- und Kleinschreibung zählt nicht, Umlaute
/// und Akzente zählen wie der Grundbuchstabe, ß wie „ss“.
///
/// Ohne diese Umwandlung stünde „Ölfarben“ hinter „Zebra“, weil `ö` im
/// Zeichensatz hinter `z` liegt.
pub fn collation_key(text: &str) -> String {
    let mut key = String::with_capacity(text.len());
    for character in text.chars().flat_map(char::to_lowercase) {
        match character {
            'ä' | 'à' | 'á' | 'â' | 'ã' | 'å' => key.push('a'),
            'é' | 'è' | 'ê' | 'ë' => key.push('e'),
            'í' | 'ì' | 'î' | 'ï' => key.push('i'),
            'ö' | 'ò' | 'ó' | 'ô' | 'õ' | 'ø' => key.push('o'),
            'ü' | 'ù' | 'ú' | 'û' => key.push('u'),
            'ç' => key.push('c'),
            'ñ' => key.push('n'),
            'ß' => key.push_str("ss"),
            other => key.push(other),
        }
    }
    key
}

/// `true` für Dateien und Ordner, die macOS beim Packen anlegt
/// (`__MACOSX`, `._Name`). Sie enthalten keine Presets.
///
/// Wird auch beim Import benutzt.
pub(crate) fn is_macos_metadata(name: &str) -> bool {
    name == "__MACOSX" || name.starts_with("._")
}

/// Liest alle `.xmp`-Dateien eines Ordners einschließlich Unterordnern.
///
/// Verknüpfungen (symbolische Links) werden verfolgt; Schleifen erkennt
/// `walkdir` selbst und meldet sie als Fehler.
fn load_folder(folder: &Path, origin: PresetOrigin, report: &mut OperationReport) -> Vec<Preset> {
    if !folder.is_dir() {
        return Vec::new();
    }

    let walker = WalkDir::new(folder)
        .follow_links(true)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| !is_macos_metadata(&entry.file_name().to_string_lossy()));

    let mut presets = Vec::new();
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                let path = error.path().unwrap_or(folder).to_path_buf();
                let source = error
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("Ordner nicht lesbar"));
                report.record(
                    path.clone(),
                    ItemOutcome::Failed(CoreError::Io { path, source }),
                );
                continue;
            }
        };

        let path = entry.path();
        let is_xmp = PresetKind::from_path(path) == Some(PresetKind::CameraRawPreset);
        if !entry.file_type().is_file() || !is_xmp {
            continue;
        }

        match read_metadata(path) {
            Ok(metadata) => {
                presets.push(Preset {
                    path: path.to_path_buf(),
                    origin,
                    preset_type: metadata.preset_type,
                    name: metadata.name,
                    group: metadata.group,
                    uuid: metadata.uuid,
                    sort_name: metadata.sort_name,
                });
                report.record(path.to_path_buf(), ItemOutcome::Done);
            }
            Err(CoreError::NotAPreset { .. }) => report.record(
                path.to_path_buf(),
                ItemOutcome::Skipped {
                    reason: "keine Preset-Datei (z. B. Sidecar eines Fotos)".to_owned(),
                },
            ),
            Err(error) => report.record(path.to_path_buf(), ItemOutcome::Failed(error)),
        }
    }
    presets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::model::PresetType;
    use std::path::PathBuf;

    fn preset(group: &str, name: &str, sort_name: &str) -> Preset {
        Preset {
            path: PathBuf::from(format!("{group}/{name}.xmp")),
            origin: PresetOrigin::Settings,
            preset_type: PresetType::Preset,
            name: name.to_owned(),
            group: group.to_owned(),
            uuid: None,
            sort_name: sort_name.to_owned(),
        }
    }

    fn names(group: &PresetGroup) -> Vec<&str> {
        group.presets.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn ordnet_nach_gruppen_ohne_ruecksicht_auf_schreibweise() {
        let library = Library::from_presets(vec![
            preset("zebra", "a", ""),
            preset("Apfel", "b", ""),
            preset("birne", "c", ""),
            preset("Zebra", "d", ""),
        ]);

        let group_names: Vec<&str> = library.groups().iter().map(|g| g.name.as_str()).collect();
        assert_eq!(group_names, ["Apfel", "birne", "Zebra", "zebra"]);
        assert_eq!(library.preset_count(), 4);
    }

    #[test]
    fn sortiert_nach_sortname_sonst_nach_name() {
        let library = Library::from_presets(vec![
            preset("G", "Charlie", ""),
            preset("G", "alpha", ""),
            preset("G", "Zulu", "0"),
            preset("G", "Bravo", ""),
        ]);

        let group = library.group(Section::Presets, "G").unwrap();
        assert_eq!(names(group), ["Zulu", "alpha", "Bravo", "Charlie"]);
    }

    #[test]
    fn trennt_presets_und_profile_gleichen_gruppennamens() {
        let mut profile = preset("Sommer", "Film", "");
        profile.preset_type = PresetType::Profile;
        let library = Library::from_presets(vec![
            profile,
            preset("Sommer", "Warm", ""),
            preset("Winter", "Kalt", ""),
        ]);

        let order: Vec<(Section, &str)> = library
            .groups()
            .iter()
            .map(|g| (g.section, g.name.as_str()))
            .collect();
        assert_eq!(
            order,
            [
                (Section::Presets, "Sommer"),
                (Section::Presets, "Winter"),
                (Section::Profiles, "Sommer"),
            ]
        );
        let profiles = library.group(Section::Profiles, "Sommer").unwrap();
        assert_eq!(names(profiles), ["Film"]);
        assert_eq!(library.groups_in(Section::Presets).count(), 2);
    }

    #[test]
    fn sortiert_umlaute_wie_grundbuchstaben() {
        let library = Library::from_presets(vec![
            preset("Zebra", "a", ""),
            preset("Ölfarben", "b", ""),
            preset("Apfel", "c", ""),
            preset("Straße", "d", ""),
            preset("Strasse 2", "e", ""),
        ]);

        let group_names: Vec<&str> = library.groups().iter().map(|g| g.name.as_str()).collect();
        assert_eq!(
            group_names,
            ["Apfel", "Ölfarben", "Straße", "Strasse 2", "Zebra"]
        );
    }

    #[test]
    fn erkennt_macos_hilfsdateien() {
        assert!(is_macos_metadata("__MACOSX"));
        assert!(is_macos_metadata("._Warm.xmp"));
        assert!(!is_macos_metadata("Warm.xmp"));
        assert!(!is_macos_metadata("_Warm.xmp"));
    }

    #[test]
    fn fehlende_ordner_ergeben_leere_bibliothek() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new("X:/gibt/es/nicht"));

        let (library, report) = Library::load(&locations);

        assert_eq!(library.preset_count(), 0);
        assert!(report.items().is_empty());
    }
}
