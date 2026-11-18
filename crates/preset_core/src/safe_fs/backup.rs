//! Sicherungen vor dem Überschreiben.
//!
//! Jede Aktion, die vorhandene Dateien ersetzt, bekommt einen eigenen
//! Sicherungsordner mit Zeitstempel, z. B.
//! `%LOCALAPPDATA%\PresetManager\backups\2026-11-17_184512\`. Darin liegen
//! die Originale mit ihrem Pfad relativ zum Camera-Raw-Ordner
//! (`Settings\Paket\Warm.xmp`). So lässt sich jede Datei von Hand
//! zurückkopieren.
//!
//! Der Ordner wird erst angelegt, wenn wirklich etwas gesichert wird. Eine
//! Aktion ohne Überschreiben hinterlässt also keinen leeren Ordner.

use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::CoreError;

/// Der übliche Ort für Sicherungen:
/// `%LOCALAPPDATA%\PresetManager\backups`.
///
/// `None`, wenn die Umgebungsvariable fehlt oder leer ist.
pub fn default_backup_root() -> Option<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").filter(|value| !value.is_empty())?;
    Some(
        PathBuf::from(local_app_data)
            .join("PresetManager")
            .join("backups"),
    )
}

/// Die Sicherungen einer einzelnen Aktion.
#[derive(Debug)]
pub struct BackupSession {
    root: PathBuf,
    folder: Option<PathBuf>,
}

impl BackupSession {
    /// Bereitet eine Sicherung unterhalb von `root` vor. Angelegt wird noch
    /// nichts.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            folder: None,
        }
    }

    /// Der Sicherungsordner dieser Aktion – `None`, solange nichts gesichert
    /// wurde.
    pub fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    /// Kopiert `original` in den Sicherungsordner, unter dem Pfad `relative`.
    ///
    /// Wird dieselbe Datei in einer Aktion zweimal gesichert, bleibt die erste
    /// Sicherung erhalten – sie enthält den Zustand vor der Aktion.
    ///
    /// # Fehler
    ///
    /// [`CoreError::Io`], wenn `relative` kein einfacher relativer Pfad ist
    /// oder Anlegen bzw. Kopieren scheitert.
    pub fn back_up(&mut self, original: &Path, relative: &Path) -> Result<PathBuf, CoreError> {
        if !is_plain_relative(relative) {
            return Err(CoreError::Io {
                path: relative.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Sicherungspfad muss relativ sein und darf kein „..“ enthalten",
                ),
            });
        }

        let destination = self.ensure_folder()?.join(relative);
        if destination.exists() {
            return Ok(destination);
        }
        if let Some(parent) = destination.parent() {
            create_dir_all(parent)?;
        }
        std::fs::copy(original, &destination).map_err(|source| CoreError::Io {
            path: original.to_path_buf(),
            source,
        })?;
        Ok(destination)
    }

    /// Legt den Sicherungsordner beim ersten Aufruf an.
    fn ensure_folder(&mut self) -> Result<PathBuf, CoreError> {
        if let Some(folder) = &self.folder {
            return Ok(folder.clone());
        }
        create_dir_all(&self.root)?;

        let stamp = timestamp(SystemTime::now());
        // Zwei Aktionen in derselben Sekunde bekommen „-2“, „-3“ … angehängt.
        for attempt in 1..=100 {
            let name = if attempt == 1 {
                stamp.clone()
            } else {
                format!("{stamp}-{attempt}")
            };
            let candidate = self.root.join(name);
            match std::fs::create_dir(&candidate) {
                Ok(()) => {
                    self.folder = Some(candidate.clone());
                    return Ok(candidate);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(source) => {
                    return Err(CoreError::Io {
                        path: candidate,
                        source,
                    })
                }
            }
        }
        Err(CoreError::Io {
            path: self.root.clone(),
            source: std::io::Error::other("kein freier Name für den Sicherungsordner"),
        })
    }
}

/// So viele Sicherungsordner bleiben erhalten (`Planung/04-Fehlerbehandlung.md`).
pub const BACKUPS_TO_KEEP: usize = 20;

