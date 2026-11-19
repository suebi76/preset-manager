//! Löschen über den Papierkorb.
//!
//! Die App löscht nie endgültig. Jede gelöschte Datei landet im
//! Windows-Papierkorb und lässt sich dort wiederherstellen.
//!
//! Das Löschen steckt hinter der Schnittstelle [`Trash`], damit Tests den
//! echten Papierkorb nicht anfassen: Sie übergeben eine eigene
//! Implementierung, die Dateien nur in einen Testordner verschiebt.

use std::path::Path;

use crate::error::CoreError;

/// Etwas, das Dateien wiederherstellbar entfernen kann.
pub trait Trash {
    /// Verschiebt `path` in den Papierkorb.
    ///
    /// # Fehler
    ///
    /// [`CoreError::Trash`], wenn das nicht möglich ist. Es wird dann
    /// **nicht** ersatzweise endgültig gelöscht.
    fn move_to_trash(&self, path: &Path) -> Result<(), CoreError>;

    /// Holt die zuletzt gelöschte Datei mit dem ursprünglichen Pfad
    /// `original` zurück. `Ok(false)`, wenn es keine solche gibt oder das
    /// Zurückholen hier nicht unterstützt wird – der Aufrufer schreibt die
    /// Datei dann selbst.
    ///
    /// # Fehler
    ///
    /// [`CoreError::Trash`], wenn der Papierkorb nicht gelesen werden kann.
    fn restore(&self, original: &Path) -> Result<bool, CoreError> {
        let _ = original;
        Ok(false)
    }
}

/// Der Papierkorb des Betriebssystems.
///
/// Zum Zurückholen wird sein Inhalt einmal gelesen und dann gemerkt – ein
/// Rückgängig über viele Dateien liest den Papierkorb also nicht für jede
/// Datei neu. Deshalb für jede Aktion einen neuen `RecycleBin` verwenden.
#[derive(Debug, Default)]
pub struct RecycleBin {
    #[cfg(windows)]
    listing: std::cell::RefCell<Option<Vec<trash::TrashItem>>>,
}

impl Trash for RecycleBin {
    fn move_to_trash(&self, path: &Path) -> Result<(), CoreError> {
        trash::delete(path).map_err(|error| CoreError::Trash {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })
    }

    #[cfg(windows)]
    fn restore(&self, original: &Path) -> Result<bool, CoreError> {
        use crate::adobe::locations::same_path;

        let failed = |error: trash::Error| CoreError::Trash {
            path: original.to_path_buf(),
            reason: error.to_string(),
        };
        let mut listing = self.listing.borrow_mut();
        if listing.is_none() {
            *listing = Some(trash::os_limited::list().map_err(failed)?);
        }
        let items = listing.get_or_insert_with(Vec::new);
        let newest = items
            .iter()
            .enumerate()
            .filter(|(_, item)| same_path(&item.original_path(), original))
            .max_by_key(|(_, item)| item.time_deleted)
            .map(|(index, _)| index);
        let Some(index) = newest else {
            return Ok(false);
        };
        let item = items.swap_remove(index);
        trash::os_limited::restore_all([item]).map_err(failed)?;
        Ok(true)
    }
}
