//! Installierte Presets verwalten: löschen, umbenennen, verschieben, sortieren,
//! aktivieren und deaktivieren.
//!
//! Jede Funktion arbeitet in derselben Reihenfolge (`Planung/04-Fehlerbehandlung.md`):
//!
//! 1. **Prüfen** – Namen gültig? Ziel frei? Datei innerhalb der Adobe-Ordner?
//!    Scheitert hier etwas, ist noch nichts verändert.
//! 2. **Sichern** – das Original kommt in den Sicherungsordner.
//! 3. **Ändern** – neuer Inhalt wird atomar geschrieben, gelöscht wird nur in
//!    den Papierkorb.
//!
//! Betrifft eine Aktion viele Dateien, laufen die übrigen weiter, wenn eine
//! scheitert; der [`OperationReport`] nennt jede einzelne. Danach ist die
//! [`Library`] veraltet und wird neu geladen ([`Library::load`]).
//!
//! Was „Gruppe“ und „Reihenfolge“ bedeuten, steht in den Dateien selbst
//! (`crs:Group`, `crs:SortName`), nicht im Ordner. Umbenennen einer Gruppe
//! oder Verschieben in eine andere Gruppe bewegt deshalb keine Dateien.

use std::path::{Path, PathBuf};

use crate::adobe::locations::same_path;
use crate::adobe::AdobeLocations;
use crate::error::{CoreError, NameProblem};
use crate::import::naming::safe_component;
use crate::library::scan::Library;
use crate::preset::model::{Preset, Section};
use crate::preset::xmp::read_metadata;
use crate::preset::xmp_edit::{rewrite_metadata, MetadataChanges};
use crate::report::{ItemOutcome, OperationReport};
use crate::safe_fs::{write_atomic, write_to_free_name, BackupSession, Trash, WriteMode};

/// Höchstlänge eines Preset- oder Gruppennamens in Zeichen.
const MAX_DISPLAY_NAME_CHARS: usize = 255;

/// Was eine verändernde Aktion braucht.
pub struct Context<'a> {
    /// Die Adobe-Ordner. Nur darin wird geändert.
    pub locations: &'a AdobeLocations,
    /// Der aktuelle Stand – für die Prüfung auf doppelte Namen.
    pub library: &'a Library,
    /// Wohin Originale vor dem Ändern gesichert werden.
    pub backups: &'a mut BackupSession,
}

// ---------------------------------------------------------------------------
// Löschen
// ---------------------------------------------------------------------------

/// Legt die Presets in den Papierkorb (F-22).
///
/// Gelöscht wird nur, was innerhalb der Adobe-Ordner liegt und beim Löschen
/// noch immer ein lesbares Preset ist. Wird ein Paketordner dadurch leer, wird
/// er entfernt.
pub fn delete(
    presets: &[&Preset],
    locations: &AdobeLocations,
    trash: &dyn Trash,
) -> OperationReport {
    let mut report = OperationReport::new();
    let mut seen: Vec<&Path> = Vec::new();
    for preset in presets {
        // Ist ein Preset doppelt ausgewählt, wird es nur einmal gelöscht.
        if seen.iter().any(|path| same_path(path, &preset.path)) {
            continue;
        }
        seen.push(&preset.path);
        let outcome = match delete_one(&preset.path, locations, trash) {
            Ok(()) => ItemOutcome::Done,
            Err(error) => ItemOutcome::Failed(error),
        };
        report.record(preset.path.clone(), outcome);
    }
    report
}

/// Legt alle Einträge einer Gruppe in den Papierkorb (F-23). Eine
/// gleichnamige Gruppe im anderen Bereich (Presets/Profile) bleibt.
///
/// # Fehler
///
/// [`CoreError::GroupNotFound`], wenn es die Gruppe nicht gibt.
pub fn delete_group(
    section: Section,
    group: &str,
    library: &Library,
    locations: &AdobeLocations,
    trash: &dyn Trash,
) -> Result<OperationReport, CoreError> {
    let presets: Vec<&Preset> = presets_of(library, section, group)?.iter().collect();
    Ok(delete(&presets, locations, trash))
}

fn delete_one(path: &Path, locations: &AdobeLocations, trash: &dyn Trash) -> Result<(), CoreError> {
    check_inside(path, locations)?;
    // Liegt dort inzwischen etwas anderes, wird es nicht angerührt.
    read_metadata(path)?;
    trash.move_to_trash(path)?;
    remove_empty_folders(path, locations);
    Ok(())
}

