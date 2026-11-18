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
}

/// Der Papierkorb des Betriebssystems.
#[derive(Debug, Clone, Copy, Default)]
pub struct RecycleBin;

impl Trash for RecycleBin {
    fn move_to_trash(&self, path: &Path) -> Result<(), CoreError> {
        trash::delete(path).map_err(|error| CoreError::Trash {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })
    }
}
