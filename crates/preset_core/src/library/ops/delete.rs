//! Löschen (F-22, F-23): in den Papierkorb, nie endgültig. Leer gewordene
//! Paketordner werden entfernt.

use std::path::Path;

use crate::adobe::locations::same_path;
use crate::adobe::AdobeLocations;
use crate::error::CoreError;
use crate::library::scan::Library;
use crate::preset::model::{Preset, Section};
use crate::preset::xmp::read_metadata;
use crate::report::{ItemOutcome, OperationReport};
use crate::safe_fs::{BackupSession, Change, Trash};

use super::common::{check_inside, presets_of, read_file};

/// Legt die Presets in den Papierkorb (F-22).
///
/// Gelöscht wird nur, was innerhalb der Adobe-Ordner liegt und beim Löschen
/// noch immer ein lesbares Preset ist. Wird ein Paketordner dadurch leer, wird
/// er entfernt.
pub fn delete(
    presets: &[&Preset],
    locations: &AdobeLocations,
    trash: &dyn Trash,
    session: &mut BackupSession,
) -> OperationReport {
    let mut report = OperationReport::new();
    let mut seen: Vec<&Path> = Vec::new();
    for preset in presets {
        // Ist ein Preset doppelt ausgewählt, wird es nur einmal gelöscht.
        if seen.iter().any(|path| same_path(path, &preset.path)) {
            continue;
        }
        seen.push(&preset.path);
        let outcome = match delete_one(&preset.path, locations, trash, session) {
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
    session: &mut BackupSession,
) -> Result<OperationReport, CoreError> {
    let presets: Vec<&Preset> = presets_of(library, section, group)?.iter().collect();
    Ok(delete(&presets, locations, trash, session))
}

fn delete_one(
    path: &Path,
    locations: &AdobeLocations,
    trash: &dyn Trash,
    session: &mut BackupSession,
) -> Result<(), CoreError> {
    check_inside(path, locations)?;
    // Liegt dort inzwischen etwas anderes, wird es nicht angerührt.
    read_metadata(path)?;
    let content = read_file(path)?;
    trash.move_to_trash(path)?;
    session.record(Change::Removed {
        path: path.to_path_buf(),
        content,
        to_trash: true,
    });
    remove_empty_folders(path, locations);
    Ok(())
}

/// Entfernt leer gewordene Paketordner oberhalb von `path`, aber nie die
/// Adobe-Ordner selbst. Klappt das nicht (z. B. weil der Ordner doch nicht
/// leer ist), bleibt er einfach stehen.
pub(crate) fn remove_empty_folders(path: &Path, locations: &AdobeLocations) {
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