/// Entfernt leer gewordene Paketordner oberhalb von `path`, aber nie die
/// Adobe-Ordner selbst. Klappt das nicht (z. B. weil der Ordner doch nicht
/// leer ist), bleibt er einfach stehen.
fn remove_empty_folders(path: &Path, locations: &AdobeLocations) {
    let mut folder = path.parent();
    while let Some(current) = folder {
        if !locations.contains(current) {
            break;
        }
        // `remove_dir` entfernt nur leere Ordner – genau das Gewünschte.
        if std::fs::remove_dir(current).is_err() {
            break;
        }
        folder = current.parent();
    }
}

// ---------------------------------------------------------------------------
// Umbenennen
// ---------------------------------------------------------------------------

/// Gibt einem Preset einen neuen Namen (F-24): `crs:Name` in der Datei und
/// der Dateiname werden geändert. Gibt den neuen Pfad zurück.
///
/// # Fehler
///
/// - [`CoreError::InvalidName`]: leer, zu lang oder mit Steuerzeichen
/// - [`CoreError::NameConflict`]: in der Gruppe gibt es schon ein Preset
///   dieses Namens (ohne Rücksicht auf Groß-/Kleinschreibung)
/// - sonst wie beim Schreiben; in allen Fehlerfällen ist nichts verändert
pub fn rename_preset(
    preset: &Preset,
    new_name: &str,
    context: &mut Context<'_>,
) -> Result<PathBuf, CoreError> {
    let new_name = valid_name(new_name, false)?;
    check_inside(&preset.path, context.locations)?;
    let taken = context
        .library
        .group(preset.section(), &preset.group)
        .into_iter()
        .flat_map(|group| &group.presets)
        .any(|other| other.path != preset.path && same_name(&other.name, &new_name));
    if taken {
        return Err(CoreError::NameConflict { name: new_name });
    }

    let changes = MetadataChanges {
        name: Some(new_name.clone()),
        ..MetadataChanges::default()
    };
    let content = prepare(&preset.path, &changes, context)?;
    let new_path = new_file_path(&preset.path, &new_name);

    if new_path == preset.path || same_path(&new_path, &preset.path) {
        write_atomic(&preset.path, &content, WriteMode::Replace)?;
        if new_path != preset.path {
            // Nur Groß-/Kleinschreibung des Dateinamens ändert sich.
            std::fs::rename(&preset.path, &new_path).map_err(|source| CoreError::Io {
                path: preset.path.clone(),
                source,
            })?;
        }
        return Ok(new_path);
    }

    check_inside(&new_path, context.locations)?;
    let written = write_to_free_name(&new_path, &content)?;
    // Das Original ist gesichert; die alte Datei kann weg. Bliebe sie stehen,
    // zeigte Camera Raw das Preset doppelt.
    if let Err(source) = std::fs::remove_file(&preset.path) {
        // Die alte Datei lässt sich nicht entfernen (z. B. gesperrt). Dann
        // wird die neue wieder entfernt, damit kein halber Zustand bleibt.
        let _ = std::fs::remove_file(&written);
        return Err(CoreError::Io {
            path: preset.path.clone(),
            source,
        });
    }
    Ok(written)
}

/// Benennt eine Gruppe um (F-25): `crs:Group` wird in allen Einträgen des
/// Bereichs `section` geändert, deren Gruppe **genau** `old` heißt. Gruppen in
/// anderer Schreibweise und gleichnamige Gruppen im anderen Bereich bleiben
/// unberührt – Camera Raw behandelt sie ebenfalls als eigene Gruppen
/// (Versuch P-5).
///
/// # Fehler
///
/// Vor jeder Änderung: [`CoreError::InvalidName`],
/// [`CoreError::GroupNotFound`], oder [`CoreError::NameConflict`], wenn es die
/// neue Gruppe im selben Bereich schon gibt (zum Zusammenführen dient
/// [`move_to_group`]).
pub fn rename_group(
    section: Section,
    old: &str,
    new: &str,
    context: &mut Context<'_>,
) -> Result<OperationReport, CoreError> {
    let new = valid_name(new, false)?;
    let presets = presets_of(context.library, section, old)?;
    if new == old {
        // Nichts zu tun – und nichts wird unnötig neu geschrieben.
        return Ok(OperationReport::new());
    }
    if context.library.group(section, &new).is_some() {
        return Err(CoreError::NameConflict { name: new });
    }

    let changes = MetadataChanges {
        group: Some(new),
        ..MetadataChanges::default()
    };
    let mut report = OperationReport::new();
    for preset in presets {
        let outcome = rewrite_in_place(&preset.path, &changes, context);
        report.record(preset.path.clone(), outcome);
    }
    Ok(report)
}

