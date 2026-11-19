//! Preset-Dateien lesen – mit Obergrenze.
//!
//! Presets und Profile können von Unbekannten stammen (Downloads, ZIPs aus
//! dem Netz) und liegen in Ordnern, in die jedes Programm schreiben kann.
//! Eine riesige `.xmp`-Datei darf die App nicht dazu bringen, Gigabytes in
//! den Speicher zu lesen. Alle Stellen, die eine Preset-Datei ganz lesen,
//! gehen deshalb über [`read_preset_file`] (`Planung/04-Fehlerbehandlung.md`,
//! N-08).

use std::io::Read;
use std::path::Path;

use crate::error::CoreError;

/// Größer ist kein echtes Preset. Selbst Profile mit eingebetteten
/// Farbtabellen haben wenige Megabyte.
pub const MAX_PRESET_BYTES: u64 = 50 * 1024 * 1024;

/// Liest eine Preset-Datei ganz, wenn sie nicht größer als
/// [`MAX_PRESET_BYTES`] ist.
///
/// Gelesen wird höchstens ein Byte mehr als erlaubt – auch eine Datei, die
/// während des Lesens wächst, kann die Grenze nicht umgehen.
///
/// # Fehler
///
/// [`CoreError::Io`], wenn die Datei fehlt, nicht gelesen werden kann oder zu
/// groß ist.
pub fn read_preset_file(path: &Path) -> Result<Vec<u8>, CoreError> {
    read_limited_as(path, path)
}

/// Wie [`read_preset_file`]; Fehlermeldungen nennen aber `shown_as` – etwa
/// den Pfad im ZIP statt des temporären Entpackorts.
pub(crate) fn read_limited_as(file: &Path, shown_as: &Path) -> Result<Vec<u8>, CoreError> {
    let io_error = |source| CoreError::Io {
        path: shown_as.to_path_buf(),
        source,
    };
    let opened = std::fs::File::open(file).map_err(io_error)?;
    let expected = opened.metadata().map(|meta| meta.len()).unwrap_or(0);
    let mut content = Vec::with_capacity(expected.min(MAX_PRESET_BYTES) as usize);
    opened
        .take(MAX_PRESET_BYTES + 1)
        .read_to_end(&mut content)
        .map_err(io_error)?;
    if content.len() as u64 > MAX_PRESET_BYTES {
        return Err(io_error(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Datei ist größer als {} MB – zu groß für ein Preset",
                MAX_PRESET_BYTES / (1024 * 1024)
            ),
        )));
    }
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liest_kleine_dateien() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Warm.xmp");
        std::fs::write(&file, b"<x/>").unwrap();
        assert_eq!(read_preset_file(&file).unwrap(), b"<x/>");
    }

    #[test]
    fn lehnt_zu_grosse_dateien_ab() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Riesig.xmp");
        // Eine „dünne“ Datei: groß laut Dateisystem, ohne Platz zu belegen.
        let handle = std::fs::File::create(&file).unwrap();
        handle.set_len(MAX_PRESET_BYTES + 10).unwrap();
        drop(handle);

        let error = read_preset_file(&file).unwrap_err();

        assert!(matches!(error, CoreError::Io { .. }), "{error:?}");
        assert!(error.to_string().contains("Riesig.xmp"));
    }

    #[test]
    fn fehlende_datei_ist_ein_fehler() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_preset_file(&dir.path().join("fehlt.xmp")).is_err());
    }
}
