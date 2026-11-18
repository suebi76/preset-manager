//! ZIP-Archive sicher entpacken.
//!
//! Preset-Pakete kommen meist als ZIP aus dem Netz. Ein Archiv kann absichtlich
//! oder versehentlich gefährlich sein. Dieses Modul schützt so:
//!
//! | Gefahr | Schutz |
//! |---|---|
//! | Einträge wie `..\..\Windows\…` („Zip Slip“) | Das ganze Archiv wird abgelehnt ([`CoreError::UnsafeArchiveEntry`]) |
//! | Ungültige Windows-Namen im Archiv (`CON`, `a:b`) | Dateien werden unter eigenen, durchnummerierten Namen entpackt; die Namen aus dem Archiv werden nie als Pfad benutzt |
//! | „ZIP-Bombe“: winziges Archiv, riesiger Inhalt | Grenzen für entpackte Größe, Dateianzahl und Verschachtelung ([`ArchiveLimits`]); gezählt werden die tatsächlich entpackten Bytes, nicht die Angaben im Archiv |
//! | Verknüpfungen (Symlinks) im Archiv | werden übergangen |
//!
//! Entpackt werden nur `.xmp`-Dateien und eingebettete ZIP-Archive. Bilder,
//! Anleitungen und andere Beigaben bleiben im Archiv.
//!
//! Ziel ist immer ein temporärer Ordner, nie der Adobe-Ordner. Kopiert wird
//! erst, wenn alle Prüfungen bestanden sind (siehe [`super::plan`]).

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::error::{ArchiveLimit, CoreError};
use crate::library::scan::is_macos_metadata;

/// Schutzgrenzen beim Entpacken (`Planung/04-Fehlerbehandlung.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLimits {
    /// Höchstens so viele Bytes werden insgesamt entpackt.
    pub max_unpacked_bytes: u64,
    /// Höchstens so viele Einträge dürfen alle Archive zusammen haben.
    pub max_entries: usize,
    /// Höchstens so viele ZIP-Ebenen: 1 = keine ZIPs im ZIP.
    pub max_nesting: usize,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_unpacked_bytes: 500 * 1024 * 1024,
            max_entries: 10_000,
            max_nesting: 3,
        }
    }
}

/// Eine entpackte Datei.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedFile {
    /// Wo die Datei jetzt liegt (im temporären Ordner).
    pub disk_path: PathBuf,
    /// Ihr Pfad im Archiv, mit `/` getrennt, z. B. `XMP/Warm.xmp` oder bei
    /// verschachtelten Archiven `Bonus.zip/Kalt.xmp`. Zum Anzeigen und als
    /// Grundlage für den Dateinamen im Adobe-Ordner.
    pub inner_path: String,
}

/// Entpackt alle Presets aus `archive` nach `into`, auch aus ZIP-Archiven im
/// Archiv.
///
/// `into` muss ein leerer, temporärer Ordner sein. Bei einem Fehler kann er
/// teilweise gefüllt sein; der Aufrufer verwirft ihn dann.
///
/// # Fehler
///
/// - [`CoreError::InvalidArchive`]: kein gültiges ZIP, defekt, verschlüsselt
///   oder mit nicht unterstütztem Komprimierungsverfahren
/// - [`CoreError::UnsafeArchiveEntry`]: ein Eintrag zeigt aus dem Archiv heraus
/// - [`CoreError::ArchiveTooLarge`]: eine der [`ArchiveLimits`] ist überschritten
/// - [`CoreError::Io`]: Lesen des Archivs oder Schreiben in `into` scheitert
pub fn extract_presets(
    archive: &Path,
    into: &Path,
    limits: &ArchiveLimits,
) -> Result<Vec<ExtractedFile>, CoreError> {
    let mut extraction = Extraction {
        archive,
        into,
        limits,
        unpacked_bytes: 0,
        entries_seen: 0,
        next_number: 1,
        extracted: Vec::new(),
    };
    extraction.extract(archive, "", 1)?;
    Ok(extraction.extracted)
}