// ---------------------------------------------------------------------------
// Verschieben
// ---------------------------------------------------------------------------

/// Verschiebt Einträge in eine andere Gruppe (F-26, F-34). Eine leere
/// `group` heißt „ohne Gruppe“. Die Gruppe darf neu sein (so entsteht eine
/// neue Gruppe, F-33) oder schon existieren.
///
/// Jeder Eintrag bleibt in seinem Bereich: Ein Profil landet in der
/// Profil-Gruppe dieses Namens, ein Preset in der Preset-Gruppe.
///
/// Gibt es in der Zielgruppe schon einen Eintrag desselben Namens, bleibt
/// dieser Eintrag stehen und erscheint mit [`CoreError::NameConflict`] im
/// Bericht.
///
/// # Fehler
///
/// [`CoreError::InvalidName`] für einen ungültigen Gruppennamen – dann ist
/// nichts verändert.
pub fn move_to_group(
    presets: &[&Preset],
    group: &str,
    context: &mut Context<'_>,
) -> Result<OperationReport, CoreError> {
    let group = valid_name(group, true)?;
    let mut names_in_target: Vec<(Section, String)> = [Section::Presets, Section::Profiles]
        .into_iter()
        .filter_map(|section| context.library.group(section, &group))
        .flat_map(|existing| &existing.presets)
        .filter(|existing| !presets.iter().any(|moved| moved.path == existing.path))
        .map(|existing| (existing.section(), existing.name.clone()))
        .collect();

    let mut report = OperationReport::new();
    for preset in presets {
        // Ein Sortierschlüssel gehört zur Reihenfolge der alten Gruppe. In der
        // neuen Gruppe würde er den Eintrag willkürlich nach vorn stellen –
        // deshalb wird er beim Verschieben geleert.
        let changes = MetadataChanges {
            group: Some(group.clone()),
            sort_name: (!preset.sort_name.is_empty()).then(String::new),
            ..MetadataChanges::default()
        };
        let outcome = if preset.group == group {
            ItemOutcome::Skipped {
                reason: "ist schon in dieser Gruppe".to_owned(),
            }
        } else if names_in_target
            .iter()
            .any(|(section, name)| *section == preset.section() && same_name(name, &preset.name))
        {
            ItemOutcome::Failed(CoreError::NameConflict {
                name: preset.name.clone(),
            })
        } else {
            let outcome = rewrite_in_place(&preset.path, &changes, context);
            if matches!(outcome, ItemOutcome::Done) {
                names_in_target.push((preset.section(), preset.name.clone()));
            }
            outcome
        };
        report.record(preset.path.clone(), outcome);
    }
    Ok(report)
}

// ---------------------------------------------------------------------------
// Sortieren
// ---------------------------------------------------------------------------

/// Legt die Reihenfolge fest (F-27): Das erste Preset bekommt den
/// Sortierschlüssel `0001`, das zweite `0002` usw. Camera Raw und Lightroom
/// sortieren danach (Versuch P-2).
///
/// Übergeben werden sollte die vollständige Gruppe in der gewünschten
/// Reihenfolge. Presets, deren Schlüssel schon stimmt, werden nicht
/// angefasst.
pub fn reorder(ordered: &[&Preset], context: &mut Context<'_>) -> OperationReport {
    let width = ordered.len().to_string().len().max(4);
    let mut report = OperationReport::new();
    for (index, preset) in ordered.iter().enumerate() {
        let sort_name = format!("{:0width$}", index + 1);
        let outcome = if preset.sort_name == sort_name {
            ItemOutcome::Skipped {
                reason: "Reihenfolge stimmt bereits".to_owned(),
            }
        } else {
            let changes = MetadataChanges {
                sort_name: Some(sort_name),
                ..MetadataChanges::default()
            };
            rewrite_in_place(&preset.path, &changes, context)
        };
        report.record(preset.path.clone(), outcome);
    }
    report
}

// ---------------------------------------------------------------------------
// Aktivieren und deaktivieren
// ---------------------------------------------------------------------------

/// Erklärt den Ordner der deaktivierten Presets, falls jemand ihn findet.
const INACTIVE_README: &str = "\
Preset Manager – deaktivierte Presets
=====================================