/// Entfernt alte Sicherungsordner unter `root`, sodass höchstens `keep`
/// übrig bleiben – die neuesten. Gibt die Anzahl der entfernten Ordner zurück.
///
/// Angefasst werden nur Ordner, deren Name wie ein Sicherungsordner aussieht
/// (`2026-11-17_184512`, `2026-11-17_184512-2`). Alles andere in `root` bleibt
/// liegen. Fehlt `root`, gibt es nichts zu tun.
///
/// Sicherungen sind Kopien, die die App selbst angelegt hat; sie werden
/// deshalb endgültig gelöscht, nicht in den Papierkorb gelegt.
///
/// # Fehler
///
/// [`CoreError::Io`], wenn `root` nicht gelesen oder ein Ordner nicht
/// entfernt werden kann. Bereits entfernte Ordner bleiben entfernt.
pub fn prune_backups(root: &Path, keep: usize) -> Result<usize, CoreError> {
    let io_error = |path: &Path, source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    };
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(io_error(root, error)),
    };

    let mut folders: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| io_error(root, error))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_folder = entry.file_type().is_ok_and(|kind| kind.is_dir());
        if is_folder && is_backup_folder_name(&name) {
            folders.push((name, entry.path()));
        }
    }

    // Neueste zuerst: nach Zeitstempel, dann nach der Laufnummer als Zahl
    // (sonst stünde „-10“ vor „-2“).
    folders.sort_by_cached_key(|(name, _)| std::cmp::Reverse(backup_sort_key(name)));
    let mut removed = 0;
    for (_name, path) in folders.into_iter().skip(keep) {
        std::fs::remove_dir_all(&path).map_err(|error| io_error(&path, error))?;
        removed += 1;
    }
    Ok(removed)
}

/// Sortierschlüssel eines Sicherungsordners: Zeitstempel und Laufnummer
/// (ohne Nummer = 1).
fn backup_sort_key(name: &str) -> (String, u64) {
    let (stamp, suffix) = name.split_at_checked(17).unwrap_or((name, ""));
    let number = suffix
        .strip_prefix('-')
        .and_then(|number| number.parse().ok())
        .unwrap_or(1);
    (stamp.to_owned(), number)
}

/// `true` für `2026-11-17_184512` und `2026-11-17_184512-2`.
fn is_backup_folder_name(name: &str) -> bool {
    let (stamp, suffix) = name.split_at_checked(17).unwrap_or((name, ""));
    let pattern_matches = stamp.len() == 17
        && stamp.char_indices().all(|(index, character)| match index {
            4 | 7 => character == '-',
            10 => character == '_',
            _ => character.is_ascii_digit(),
        });
    let suffix_ok = suffix.is_empty()
        || suffix
            .strip_prefix('-')
            .is_some_and(|number| !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()));
    pattern_matches && suffix_ok
}

fn create_dir_all(folder: &Path) -> Result<(), CoreError> {
    std::fs::create_dir_all(folder).map_err(|source| CoreError::Io {
        path: folder.to_path_buf(),
        source,
    })
}

/// `true` für Pfade wie `Settings\Paket\a.xmp` – ohne Laufwerk, ohne
/// führenden Schrägstrich, ohne `..`.
fn is_plain_relative(path: &Path) -> bool {
    path.components().next().is_some()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Zeitstempel für den Ordnernamen, z. B. `2026-11-17_184512`.
///
/// Verwendet wird UTC, weil die Standardbibliothek die lokale Zeitzone nicht
/// kennt. Für die Sortierung der Ordner ist das egal.
fn timestamp(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let (year, month, day) = civil_date(seconds / 86_400);
    let second_of_day = seconds % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}_{:02}{:02}{:02}",
        second_of_day / 3600,
        second_of_day % 3600 / 60,
        second_of_day % 60
    )
}

