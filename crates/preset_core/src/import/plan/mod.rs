//! Importplan erstellen und ausführen.
//!
//! Der Import läuft in zwei Schritten (`Planung/03-Architektur.md`):
//!
//! 1. [`ImportPlan::new`] **liest nur**. Der Plan sagt für jedes gefundene
//!    Preset, wohin es kopiert wird und ob es dort schon eine Datei gibt.
//!    Daraus baut die Oberfläche die Vorschau (F-07).
//! 2. [`ImportPlan::execute`] kopiert und hält im [`OperationReport`] fest,
//!    was mit jeder Datei passiert ist.
//!
//! # Wohin wird installiert?
//!
//! Jedes Paket bekommt einen eigenen Unterordner in `CameraRaw\Settings`,
//! benannt nach dem Ordner bzw. Archiv (`Settings\Sommerlicht Presets\`). Camera
//! Raw und Lightroom Classic lesen Unterordner mit (Versuch P-1). Innerhalb
//! des Pakets liegen alle Presets flach nebeneinander; die Gruppe, unter der
//! Camera Raw sie zeigt, steht ohnehin in der Datei.
//!
//! # Konflikte
//!
//! Gibt es die Zieldatei schon, entscheidet die [`ConflictPolicy`]:
//! überspringen (Standard), ersetzen (vorher sichern) oder beide behalten
//! (neuer Dateiname, neue Kennung). Ob eine Datei existiert, wird beim
//! Ausführen noch einmal geprüft – zwischen Vorschau und Klick auf
//! „Installieren“ kann sich der Ordner geändert haben.
//!
//! # Schon anderswo installiert
//!
//! Ist ein Preset mit derselben Kennung bereits an anderer Stelle installiert
//! – meist über Adobes Importdialog in `ImportedSettings` –, würde Camera Raw
//! es nach dem Import doppelt zeigen. Solche Einträge sind im Plan deshalb
//! **abgewählt** ([`PlannedImport::selected`]). Der Nutzer kann sie in der
//! Vorschau mit [`ImportPlan::set_selected`] wieder auswählen.
//!
//! | Datei | Inhalt |
//! |---|---|
//! | `execute` | Plan ausführen |

mod execute;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::adobe::locations::same_path;
use crate::adobe::AdobeLocations;
use crate::error::CoreError;
use crate::import::naming::{numbered, safe_component};
use crate::import::source::{FoundPreset, ScannedSource};
use crate::library::ops::valid_name;
use crate::library::Library;
use crate::preset::model::Section;

/// Was geschieht, wenn es die Zieldatei schon gibt (F-08).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConflictPolicy {
    /// Die vorhandene Datei bleibt, das neue Preset wird nicht installiert.
    #[default]
    Skip,
    /// Die vorhandene Datei wird gesichert und dann ersetzt.
    Replace,
    /// Das neue Preset wird zusätzlich installiert, unter einem nummerierten
    /// Dateinamen (`Warm (2).xmp`) und mit neuer Kennung (`crs:UUID`).
    KeepBoth,
}

/// Was der Plan für ein einzelnes Preset vorsieht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannedAction {
    /// Die Zieldatei gibt es noch nicht – einfach installieren.
    Install,
    /// Die Zieldatei gibt es schon. Was passiert, entscheidet die
    /// [`ConflictPolicy`].
    Conflict,
    /// Dasselbe Preset (gleiche Kennung) kommt in diesem Import schon einmal
    /// vor, z. B. weil ein Paket je einen Ordner für Lightroom und Camera Raw
    /// mitbringt. Es wird nur einmal installiert.
    DuplicateInImport {
        /// Wo das Preset zuerst gefunden wurde.
        first: PathBuf,
    },
}