Die Presets und Profile in diesem Ordner sind im Preset Manager
deaktiviert. Photoshop und Lightroom Classic zeigen sie nicht an.
Zum Aktivieren in Preset Manager auswählen und „Aktivieren“ wählen –
sie kommen dann an genau dieselbe Stelle unter Settings bzw.
ImportedSettings zurück. Bitte die Dateien hier nicht von Hand
verschieben.

Preset Manager – deactivated presets
====================================

The presets and profiles in this folder are deactivated in Preset
Manager. Photoshop and Lightroom Classic do not show them. To
activate them, select them in Preset Manager and choose “Activate” –
they return to exactly the same place under Settings or
ImportedSettings. Please do not move the files here by hand.
";

/// Aktiviert oder deaktiviert Presets (F-48).
///
/// Deaktivieren verschiebt die Datei aus `Settings` (bzw. `ImportedSettings`)
/// in den Ordner der deaktivierten Presets – mit demselben Unterpfad.
/// Aktivieren legt sie genau dorthin zurück. Der Inhalt bleibt unverändert,
/// Favoriten in Camera Raw bleiben erhalten (Versuch P-7).
///
/// Liegt am Ziel schon eine Datei gleichen Namens, bleibt das Preset, wo es
/// ist, und erscheint mit [`CoreError::AlreadyExists`] im Bericht. Einträge,
/// die schon im gewünschten Zustand sind, werden übersprungen.
///
/// Gibt den Bericht (je alter Pfad) und die neuen Pfade der verschobenen
/// Presets zurück.
pub fn set_active(
    presets: &[&Preset],
    active: bool,
    locations: &AdobeLocations,
) -> (OperationReport, Vec<PathBuf>) {
    let mut report = OperationReport::new();
    let mut moved = Vec::new();
    let mut seen: Vec<&Path> = Vec::new();
    for preset in presets {
        if seen.iter().any(|path| same_path(path, &preset.path)) {
            continue;
        }
        seen.push(&preset.path);
        let outcome = if preset.active == active {
            ItemOutcome::Skipped {
                reason: if active {
                    "ist schon aktiv"
                } else {
                    "ist schon deaktiviert"
                }
                .to_owned(),
            }
        } else {
            match move_activation(&preset.path, locations) {
                Ok(target) => {
                    moved.push(target);
                    ItemOutcome::Done
                }
                Err(error) => ItemOutcome::Failed(error),
            }
        };
        report.record(preset.path.clone(), outcome);
    }
    (report, moved)
}

/// Verschiebt eine Datei in den Ordner des anderen Zustands.
fn move_activation(path: &Path, locations: &AdobeLocations) -> Result<PathBuf, CoreError> {
    check_inside(path, locations)?;
    let outside = || CoreError::OutsideAdobeFolder {
        path: path.to_path_buf(),
    };
    let (from, to, now_active) = locations.activation_pair(path).ok_or_else(outside)?;
    let relative = path.strip_prefix(from).map_err(|_| outside())?;
    let target = to.join(relative);
    check_inside(&target, locations)?;
    // Liegt dort inzwischen etwas anderes, wird es nicht angerührt.
    read_metadata(path)?;
    if target.exists() {
        return Err(CoreError::AlreadyExists { path: target });
    }
    let parent = target.parent().ok_or_else(outside)?;
    std::fs::create_dir_all(parent).map_err(|source| CoreError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    if now_active {
        write_inactive_readme(locations);
    }
    move_file(path, &target)?;
    remove_empty_folders(path, locations);
    Ok(target)
}

/// Verschiebt eine Datei. Klappt das Umbenennen nicht (anderes Laufwerk über
/// eine Verknüpfung), wird kopiert, geprüft und erst dann das Original
/// entfernt.
fn move_file(from: &Path, to: &Path) -> Result<(), CoreError> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| CoreError::Io { path, source }
    };
    let content = std::fs::read(from).map_err(io(from))?;
    write_atomic(to, &content, WriteMode::CreateNew)?;
    let copied = std::fs::read(to).map_err(io(to))?;
    if copied != content {
        let _ = std::fs::remove_file(to);
        return Err(CoreError::Io {
            path: to.to_path_buf(),
            source: std::io::Error::other("Kopie weicht vom Original ab"),
        });
    }
    if let Err(source) = std::fs::remove_file(from) {
        // Lieber gar nicht verschoben als doppelt vorhanden.
        let _ = std::fs::remove_file(to);
        return Err(CoreError::Io {
            path: from.to_path_buf(),
            source,
        });
    }
    Ok(())
}

