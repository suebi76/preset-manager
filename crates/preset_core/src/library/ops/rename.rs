//! Umbenennen von Presets (F-24) und Gruppen (F-25).

use std::path::PathBuf;

use crate::adobe::locations::same_path;
use crate::error::CoreError;
use crate::preset::model::{Preset, Section};
use crate::preset::xmp_edit::MetadataChanges;
use crate::report::OperationReport;
use crate::safe_fs::{write_atomic, write_to_free_name, Change, WriteMode};

use super::common::{
    check_inside, new_file_path, prepare, presets_of, rewrite_in_place, same_name, valid_name,
};

use super::Context;

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
    let (original, content) = prepare(&preset.path, &changes, context)?;
    let new_path = new_file_path(&preset.path, &new_name);

    if new_path == preset.path || same_path(&new_path, &preset.path) {
        write_atomic(&preset.path, &content, WriteMode::Replace)?;
        context.backups.record(Change::Rewritten {
            path: preset.path.clone(),
            before: original,
            after: content,
        });
        if new_path != preset.path {
            // Nur Groß-/Kleinschreibung des Dateinamens ändert sich.
            std::fs::rename(&preset.path, &new_path).map_err(|source| CoreError::Io {
                path: preset.path.clone(),
                source,
            })?;
            context.backups.record(Change::Moved {
                from: preset.path.clone(),
                to: new_path.clone(),
            });
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
    context.backups.record(Change::Created {
        path: written.clone(),
        content,
        from_trash: false,
    });
    context.backups.record(Change::Removed {
        path: preset.path.clone(),
        content: original,
        to_trash: false,
    });
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
