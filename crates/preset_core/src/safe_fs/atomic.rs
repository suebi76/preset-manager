//! Atomares Schreiben.
//!
//! Wer eine Datei direkt beschreibt und mittendrin abbricht (Absturz,
//! Stromausfall, voller Datenträger), hinterlässt eine halbe Datei. Camera Raw
//! würde ein halbes Preset als defekt anzeigen oder ganz übergehen.
//!
//! Deshalb wird zuerst eine temporäre Datei **im selben Ordner** geschrieben
//! und auf den Datenträger gebracht. Erst danach bekommt sie ihren richtigen
//! Namen. Das Umbenennen innerhalb eines Ordners ist unter Windows ein
//! einzelner Schritt: Danach gibt es entweder die alte oder die neue Datei,
//! nie eine Mischung.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::CoreError;
use crate::import::naming::numbered;

/// Was passieren soll, wenn die Zieldatei schon existiert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    /// Nur schreiben, wenn es die Datei noch nicht gibt. Sonst
    /// [`CoreError::AlreadyExists`].
    CreateNew,
    /// Eine vorhandene Datei ersetzen. Der Aufrufer ist dafür verantwortlich,
    /// sie vorher zu sichern.
    Replace,
}

/// Schreibt `content` atomar nach `target`.
///
/// Der Ordner von `target` muss bereits existieren. Scheitert das Schreiben,
/// bleibt die temporäre Datei nicht liegen und eine vorhandene Zieldatei
/// unverändert.
///
/// # Fehler
///
/// - [`CoreError::AlreadyExists`] bei [`WriteMode::CreateNew`], wenn die
///   Zieldatei schon da ist.
/// - [`CoreError::Io`], wenn Schreiben oder Umbenennen scheitert.
pub fn write_atomic(target: &Path, content: &[u8], mode: WriteMode) -> Result<(), CoreError> {
    let io_error = |source| CoreError::Io {
        path: target.to_path_buf(),
        source,
    };
    let folder = target
        .parent()
        .ok_or_else(|| io_error(std::io::Error::other("Zielpfad hat keinen Ordner")))?;

    // Der Punkt am Anfang und die Endung `.tmp` sorgen dafür, dass Camera Raw
    // die Datei nicht für ein Preset hält, falls sie doch einmal liegen bleibt.
    let mut temporary = tempfile::Builder::new()
        .prefix(".apm-")
        .suffix(".tmp")
        .tempfile_in(folder)
        .map_err(io_error)?;
    temporary.write_all(content).map_err(io_error)?;
    temporary.as_file().sync_all().map_err(io_error)?;

    let persisted = match mode {
        WriteMode::CreateNew => temporary.persist_noclobber(target),
        WriteMode::Replace => temporary.persist(target),
    };
    match persisted {
        Ok(_file) => Ok(()),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(CoreError::AlreadyExists {
                path: target.to_path_buf(),
            })
        }
        Err(error) => Err(io_error(error.error)),
    }
}

/// Schreibt `content` nach `target` oder – falls die Datei schon existiert –
/// unter den ersten freien nummerierten Namen im selben Ordner
/// (`Warm (2).xmp`, `Warm (3).xmp` …). Gibt den tatsächlichen Pfad zurück.
///
/// Überschrieben wird nie. Der Aufrufer prüft vorher, dass `target` an einer
/// erlaubten Stelle liegt; die Varianten liegen im selben Ordner.
///
/// # Fehler
///
/// Wie [`write_atomic`]; [`CoreError::AlreadyExists`], wenn auch 999
/// Varianten belegt sind.
pub fn write_to_free_name(target: &Path, content: &[u8]) -> Result<PathBuf, CoreError> {
    let folder = target.parent().unwrap_or(target);
    let file_name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    for number in 1..=999 {
        let candidate = if number == 1 {
            target.to_path_buf()
        } else {
            folder.join(numbered(&file_name, number))
        };
        match write_atomic(&candidate, content, WriteMode::CreateNew) {
            Ok(()) => return Ok(candidate),
            Err(CoreError::AlreadyExists { .. }) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(CoreError::AlreadyExists {
        path: target.to_path_buf(),
    })
}

/// Verschiebt eine Datei. Klappt das Umbenennen nicht (anderes Laufwerk über
/// eine Verknüpfung), wird kopiert, geprüft und erst dann das Original
/// entfernt.
///
/// # Fehler
///
/// [`CoreError::Io`] oder [`CoreError::AlreadyExists`]; die Datei liegt dann
/// weiter an ihrem alten Ort.
pub fn move_file(from: &Path, to: &Path) -> Result<(), CoreError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn files_in(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn schreibt_neue_datei_ohne_reste() {
        let folder = tempfile::tempdir().unwrap();
        let target = folder.path().join("neu.xmp");

        write_atomic(&target, b"Inhalt", WriteMode::CreateNew).unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"Inhalt");
        assert_eq!(files_in(folder.path()), ["neu.xmp"]);
    }

    #[test]
    fn ueberschreibt_nicht_ohne_erlaubnis() {
        let folder = tempfile::tempdir().unwrap();
        let target = folder.path().join("da.xmp");
        std::fs::write(&target, b"alt").unwrap();

        let result = write_atomic(&target, b"neu", WriteMode::CreateNew);

        assert!(matches!(result, Err(CoreError::AlreadyExists { .. })));
        assert_eq!(std::fs::read(&target).unwrap(), b"alt");
        assert_eq!(files_in(folder.path()), ["da.xmp"]);
    }

    #[test]
    fn ersetzt_wenn_erlaubt() {
        let folder = tempfile::tempdir().unwrap();
        let target = folder.path().join("da.xmp");
        std::fs::write(&target, b"alt").unwrap();

        write_atomic(&target, b"neu", WriteMode::Replace).unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"neu");
        assert_eq!(files_in(folder.path()), ["da.xmp"]);
    }

    #[test]
    fn freier_name_nummeriert_statt_zu_ueberschreiben() {
        let folder = tempfile::tempdir().unwrap();
        let target = folder.path().join("Warm.xmp");

        let first = write_to_free_name(&target, b"1").unwrap();
        let second = write_to_free_name(&target, b"2").unwrap();

        assert_eq!(first, target);
        assert_eq!(second, folder.path().join("Warm (2).xmp"));
        assert_eq!(std::fs::read(&target).unwrap(), b"1");
    }

    #[test]
    fn fehlender_ordner_ist_ein_fehler() {
        let folder = tempfile::tempdir().unwrap();
        let target = folder.path().join("fehlt").join("x.xmp");

        let result = write_atomic(&target, b"x", WriteMode::CreateNew);

        assert!(matches!(result, Err(CoreError::Io { .. })));
    }
}