/// Legt die Erklärung in den Ordner der deaktivierten Presets, falls sie
/// fehlt. Scheitert das, ist es kein Fehler.
fn write_inactive_readme(locations: &AdobeLocations) {
    if let Some(root) = locations.inactive_settings.parent() {
        let readme = root.join("LIES MICH - README.txt");
        if !readme.exists() {
            let _ = std::fs::write(readme, INACTIVE_README);
        }
    }
}

// ---------------------------------------------------------------------------
// Gemeinsame Bausteine
// ---------------------------------------------------------------------------

/// Ändert eine Datei an Ort und Stelle: prüfen, sichern, atomar schreiben.
fn rewrite_in_place(
    path: &Path,
    changes: &MetadataChanges,
    context: &mut Context<'_>,
) -> ItemOutcome {
    let result = check_inside(path, context.locations)
        .and_then(|()| prepare(path, changes, context))
        .and_then(|content| write_atomic(path, &content, WriteMode::Replace));
    match result {
        Ok(()) => ItemOutcome::Done,
        Err(error) => ItemOutcome::Failed(error),
    }
}

/// Liest die Datei, berechnet den neuen Inhalt und sichert das Original.
/// Geschrieben wird noch nichts.
fn prepare(
    path: &Path,
    changes: &MetadataChanges,
    context: &mut Context<'_>,
) -> Result<Vec<u8>, CoreError> {
    let original = std::fs::read(path).map_err(|source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let content = rewrite_metadata(&original, changes, path)?;
    let relative = context.locations.relative_path(path)?;
    context.backups.back_up(path, relative)?;
    Ok(content)
}

/// Der Dateiname zum neuen Anzeigenamen, im selben Ordner wie bisher.
fn new_file_path(current: &Path, new_name: &str) -> PathBuf {
    let file_name = format!("{}.xmp", safe_component(new_name, "Preset"));
    current.with_file_name(file_name)
}

/// Prüft und bereinigt einen Anzeigenamen: Leerzeichen am Rand werden
/// entfernt; leer ist er nur erlaubt, wenn `allow_empty` gesetzt ist.
pub(crate) fn valid_name(name: &str, allow_empty: bool) -> Result<String, CoreError> {
    let trimmed = name.trim();
    let invalid = |problem| CoreError::InvalidName {
        name: name.to_owned(),
        problem,
    };
    if trimmed.is_empty() && !allow_empty {
        return Err(invalid(NameProblem::Empty));
    }
    if trimmed.chars().count() > MAX_DISPLAY_NAME_CHARS {
        return Err(invalid(NameProblem::TooLong {
            max: MAX_DISPLAY_NAME_CHARS,
        }));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(invalid(NameProblem::ControlCharacters));
    }
    Ok(trimmed.to_owned())
}

/// Die Einträge einer Gruppe – genau dieser Schreibweise, in diesem Bereich.
fn presets_of<'a>(
    library: &'a Library,
    section: Section,
    group: &str,
) -> Result<&'a [Preset], CoreError> {
    library
        .group(section, group)
        .map(|found| found.presets.as_slice())
        .ok_or_else(|| CoreError::GroupNotFound {
            name: group.to_owned(),
        })
}

/// Letzte Sicherung gegen Programmierfehler: geändert wird nur innerhalb der
/// Adobe-Ordner.
fn check_inside(path: &Path, locations: &AdobeLocations) -> Result<(), CoreError> {
    locations.ensure_contains(path)
}

/// Zwei Namen gelten als gleich, wenn sie sich nur in Groß-/Kleinschreibung
/// unterscheiden – sonst wären sie in der Liste kaum zu unterscheiden.
fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prueft_namen() {
        assert_eq!(valid_name("  Warm  ", false).unwrap(), "Warm");
        assert!(valid_name("   ", false).is_err());
        assert_eq!(valid_name("", true).unwrap(), "");
        assert!(valid_name("a\tb", false).is_err());
        assert!(valid_name(&"x".repeat(256), false).is_err());
    }

    #[test]
    fn leitet_dateinamen_vom_anzeigenamen_ab() {
        let current = Path::new("C:/CR/Settings/Paket/alt.xmp");
        assert_eq!(
            new_file_path(current, "Neu: Warm?"),
            Path::new("C:/CR/Settings/Paket/Neu_ Warm_.xmp")
        );
    }

    #[test]
    fn vergleicht_namen_ohne_schreibweise() {
        assert!(same_name("Warm", "WARM"));
        assert!(same_name(
            "Größe",
            "GRÖSSE".to_lowercase().replace("ss", "ß").as_str()
        ));
        assert!(!same_name("Warm", "Warm 2"));
    }
}