/// Zustand eines Entpackvorgangs über alle Ebenen hinweg. Die Grenzen gelten
/// für alle Ebenen zusammen, damit eine Bombe sich nicht auf viele kleine
/// innere Archive verteilen kann.
struct Extraction<'a> {
    /// Das Archiv, das der Nutzer gewählt hat – für Fehlermeldungen.
    archive: &'a Path,
    into: &'a Path,
    limits: &'a ArchiveLimits,
    unpacked_bytes: u64,
    entries_seen: usize,
    next_number: u32,
    extracted: Vec<ExtractedFile>,
}

impl Extraction<'_> {
    /// Entpackt ein Archiv der Ebene `level`. `prefix` ist der Pfad der
    /// äußeren Archive für die Anzeige, z. B. `Bonus.zip/`.
    fn extract(&mut self, zip_file: &Path, prefix: &str, level: usize) -> Result<(), CoreError> {
        let file = File::open(zip_file).map_err(|source| CoreError::Io {
            path: zip_file.to_path_buf(),
            source,
        })?;
        let mut zip = ZipArchive::new(file).map_err(|error| self.invalid(prefix, error))?;

        self.entries_seen += zip.len();
        if self.entries_seen > self.limits.max_entries {
            return Err(self.too_large(ArchiveLimit::TooManyEntries {
                max: self.limits.max_entries,
            }));
        }

        for index in 0..zip.len() {
            let mut entry = zip
                .by_index(index)
                .map_err(|error| self.invalid(prefix, error))?;
            let entry_name = String::from_utf8_lossy(entry.name_raw()).into_owned();
            let Some(parts) = enclosed_parts(&entry_name) else {
                return Err(CoreError::UnsafeArchiveEntry {
                    entry: format!("{prefix}{entry_name}"),
                });
            };
            let is_macos_file = parts.iter().any(|part| is_macos_metadata(part));
            if entry.is_dir() || entry.is_symlink() || is_macos_file {
                continue;
            }

            let inner_path = format!("{prefix}{}", parts.join("/"));
            match lowercase_extension(&parts).as_deref() {
                Some("xmp") => {
                    let disk_path = self.next_path("xmp");
                    self.copy_entry(&mut entry, &disk_path, &inner_path)?;
                    self.extracted.push(ExtractedFile {
                        disk_path,
                        inner_path,
                    });
                }
                Some("zip") => {
                    if level >= self.limits.max_nesting {
                        return Err(self.too_large(ArchiveLimit::TooDeep {
                            max: self.limits.max_nesting,
                        }));
                    }
                    let disk_path = self.next_path("zip");
                    self.copy_entry(&mut entry, &disk_path, &inner_path)?;
                    self.extract(&disk_path, &format!("{inner_path}/"), level + 1)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Kopiert den Inhalt eines Eintrags und zählt dabei die Bytes mit.
    fn copy_entry(
        &mut self,
        entry: &mut impl Read,
        destination: &Path,
        inner_path: &str,
    ) -> Result<(), CoreError> {
        let write_error = |source| CoreError::Io {
            path: destination.to_path_buf(),
            source,
        };
        let mut output = File::create_new(destination).map_err(write_error)?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = entry
                .read(&mut buffer)
                .map_err(|error| CoreError::InvalidArchive {
                    path: self.archive.to_path_buf(),
                    reason: format!("{inner_path}: {error}"),
                })?;
            if read == 0 {
                return Ok(());
            }
            self.unpacked_bytes += read as u64;
            if self.unpacked_bytes > self.limits.max_unpacked_bytes {
                return Err(self.too_large(ArchiveLimit::TooLarge {
                    max_mb: self.limits.max_unpacked_bytes / (1024 * 1024),
                }));
            }
            output
                .write_all(buffer.get(..read).unwrap_or_default())
                .map_err(write_error)?;
        }
    }

    /// Nächster freier Dateiname im temporären Ordner: `00001.xmp` usw.
    fn next_path(&mut self, extension: &str) -> PathBuf {
        let name = format!("{:05}.{extension}", self.next_number);
        self.next_number += 1;
        self.into.join(name)
    }

    fn invalid(&self, prefix: &str, error: zip::result::ZipError) -> CoreError {
        let location = if prefix.is_empty() {
            String::new()
        } else {
            format!("in {}: ", prefix.trim_end_matches('/'))
        };
        CoreError::InvalidArchive {
            path: self.archive.to_path_buf(),
            reason: format!("{location}{error}"),
        }
    }

    fn too_large(&self, limit: ArchiveLimit) -> CoreError {
        CoreError::ArchiveTooLarge {
            path: self.archive.to_path_buf(),
            limit,
        }
    }
}

/// Zerlegt einen Eintragsnamen in seine Ordner- und Dateinamen und prüft
/// dabei, dass er im Archiv bleibt.
///
/// `None` (= unzulässig) für:
/// - absolute Pfade (`/etc/x`, `\Windows\x`)
/// - Laufwerksangaben (`C:\x`, `C:x`)
/// - `..`, das über die oberste Ebene hinausführt (`a/../../x`)
/// - Nullzeichen im Namen
///
/// `\` und `/` gelten beide als Trenner, weil Windows beide akzeptiert.
/// Harmlose Umwege wie `a/../b.xmp` sind erlaubt und ergeben `["b.xmp"]`.
///
/// Die Prüfung ist bewusst eigene Logik statt der Prüfung der ZIP-Bibliothek:
/// Sie verhält sich auf jedem Betriebssystem gleich und lehnt keine Namen
/// ab, die nur unter Windows ungültig wären (`CON.xmp`) – solche Namen
/// werden ja nie als Pfad benutzt, sondern beim Installieren bereinigt.
fn enclosed_parts(name: &str) -> Option<Vec<String>> {
    if name.contains('\0') {
        return None;
    }
    let normalized = name.replace('\\', "/");
    if normalized.starts_with('/') || has_drive_letter(&normalized) {
        return None;
    }

    let mut parts: Vec<String> = Vec::new();
    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part.to_owned()),
        }
    }
    Some(parts)
}

