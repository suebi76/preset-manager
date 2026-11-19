//! Presets und Profile als ZIP-Datei speichern (F-37).
//!
//! Gedacht zum Weitergeben, Sichern oder Umziehen auf einen anderen Rechner.
//! Das ZIP lässt sich mit Preset Manager wieder installieren und ebenso über
//! Adobes eigenen Importdialog in Camera Raw.
//!
//! Aufbau: je Gruppe ein Ordner, darin die Dateien unverändert und mit ihrem
//! bisherigen Namen. Presets ohne Gruppe liegen oben im ZIP. Gleiche Namen
//! im selben Ordner werden nummeriert („Warm (2).xmp“).
//!
//! Das ZIP entsteht zuerst im Speicher und wird dann in einem Zug geschrieben
//! – ein abgebrochener Export hinterlässt keine halbe Datei.

use std::collections::BTreeSet;
use std::io::{Cursor, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::error::CoreError;
use crate::import::naming::{numbered, safe_component};
use crate::library::scan::collation_key;
use crate::preset::model::Preset;
use crate::report::{ItemOutcome, OperationReport};
use crate::safe_fs::{read_preset_file, write_atomic, WriteMode};

/// Speichert die Presets als ZIP unter `target` (eine vorhandene Datei wird
/// ersetzt – der Speichern-Dialog von Windows hat vorher gefragt).
///
/// Der Bericht nennt jede Datei: geschrieben, oder nicht lesbar und deshalb
/// ausgelassen.
///
/// # Fehler
///
/// - [`CoreError::NoPresetsFound`], wenn keine einzige Datei gelesen werden
///   konnte – dann wird auch kein ZIP angelegt.
/// - [`CoreError::Io`], wenn das ZIP nicht geschrieben werden kann.
pub fn export_zip(presets: &[&Preset], target: &Path) -> Result<OperationReport, CoreError> {
    let mut sorted: Vec<&Preset> = presets.to_vec();
    sorted.sort_by_cached_key(|preset| {
        (
            collation_key(&preset.group),
            collation_key(&preset.name),
            preset.path.clone(),
        )
    });
    sorted.dedup_by(|a, b| a.path == b.path);

    let io_error = |error: zip::result::ZipError| CoreError::Io {
        path: target.to_path_buf(),
        source: std::io::Error::other(error),
    };
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let mut report = OperationReport::new();
    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut written = 0;

    for preset in sorted {
        let content = match read_preset_file(&preset.path) {
            Ok(content) => content,
            Err(error) => {
                report.record(preset.path.clone(), ItemOutcome::Failed(error));
                continue;
            }
        };
        let entry = entry_name(preset, &mut used);
        zip.start_file(entry, options).map_err(io_error)?;
        zip.write_all(&content).map_err(|source| CoreError::Io {
            path: target.to_path_buf(),
            source,
        })?;
        report.record(preset.path.clone(), ItemOutcome::Done);
        written += 1;
    }

    if written == 0 {
        return Err(CoreError::NoPresetsFound {
            source_path: target.to_path_buf(),
        });
    }
    let bytes = zip.finish().map_err(io_error)?.into_inner();
    write_atomic(target, &bytes, WriteMode::Replace)?;
    Ok(report)
}

/// Der Pfad im ZIP: `Gruppe/Datei.xmp`, ohne Gruppe nur `Datei.xmp`. Bereits
/// vergebene Namen (ohne Rücksicht auf Groß-/Kleinschreibung) werden
/// nummeriert.
fn entry_name(preset: &Preset, used: &mut BTreeSet<String>) -> String {
    let file = preset
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let file = safe_component(&file, "Preset.xmp");
    let folder = if preset.group.trim().is_empty() {
        String::new()
    } else {
        format!("{}/", safe_component(preset.group.trim(), "Gruppe"))
    };
    let mut candidate = format!("{folder}{file}");
    let mut number = 2;
    while !used.insert(candidate.to_lowercase()) {
        candidate = format!("{folder}{}", numbered(&file, number));
        number += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;
    use crate::preset::model::{PresetOrigin, PresetType};

    fn write_preset(dir: &Path, file: &str, name: &str, group: &str) -> Preset {
        let path = dir.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            format!(
                r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:PresetType="Normal" crs:Name="{name}" crs:Group="{}"/></rdf:RDF></x:xmpmeta>"#,
                group.replace('&', "&amp;")
            ),
        )
        .unwrap();
        Preset {
            path,
            origin: PresetOrigin::Settings,
            preset_type: PresetType::Preset,
            name: name.to_owned(),
            group: group.to_owned(),
            uuid: None,
            sort_name: String::new(),
            active: true,
        }
    }

    fn entries(zip_path: &Path) -> Vec<(String, Vec<u8>)> {
        let file = std::fs::File::open(zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        (0..archive.len())
            .map(|index| {
                let mut entry = archive.by_index(index).unwrap();
                let mut content = Vec::new();
                entry.read_to_end(&mut content).unwrap();
                (entry.name().unwrap().into_owned(), content)
            })
            .collect()
    }

    #[test]
    fn je_gruppe_ein_ordner_inhalt_unveraendert() {
        let dir = tempfile::tempdir().unwrap();
        let warm = write_preset(dir.path(), "a/Warm.xmp", "Warm", "Sommer");
        let cold = write_preset(dir.path(), "b/Kalt.xmp", "Kalt", "Winter");
        let loose = write_preset(dir.path(), "Frei.xmp", "Frei", "");
        let target = dir.path().join("Export.zip");

        let report = export_zip(&[&warm, &cold, &loose], &target).unwrap();

        assert_eq!(report.done_count(), 3);
        let entries = entries(&target);
        let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            vec!["Frei.xmp", "Sommer/Warm.xmp", "Winter/Kalt.xmp"]
        );
        assert_eq!(entries[1].1, std::fs::read(&warm.path).unwrap());
    }

    #[test]
    fn gleiche_namen_werden_nummeriert() {
        let dir = tempfile::tempdir().unwrap();
        let first = write_preset(dir.path(), "a/Warm.xmp", "Warm", "Sommer");
        let second = write_preset(dir.path(), "b/WARM.xmp", "Warm 2", "Sommer");
        let target = dir.path().join("Export.zip");

        export_zip(&[&first, &second, &first], &target).unwrap();

        let names: Vec<String> = entries(&target).into_iter().map(|(name, _)| name).collect();
        assert_eq!(names, vec!["Sommer/Warm.xmp", "Sommer/WARM (2).xmp"]);
    }

    #[test]
    fn unlesbare_dateien_werden_ausgelassen() {
        let dir = tempfile::tempdir().unwrap();
        let warm = write_preset(dir.path(), "Warm.xmp", "Warm", "Sommer");
        let mut gone = warm.clone();
        gone.path = dir.path().join("fehlt.xmp");
        let target = dir.path().join("Export.zip");

        let report = export_zip(&[&warm, &gone], &target).unwrap();
        assert_eq!(report.done_count(), 1);
        assert_eq!(report.failed_count(), 1);

        let only_missing = export_zip(&[&gone], &dir.path().join("Leer.zip"));
        assert!(matches!(
            only_missing,
            Err(CoreError::NoPresetsFound { .. })
        ));
        assert!(!dir.path().join("Leer.zip").exists(), "kein leeres ZIP");
    }

    #[test]
    fn export_laesst_sich_wieder_einlesen() {
        let dir = tempfile::tempdir().unwrap();
        let warm = write_preset(dir.path(), "Warm.xmp", "Warm", "Sommer & Sonne");
        let target = dir.path().join("Paket.zip");
        export_zip(&[&warm], &target).unwrap();

        let scanned = crate::import::scan(&target).unwrap();

        assert_eq!(scanned.presets.len(), 1);
        assert_eq!(scanned.presets[0].metadata.name, "Warm");
        assert_eq!(scanned.presets[0].metadata.group, "Sommer & Sonne");
    }
}
