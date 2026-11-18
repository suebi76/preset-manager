//! Eine Importquelle – Ordner oder ZIP – nach Presets durchsuchen.
//!
//! Beide Arten von Quellen liefern dasselbe Ergebnis: eine [`ScannedSource`]
//! mit allen gefundenen Presets. Was danach passiert, muss also nicht mehr
//! unterscheiden, woher ein Preset kam.
//!
//! Durchsucht wird rekursiv, auch in ZIP-Archiven, die in einem Ordner oder
//! einem anderen Archiv liegen (F-05). Dateien, die keine Presets sind (z. B.
//! das Sidecar eines Fotos), stehen als „übersprungen“ im Bericht, defekte
//! als „gescheitert“ (F-06).

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use walkdir::WalkDir;

use crate::error::CoreError;
use crate::import::archive::{extract_presets, ArchiveLimits, ExtractedFile};
use crate::import::naming::safe_component;
use crate::library::scan::is_macos_metadata;
use crate::preset::xmp::{metadata_from_bytes, PresetMetadata};
use crate::report::{ItemOutcome, OperationReport};

/// Ein Preset, das in einer Quelle gefunden wurde.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundPreset {
    /// Wo die Datei zum Lesen liegt. Bei ZIP-Quellen im temporären Ordner.
    pub file: PathBuf,
    /// Wie die Datei dem Nutzer gezeigt wird: der echte Pfad bei Ordnern,
    /// bei Archiven Archivpfad plus Pfad im Archiv
    /// (`C:\Downloads\Paket.zip\XMP\Warm.xmp`). Der Dateiname daraus wird
    /// zum Dateinamen im Adobe-Ordner.
    pub shown_as: PathBuf,
    /// Die Kennfelder aus der Datei.
    pub metadata: PresetMetadata,
}

/// Das Ergebnis einer durchsuchten Quelle.
///
/// Bei ZIP-Quellen hält sie den temporären Ordner mit den entpackten Dateien
/// am Leben. Er wird gelöscht, sobald die `ScannedSource` (bzw. der
/// [`super::ImportPlan`], der sie übernimmt) nicht mehr gebraucht wird.
#[derive(Debug)]
pub struct ScannedSource {
    /// Der Ordner oder das Archiv, wie der Nutzer es gewählt hat.
    pub path: PathBuf,
    /// Name des Unterordners in `Settings`, in den das Paket installiert wird.
    /// Abgeleitet aus dem Ordner- bzw. Archivnamen, bereinigt für Windows.
    pub package_name: String,
    /// Alle gefundenen Presets und Profile.
    pub presets: Vec<FoundPreset>,
    /// Übersprungene und defekte Dateien der Quelle.
    pub report: OperationReport,
    /// Temporärer Ordner mit entpackten Dateien. Wird nur gehalten, damit er
    /// nicht zu früh gelöscht wird.
    _unpacked: Option<TempDir>,
}

/// Durchsucht einen Ordner oder ein ZIP-Archiv mit den üblichen
/// Schutzgrenzen ([`ArchiveLimits::default`]).
///
/// # Fehler
///
/// - [`CoreError::Io`], wenn die Quelle nicht existiert oder nicht lesbar ist
/// - [`CoreError::UnsupportedSource`] für andere Dateien als `.zip`
/// - die Fehler aus [`extract_presets`], wenn die Quelle ein Archiv ist
/// - [`CoreError::NoPresetsFound`], wenn die Quelle kein einziges Preset
///   enthält. Ist dabei eine Datei gescheitert (etwa ein defektes ZIP im
///   Ordner), wird stattdessen deren Fehler gemeldet – er erklärt mehr.
pub fn scan(path: &Path) -> Result<ScannedSource, CoreError> {
    scan_with_limits(path, &ArchiveLimits::default())
}

/// Wie [`scan`], aber mit eigenen Schutzgrenzen – vor allem für Tests.
pub fn scan_with_limits(path: &Path, limits: &ArchiveLimits) -> Result<ScannedSource, CoreError> {
    let metadata = std::fs::metadata(path).map_err(|source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let scanned = if metadata.is_dir() {
        scan_folder(path, limits)?
    } else if is_zip(path) {
        scan_archive(path, limits)?
    } else {
        return Err(CoreError::UnsupportedSource {
            path: path.to_path_buf(),
        });
    };

    if scanned.presets.is_empty() {
        return Err(
            first_failure(scanned.report).unwrap_or(CoreError::NoPresetsFound {
                source_path: path.to_path_buf(),
            }),
        );
    }
    Ok(scanned)
}

/// Eine Datei, die vielleicht ein Preset ist.
struct Candidate {
    file: PathBuf,
    shown_as: PathBuf,
}

fn scan_folder(folder: &Path, limits: &ArchiveLimits) -> Result<ScannedSource, CoreError> {
    let mut report = OperationReport::new();
    let mut candidates = Vec::new();
    let mut unpacked: Option<TempDir> = None;

    // Verknüpfungen werden hier nicht verfolgt: Ein heruntergeladener Ordner
    // soll nicht auf Dateien an ganz anderer Stelle zeigen können.
    let walker = WalkDir::new(folder)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| !is_macos_metadata(&entry.file_name().to_string_lossy()));

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
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if has_extension(path, "xmp") {
            candidates.push(Candidate {
                file: path.to_path_buf(),
                shown_as: path.to_path_buf(),
            });
        } else if is_zip(path) {
            match extract_nested_archive(path, &mut unpacked, limits) {
                Ok(files) => candidates.extend(files),
                Err(error) => report.record(path.to_path_buf(), ItemOutcome::Failed(error)),
            }
        }
    }

    let presets = read_candidates(candidates, &mut report);
    Ok(ScannedSource {
        path: folder.to_path_buf(),
        package_name: package_name(folder.file_name()),
        presets,
        report,
        _unpacked: unpacked,
    })
}