/// `true` für Namen, die mit einer Laufwerksangabe wie `C:` beginnen.
fn has_drive_letter(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(
        (characters.next(), characters.next()),
        (Some(letter), Some(':')) if letter.is_ascii_alphabetic()
    )
}

/// Endung des Dateinamens in Kleinbuchstaben. Punkte und Leerzeichen am Ende
/// zählen nicht – Windows schneidet sie ab, `Warm.xmp.` ist also `Warm.xmp`.
fn lowercase_extension(parts: &[String]) -> Option<String> {
    let file_name = Path::new(parts.last()?.trim_end_matches([' ', '.']));
    Some(
        file_name
            .extension()?
            .to_string_lossy()
            .to_ascii_lowercase(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zerlegt_harmlose_namen() {
        assert_eq!(
            enclosed_parts("Paket/XMP/Warm.xmp").unwrap(),
            ["Paket", "XMP", "Warm.xmp"]
        );
        assert_eq!(
            enclosed_parts("Paket\\Warm.xmp").unwrap(),
            ["Paket", "Warm.xmp"]
        );
        assert_eq!(enclosed_parts("./a//b/../c.xmp").unwrap(), ["a", "c.xmp"]);
        assert_eq!(enclosed_parts("CON.xmp").unwrap(), ["CON.xmp"]);
    }

    #[test]
    fn lehnt_ausbrechende_namen_ab() {
        for name in [
            "../x.xmp",
            "a/../../x.xmp",
            "..\\..\\x.xmp",
            "/x.xmp",
            "\\x.xmp",
            "C:\\x.xmp",
            "C:x.xmp",
            "x\0.xmp",
        ] {
            assert_eq!(enclosed_parts(name), None, "{name}");
        }
    }

    #[test]
    fn erkennt_endung_unabhaengig_von_schreibweise() {
        let parts = vec!["Paket".to_owned(), "Warm.XMP".to_owned()];
        assert_eq!(lowercase_extension(&parts).as_deref(), Some("xmp"));
        let trailing_dot = vec!["Warm.xmp. ".to_owned()];
        assert_eq!(lowercase_extension(&trailing_dot).as_deref(), Some("xmp"));
    }
}
