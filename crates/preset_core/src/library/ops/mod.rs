//! Installierte Presets verwalten: löschen, umbenennen, verschieben, sortieren,
//! aktivieren und deaktivieren.
//!
//! Jede Funktion arbeitet in derselben Reihenfolge (`Planung/04-Fehlerbehandlung.md`):
//!
//! 1. **Prüfen** – Namen gültig? Ziel frei? Datei innerhalb der Adobe-Ordner?
//!    Scheitert hier etwas, ist noch nichts verändert.
//! 2. **Sichern** – das Original kommt in den Sicherungsordner.
//! 3. **Ändern** – neuer Inhalt wird atomar geschrieben, gelöscht wird nur in
//!    den Papierkorb.
//!
//! Jede vollzogene Änderung wird in das Protokoll der [`BackupSession`]
//! eingetragen; daraus macht [`crate::history`] die Aktion rückgängig.
//!
//! Betrifft eine Aktion viele Dateien, laufen die übrigen weiter, wenn eine
//! scheitert; der [`OperationReport`] nennt jede einzelne. Danach ist die
//! [`Library`] veraltet und wird neu geladen ([`Library::load`]).
//!
//! Was „Gruppe“ und „Reihenfolge“ bedeuten, steht in den Dateien selbst
//! (`crs:Group`, `crs:SortName`), nicht im Ordner. Umbenennen einer Gruppe
//! oder Verschieben in eine andere Gruppe bewegt deshalb keine Dateien.
//!
//! | Datei | Aktion |
//! |---|---|
//! | `delete` | löschen |
//! | `rename` | Presets und Gruppen umbenennen |
//! | `grouping` | in andere Gruppen verschieben, Reihenfolge |
//! | `activation` | aktivieren und deaktivieren |
//! | `common` | gemeinsame Bausteine |

mod activation;
mod common;
mod delete;
mod grouping;
mod rename;

pub use activation::set_active;
pub use common::same_name;
pub(crate) use common::{read_file, valid_name};
pub(crate) use delete::remove_empty_folders;
pub use delete::{delete, delete_group};
pub use grouping::{move_to_group, reorder};
pub use rename::{rename_group, rename_preset};

use crate::adobe::AdobeLocations;
use crate::library::scan::Library;
use crate::safe_fs::BackupSession;

/// Was eine verändernde Aktion braucht.
pub struct Context<'a> {
    /// Die Adobe-Ordner. Nur darin wird geändert.
    pub locations: &'a AdobeLocations,
    /// Der aktuelle Stand – für die Prüfung auf doppelte Namen.
    pub library: &'a Library,
    /// Wohin Originale vor dem Ändern gesichert werden.
    pub backups: &'a mut BackupSession,
}