/// Entpackt ein ZIP, das in einem Quellordner liegt, in einen eigenen
/// Unterordner des temporären Ordners.
fn extract_nested_archive(
    archive: &Path,
    unpacked: &mut Option<TempDir>,
    limits: &ArchiveLimits,
) -> Result<Vec<Candidate>, CoreError> {
    let temporary_root = match unpacked {
        Some(folder) => folder.path().to_path_buf(),
        None => unpacked
            .insert(new_temporary_folder()?)
            .path()
            .to_path_buf(),
    };
    let target = tempfile::Builder::new()
        .prefix("zip-")
        .tempdir_in(&temporary_root)
        .map_err(|source| CoreError::Io {
            path: temporary_root.clone(),
            source,
        })?
        // Der Unterordner wird mit dem übergeordneten temporären Ordner gelöscht.
        .keep();

    let files = extract_presets(archive, &target, limits)?;
    Ok(files
        .into_iter()
        .map(|file| candidate_from_archive(archive, file))
        .collect())
}

fn scan_archive(archive: &Path, limits: &ArchiveLimits) -> Result<ScannedSource, CoreError> {
    let unpacked = new_temporary_folder()?;
    let files = extract_presets(archive, unpacked.path(), limits)?;
    let candidates = files
        .into_iter()
        .map(|file| candidate_from_archive(archive, file))
        .collect();

    let mut report = OperationReport::new();
    let presets = read_candidates(candidates, &mut report);
    Ok(ScannedSource {
        path: archive.to_path_buf(),
        package_name: package_name(archive.file_stem()),
        presets,
        report,
        _unpacked: Some(unpacked),
    })
}

fn candidate_from_archive(archive: &Path, file: ExtractedFile) -> Candidate {
    let shown_as = file
        .inner_path
        .split('/')
        .fold(archive.to_path_buf(), |path, part| path.join(part));
    Candidate {
        file: file.disk_path,
        shown_as,
    }
}

/// Liest alle Kandidaten und trägt Nicht-Presets und defekte Dateien in den
/// Bericht ein.
fn read_candidates(candidates: Vec<Candidate>, report: &mut OperationReport) -> Vec<FoundPreset> {
    let mut presets = Vec::new();
    for candidate in candidates {
        let read = read_limited(&candidate.file, &candidate.shown_as)
            .and_then(|bytes| metadata_from_bytes(&bytes, &candidate.shown_as));

        match read {
            Ok(metadata) => presets.push(FoundPreset {
                file: candidate.file,
                shown_as: candidate.shown_as,
                metadata,
            }),
            Err(CoreError::NotAPreset { .. }) => report.record(
                candidate.shown_as,
                ItemOutcome::Skipped {
                    reason: "keine Preset-Datei (z. B. Sidecar eines Fotos)".to_owned(),
                },
            ),
            Err(error) => report.record(candidate.shown_as, ItemOutcome::Failed(error)),
        }
    }
    presets
}

/// Größer ist kein echtes Preset. Selbst Presets mit eingebettetem Profil
/// haben wenige Megabyte. Die Grenze verhindert, dass eine riesige `.xmp`-Datei
/// in einem Quellordner komplett in den Speicher gelesen wird.
const MAX_PRESET_BYTES: u64 = 50 * 1024 * 1024;

/// Liest eine Datei, aber nur, wenn sie nicht größer als
/// [`MAX_PRESET_BYTES`] ist.
pub(crate) fn read_limited(file: &Path, shown_as: &Path) -> Result<Vec<u8>, CoreError> {
    let io_error = |source| CoreError::Io {
        path: shown_as.to_path_buf(),
        source,
    };
    let size = std::fs::metadata(file).map_err(io_error)?.len();
    if size > MAX_PRESET_BYTES {
        return Err(io_error(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Datei ist {} MB groß – zu groß für ein Preset",
                size / (1024 * 1024)
            ),
        )));
    }
    std::fs::read(file).map_err(io_error)
}

fn new_temporary_folder() -> Result<TempDir, CoreError> {
    tempfile::Builder::new()
        .prefix("apm-import-")
        .tempdir()
        .map_err(|source| CoreError::Io {
            path: std::env::temp_dir(),
            source,
        })
}

fn package_name(name: Option<&std::ffi::OsStr>) -> String {
    let name = name.map(|name| name.to_string_lossy()).unwrap_or_default();
    safe_component(&name, "Import")
}

fn first_failure(report: OperationReport) -> Option<CoreError> {
    report
        .into_items()
        .into_iter()
        .find_map(|(_path, outcome)| match outcome {
            ItemOutcome::Failed(error) => Some(error),
            _ => None,
        })
}

fn is_zip(path: &Path) -> bool {
    has_extension(path, "zip")
}

fn has_extension(path: &Path, wanted: &str) -> bool {
    path.extension()
        .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case(wanted))
}