/// Ein Eintrag im Plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedImport {
    /// Das gefundene Preset.
    pub preset: FoundPreset,
    /// Wohin es kopiert wird.
    pub target: PathBuf,
    /// Was geplant ist.
    pub action: PlannedAction,
    /// Bereits installierte Presets mit derselben Kennung, z. B. weil das
    /// Paket schon einmal über Adobes Importdialog installiert wurde. Der
    /// Import ändert an diesen Dateien nichts.
    pub installed_with_same_uuid: Vec<PathBuf>,
    /// Ob der Eintrag installiert wird. Anfangs `false` genau dann, wenn
    /// `installed_with_same_uuid` nicht leer ist; änderbar über
    /// [`ImportPlan::set_selected`].
    pub selected: bool,
}

/// Zahlen eines Plans für die Vorschau.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanSummary {
    /// Presets, deren Zieldatei es noch nicht gibt.
    pub new_files: usize,
    /// Presets, deren Zieldatei es schon gibt.
    pub conflicts: usize,
    /// Presets, die im Import doppelt vorkommen und nur einmal installiert
    /// werden.
    pub duplicates_in_import: usize,
    /// Presets, deren Kennung schon an anderer Stelle installiert ist.
    pub already_installed_elsewhere: usize,
    /// Abgewählte Einträge – sie werden nicht installiert.
    pub deselected: usize,
}

/// Der vollständige Plan für einen Import aus einer oder mehreren Quellen.
#[derive(Debug)]
pub struct ImportPlan {
    locations: AdobeLocations,
    items: Vec<PlannedImport>,
    /// Die Quellen ohne ihre Presets (die stehen in `items`). Sie werden
    /// gehalten, damit entpackte ZIP-Dateien bis zum Ausführen erhalten bleiben,
    /// und für ihre Berichte.
    sources: Vec<ScannedSource>,
    /// Gruppe, in die alle Einträge eines Bereichs kommen, statt in die
    /// Gruppe aus der Datei (F-12: Ordner/ZIP auf eine Gruppe gezogen).
    target_group: Option<(Section, String)>,
}