/// Rechnet Tage seit dem 1.1.1970 in ein Kalenderdatum um.
///
/// Verfahren nach Howard Hinnant, „chrono-Compatible Low-Level Date
/// Algorithms“. Gilt für alle Daten ab 1970.
fn civil_date(days_since_epoch: u64) -> (u64, u64, u64) {
    let shifted = days_since_epoch + 719_468; // Tage seit dem 1.3.0000
    let era = shifted / 146_097; // 400-Jahres-Abschnitte
    let day_of_era = shifted % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn rechnet_datum_richtig_um() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(11_016), (2000, 2, 29));
        assert_eq!(civil_date(20_774), (2026, 11, 17));
    }

    #[test]
    fn zeitstempel_hat_festes_format() {
        let time = UNIX_EPOCH + Duration::from_secs(20_774 * 86_400 + 18 * 3600 + 45 * 60 + 12);
        assert_eq!(timestamp(time), "2026-11-17_184512");
    }

    #[test]
    fn sichert_mit_relativem_pfad_und_legt_ordner_erst_bei_bedarf_an() {
        let workspace = tempfile::tempdir().unwrap();
        let original = workspace.path().join("Warm.xmp");
        std::fs::write(&original, b"Original").unwrap();
        let root = workspace.path().join("backups");
        let mut session = BackupSession::new(&root);
        assert!(session.folder().is_none());
        assert!(!root.exists());

        let copy = session
            .back_up(&original, Path::new("Settings/Paket/Warm.xmp"))
            .unwrap();

        assert_eq!(std::fs::read(&copy).unwrap(), b"Original");
        assert!(copy.starts_with(session.folder().unwrap()));
        assert!(copy.ends_with("Settings/Paket/Warm.xmp"));
    }

    #[test]
    fn erste_sicherung_bleibt_erhalten() {
        let workspace = tempfile::tempdir().unwrap();
        let original = workspace.path().join("Warm.xmp");
        let mut session = BackupSession::new(workspace.path().join("backups"));
        std::fs::write(&original, b"vorher").unwrap();
        session.back_up(&original, Path::new("Warm.xmp")).unwrap();

        std::fs::write(&original, b"zwischendurch").unwrap();
        let copy = session.back_up(&original, Path::new("Warm.xmp")).unwrap();

        assert_eq!(std::fs::read(copy).unwrap(), b"vorher");
    }

    #[test]
    fn zwei_aktionen_bekommen_getrennte_ordner() {
        let workspace = tempfile::tempdir().unwrap();
        let original = workspace.path().join("a.xmp");
        std::fs::write(&original, b"a").unwrap();
        let root = workspace.path().join("backups");

        let mut first = BackupSession::new(&root);
        let mut second = BackupSession::new(&root);
        first.back_up(&original, Path::new("a.xmp")).unwrap();
        second.back_up(&original, Path::new("a.xmp")).unwrap();

        assert_ne!(first.folder(), second.folder());
    }

    #[test]
    fn erkennt_namen_von_sicherungsordnern() {
        assert!(is_backup_folder_name("2026-11-17_184512"));
        assert!(is_backup_folder_name("2026-11-17_184512-12"));
        assert!(!is_backup_folder_name("2026-11-17"));
        assert!(!is_backup_folder_name("Meine Fotos"));
        assert!(!is_backup_folder_name("2026-11-17_184512-"));
        assert!(!is_backup_folder_name("2026-11-17_184512x"));
        assert!(!is_backup_folder_name("2026_11_17-184512"));
    }

    #[test]
    fn behaelt_nur_die_neuesten_sicherungen() {
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path();
        for name in [
            "2026-11-15_100000",
            "2026-11-16_100000",
            "2026-11-17_100000",
            "2026-11-17_100000-2",
            "Fremder Ordner",
        ] {
            std::fs::create_dir_all(root.join(name).join("Settings")).unwrap();
        }
        std::fs::write(root.join("notiz.txt"), b"x").unwrap();

        let removed = prune_backups(root, 2).unwrap();

        assert_eq!(removed, 2);
        let mut left: Vec<String> = std::fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "2026-11-17_100000",
                "2026-11-17_100000-2",
                "Fremder Ordner",
                "notiz.txt"
            ]
        );
    }

    #[test]
    fn laufnummern_werden_als_zahl_sortiert() {
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path();
        for name in [
            "2026-11-17_100000",
            "2026-11-17_100000-2",
            "2026-11-17_100000-10",
        ] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }

        prune_backups(root, 1).unwrap();

        assert!(root.join("2026-11-17_100000-10").exists(), "neueste bleibt");
        assert!(!root.join("2026-11-17_100000-2").exists());
    }

    #[test]
    fn aufraeumen_ohne_sicherungsordner_ist_kein_fehler() {
        let workspace = tempfile::tempdir().unwrap();

        assert_eq!(
            prune_backups(&workspace.path().join("fehlt"), 20).unwrap(),
            0
        );
    }

    #[test]
    fn lehnt_pfade_ausserhalb_ab() {
        let workspace = tempfile::tempdir().unwrap();
        let original = workspace.path().join("a.xmp");
        std::fs::write(&original, b"a").unwrap();
        let mut session = BackupSession::new(workspace.path().join("backups"));

        for relative in ["../a.xmp", "", "C:/a.xmp", "/a.xmp"] {
            let result = session.back_up(&original, Path::new(relative));
            assert!(
                matches!(result, Err(CoreError::Io { .. })),
                "{relative} hätte abgelehnt werden müssen"
            );
        }
    }
}
