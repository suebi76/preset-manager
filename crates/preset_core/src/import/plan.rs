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

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::adobe::locations::same_path;
use crate::adobe::AdobeLocations;
use crate::error::CoreError;
use crate::import::naming::{numbered, safe_component};
use crate::import::source::{read_limited, FoundPreset, ScannedSource};
use crate::library::ops::valid_name;
use crate::library::Library;
use crate::preset::model::Section;
use crate::preset::xmp::{metadata_from_bytes, PresetMetadata};
use crate::preset::xmp_edit::{replace_uuid, rewrite_metadata, MetadataChanges};
use crate::report::{ItemOutcome, OperationReport};
use crate::safe_fs::{write_atomic, write_to_free_name, BackupSession, WriteMode};

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

    /// Führt den Plan aus.
    ///
    /// Scheitert ein Preset, laufen die übrigen weiter. Der Bericht enthält
    /// für jeden Eintrag den tatsächlichen Zielpfad und das Ergebnis.
    /// Ersetzte Dateien liegen danach in `backups` (siehe
    /// [`BackupSession::folder`]).
    pub fn execute(&self, policy: ConflictPolicy, backups: &mut BackupSession) -> OperationReport {
        let mut report = OperationReport::new();
        // Kennungen, deren Preset jetzt im Zielordner liegt (neu installiert
        // oder schon vorhanden). Nur dann darf eine Dublette entfallen.
        let mut present_uuids: HashSet<&str> = HashSet::new();

        for item in &self.items {
            let uuid = item.preset.metadata.uuid.as_deref();
            let (path, outcome) = match &item.action {
                _ if !item.selected => (
                    item.preset.shown_as.clone(),
                    ItemOutcome::Skipped {
                        reason: deselected_reason(item),
                    },
                ),
                PlannedAction::DuplicateInImport { first }
                    if uuid.is_some_and(|uuid| present_uuids.contains(uuid)) =>
                {
                    (
                        item.preset.shown_as.clone(),
                        ItemOutcome::Skipped {
                            reason: format!(
                                "gleiches Preset (gleiche UUID) wie {}",
                                first.display()
                            ),
                        },
                    )
                }
                // Ist die erste Fundstelle gescheitert, wird stattdessen
                // diese Kopie installiert – sonst fehlte das Preset ganz.
                _ => match self.install(item, policy, backups) {
                    Ok(result) => result,
                    Err(error) => (item.target.clone(), ItemOutcome::Failed(error)),
                },
            };
            if !matches!(outcome, ItemOutcome::Failed(_)) {
                present_uuids.extend(uuid);
            }
            report.record(path, outcome);
        }
        report
    }

    /// Installiert ein Preset und gibt den tatsächlichen Zielpfad zurück.
    fn install(
        &self,
        item: &PlannedImport,
        policy: ConflictPolicy,
        backups: &mut BackupSession,
    ) -> Result<(PathBuf, ItemOutcome), CoreError> {
        let target = &item.target;
        self.check_target(target)?;

        // Die Datei wird unmittelbar vor dem Schreiben noch einmal geprüft.
        // So kann nie etwas anderes als ein lesbares Preset installiert werden.
        let content = read_limited(&item.preset.file, &item.preset.shown_as)?;
        let metadata = metadata_from_bytes(&content, &item.preset.shown_as)?;

        // Zielgruppe gewählt? Dann bekommt die Kopie diese Gruppe. Die
        // Quelldatei selbst bleibt unverändert.
        let group = self.group_after_import(item);
        let content = if group == metadata.group {
            content
        } else {
            let changes = MetadataChanges {
                group: Some(group.to_owned()),
                ..MetadataChanges::default()
            };
            rewrite_metadata(&content, &changes, &item.preset.shown_as)?
        };

        if let Some(folder) = target.parent() {
            std::fs::create_dir_all(folder).map_err(|source| CoreError::Io {
                path: folder.to_path_buf(),
                source,
            })?;
        }

        match write_atomic(target, &content, WriteMode::CreateNew) {
            Ok(()) => return Ok((target.clone(), ItemOutcome::Done)),
            Err(CoreError::AlreadyExists { .. }) => {}
            Err(error) => return Err(error),
        }

        match policy {
            ConflictPolicy::Skip => Ok((
                target.clone(),
                ItemOutcome::Skipped {
                    reason: "eine Datei mit diesem Namen ist schon installiert".to_owned(),
                },
            )),
            ConflictPolicy::Replace => {
                let relative = self.locations.relative_path(target)?;
                backups.back_up(target, relative)?;
                write_atomic(target, &content, WriteMode::Replace)?;
                Ok((target.clone(), ItemOutcome::Done))
            }
            ConflictPolicy::KeepBoth => {
                let written = self.install_copy(target, &content, &item.preset.shown_as)?;
                Ok((written, ItemOutcome::Done))
            }
        }
    }

    /// „Beide behalten“: schreibt das Preset mit neuer Kennung unter den
    /// ersten freien nummerierten Namen.
    fn install_copy(
        &self,
        target: &Path,
        content: &[u8],
        shown_as: &Path,
    ) -> Result<PathBuf, CoreError> {
        let new_uuid = uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .to_ascii_uppercase();
        let content = replace_uuid(content, &new_uuid, shown_as)?;
        check_uuid(&content, shown_as, &new_uuid)?;

        // `target` ist belegt; die Varianten liegen im selben, schon
        // geprüften Ordner.
        write_to_free_name(target, &content)
    }

    /// Letzte Sicherung gegen Programmierfehler: Der Import schreibt nur
    /// innerhalb von `Settings` (Entscheidung E-4).
    fn check_target(&self, target: &Path) -> Result<(), CoreError> {
        if !target.starts_with(&self.locations.settings) {
            return Err(CoreError::OutsideAdobeFolder {
                path: target.to_path_buf(),
            });
        }
        self.locations.ensure_contains(target)
    }
}

/// Begründung im Bericht für einen abgewählten Eintrag.
fn deselected_reason(item: &PlannedImport) -> String {
    match item.installed_with_same_uuid.first() {
        Some(other) => format!(
            "abgewählt, gleiches Preset schon installiert: {}",
            other.display()
        ),
        None => "abgewählt".to_owned(),
    }
}

/// Prüft, dass die geänderte Datei lesbar ist und die neue Kennung trägt.
fn check_uuid(content: &[u8], shown_as: &Path, expected: &str) -> Result<(), CoreError> {
    let metadata: PresetMetadata = metadata_from_bytes(content, shown_as)?;
    match metadata.uuid.as_deref() {
        Some(uuid) if uuid == expected => Ok(()),
        // Eine Datei ohne Kennung bleibt ohne Kennung – das ist in Ordnung.
        None => Ok(()),
        Some(_) => Err(CoreError::MalformedXmp {
            path: shown_as.to_path_buf(),
            reason: "neue UUID wurde nicht übernommen".to_owned(),
        }),
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