impl ImportPlan {
    /// Erstellt den Plan. Liest nur, verändert nichts.
    ///
    /// `installed` ist die aktuell installierte Bibliothek; aus ihr stammen
    /// die Hinweise auf gleiche Kennungen.
    pub fn new(
        mut sources: Vec<ScannedSource>,
        locations: &AdobeLocations,
        installed: &Library,
    ) -> Self {
        let installed_by_uuid = index_by_uuid(installed);
        let mut planner = Planner {
            reserved_targets: HashSet::new(),
            first_by_uuid: HashMap::new(),
        };

        let mut items = Vec::new();
        for source in &mut sources {
            let package_folder = locations.settings.join(&source.package_name);
            for preset in std::mem::take(&mut source.presets) {
                let (target, action) = planner.place(&package_folder, &preset);
                let installed_with_same_uuid: Vec<PathBuf> = preset
                    .metadata
                    .uuid
                    .as_ref()
                    .and_then(|uuid| installed_by_uuid.get(uuid.as_str()))
                    .map(|paths| {
                        paths
                            .iter()
                            .filter(|path| !same_path(path, &target))
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                let selected = installed_with_same_uuid.is_empty();
                items.push(PlannedImport {
                    preset,
                    target,
                    action,
                    installed_with_same_uuid,
                    selected,
                });
            }
        }

        Self {
            locations: locations.clone(),
            items,
            sources,
            target_group: None,
        }
    }

    /// Legt fest, dass alle Einträge des Bereichs `section` in die Gruppe
    /// `group` installiert werden – statt in die Gruppe, die in ihrer Datei
    /// steht (F-12). Einträge des anderen Bereichs behalten ihre Gruppe.
    ///
    /// # Fehler
    ///
    /// [`CoreError::InvalidName`] für einen leeren Namen, Steuerzeichen oder
    /// mehr als 255 Zeichen.
    pub fn set_target_group(&mut self, section: Section, group: &str) -> Result<(), CoreError> {
        let group = valid_name(group, false)?;
        self.target_group = Some((section, group));
        Ok(())
    }

    /// Die Gruppe, in der ein Eintrag nach dem Import erscheint.
    pub fn group_after_import<'a>(&'a self, item: &'a PlannedImport) -> &'a str {
        match &self.target_group {
            Some((section, group)) if *section == item.preset.metadata.preset_type.section() => {
                group
            }
            _ => &item.preset.metadata.group,
        }
    }

    /// Alle Einträge in der Reihenfolge, in der sie ausgeführt werden.
    pub fn items(&self) -> &[PlannedImport] {
        &self.items
    }

    /// Wählt den Eintrag mit der Nummer `index` (wie in [`Self::items`]) für
    /// die Installation aus oder ab. Gibt `false` zurück, wenn es den Eintrag
    /// nicht gibt.
    pub fn set_selected(&mut self, index: usize, selected: bool) -> bool {
        match self.items.get_mut(index) {
            Some(item) => {
                item.selected = selected;
                true
            }
            None => false,
        }
    }

    /// Die durchsuchten Quellen mit ihren Berichten über übersprungene und
    /// defekte Dateien.
    pub fn sources(&self) -> &[ScannedSource] {
        &self.sources
    }

    /// Zahlen für die Vorschau.
    pub fn summary(&self) -> PlanSummary {
        let mut summary = PlanSummary::default();
        for item in &self.items {
            match item.action {
                PlannedAction::Install => summary.new_files += 1,
                PlannedAction::Conflict => summary.conflicts += 1,
                PlannedAction::DuplicateInImport { .. } => summary.duplicates_in_import += 1,
            }
            if !item.installed_with_same_uuid.is_empty() {
                summary.already_installed_elsewhere += 1;
            }
            if !item.selected {
                summary.deselected += 1;
            }
        }
        summary
    }
}

/// Vergibt die Zielpfade während der Planung.
struct Planner {
    /// Bereits vergebene Ziele (klein geschrieben, weil Windows bei
    /// Dateinamen nicht zwischen Groß und Klein unterscheidet).
    reserved_targets: HashSet<String>,
    /// Kennung → wo das Preset zuerst gefunden wurde.
    first_by_uuid: HashMap<String, PathBuf>,
}

impl Planner {
    fn place(&mut self, package_folder: &Path, preset: &FoundPreset) -> (PathBuf, PlannedAction) {
        let file_name = preset
            .shown_as
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let file_name = safe_component(&file_name, "Preset.xmp");

        if let Some(uuid) = &preset.metadata.uuid {
            if let Some(first) = self.first_by_uuid.get(uuid) {
                let action = PlannedAction::DuplicateInImport {
                    first: first.clone(),
                };
                return (package_folder.join(file_name), action);
            }
            self.first_by_uuid
                .insert(uuid.clone(), preset.shown_as.clone());
        }

        let target = self.reserve(package_folder, &file_name);
        let action = if target.exists() {
            PlannedAction::Conflict
        } else {
            PlannedAction::Install
        };
        (target, action)
    }

    /// Haben zwei Presets im selben Paket denselben Dateinamen (aus
    /// verschiedenen Unterordnern), bekommt das zweite eine Nummer.
    fn reserve(&mut self, package_folder: &Path, file_name: &str) -> PathBuf {
        let mut candidate = package_folder.join(file_name);
        let mut number = 2;
        while !self.reserved_targets.insert(lowercase(&candidate)) {
            candidate = package_folder.join(numbered(file_name, number));
            number += 1;
        }
        candidate
    }
}

fn index_by_uuid(library: &Library) -> HashMap<&str, Vec<PathBuf>> {
    let mut index: HashMap<&str, Vec<PathBuf>> = HashMap::new();
    for preset in library.groups().iter().flat_map(|group| &group.presets) {
        if let Some(uuid) = &preset.uuid {
            index
                .entry(uuid.as_str())
                .or_default()
                .push(preset.path.clone());
        }
    }
    index
}

fn lowercase(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}
