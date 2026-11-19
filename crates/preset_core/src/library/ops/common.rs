//! Gemeinsame Bausteine: an Ort und Stelle ändern, Namen prüfen, Lage
//! prüfen.

use std::path::{Path, PathBuf};

use crate::adobe::AdobeLocations;
use crate::error::{CoreError, NameProblem};
use crate::import::naming::safe_component;
use crate::library::scan::Library;
use crate::preset::model::{Preset, Section};
use crate::preset::xmp_edit::{rewrite_metadata, MetadataChanges};
use crate::report::ItemOutcome;
use crate::safe_fs::{read_preset_file, write_atomic, Change, WriteMode};

use super::Context;

/// Höchstlänge eines Preset- oder Gruppennamens in Zeichen.
const MAX_DISPLAY_NAME_CHARS: usize = 255;

// ---------------------------------------------------------------------------
// Gemeinsame Bausteine
// ---------------------------------------------------------------------------

/// Ändert eine Datei an Ort und Stelle: prüfen, sichern, atomar schreiben.
pub(super) fn rewrite_in_place(
    path: &Path,
    changes: &MetadataChanges,
    context: &mut Context<'_>,
) -> ItemOutcome {
    let result = check_inside(path, context.locations)
        .and_then(|()| prepare(path, changes, context))
        .and_then(|(original, content)| {
            write_atomic(path, &content, WriteMode::Replace)?;
            context.backups.record(Change::Rewritten {
                path: path.to_path_buf(),
                before: original,
                after: content,
            });
            Ok(())
        });
    match result {
        Ok(()) => ItemOutcome::Done,
        Err(error) => ItemOutcome::Failed(error),
    }
}

/// Liest die Datei, berechnet den neuen Inhalt und sichert das Original.
/// Geschrieben wird noch nichts. Gibt Original und neuen Inhalt zurück.
pub(super) fn prepare(
    path: &Path,
    changes: &MetadataChanges,
    context: &mut Context<'_>,
) -> Result<(Vec<u8>, Vec<u8>), CoreError> {
    let original = read_file(path)?;
    let content = rewrite_metadata(&original, changes, path)?;
    let relative = context.locations.relative_path(path)?;
    context.backups.back_up(path, relative)?;
    Ok((original, content))
}

/// Liest eine Preset-Datei ganz (mit Obergrenze, siehe [`read_preset_file`]).
pub(crate) fn read_file(path: &Path) -> Result<Vec<u8>, CoreError> {
    read_preset_file(path)
}

/// Der Dateiname zum neuen Anzeigenamen, im selben Ordner wie bisher.
pub(super) fn new_file_path(current: &Path, new_name: &str) -> PathBuf {
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
pub(super) fn presets_of<'a>(
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
pub(super) fn check_inside(path: &Path, locations: &AdobeLocations) -> Result<(), CoreError> {
    locations.ensure_contains(path)
}

/// Zwei Namen gelten als gleich, wenn sie sich nur in Groß-/Kleinschreibung
/// unterscheiden – sonst wären sie in der Liste kaum zu unterscheiden.
pub fn same_name(a: &str, b: &str) -> bool {
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
