//! Aktivieren und deaktivieren (F-48): Dateien wandern zwischen den
//! Adobe-Ordnern und dem Ordner der deaktivierten Presets.

use std::path::{Path, PathBuf};

use crate::adobe::locations::same_path;
use crate::adobe::AdobeLocations;
use crate::error::CoreError;
use crate::preset::model::Preset;
use crate::preset::xmp::read_metadata;
use crate::report::{ItemOutcome, OperationReport};
use crate::safe_fs::{move_file, BackupSession, Change};

use super::common::check_inside;
use super::delete::remove_empty_folders;

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
    session: &mut BackupSession,
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
                    session.record(Change::Moved {
                        from: preset.path.clone(),
                        to: target.clone(),
                    });
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
