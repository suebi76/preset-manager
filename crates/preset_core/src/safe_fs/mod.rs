//! Dateioperationen, die keine Daten verlieren.
//!
//! Alle verändernden Zugriffe auf Preset-Dateien laufen über dieses Modul,
//! nie direkt über `std::fs`. So sitzen die Schutzmaßnahmen an einer Stelle.
//!
//! - [`write_atomic`]: unter temporärem Namen schreiben, dann umbenennen –
//!   es entsteht nie eine halb geschriebene Datei
//! - [`BackupSession`]: Original vor dem Überschreiben in den
//!   Sicherungsordner kopieren; [`prune_backups`] behält nur die neuesten
//! - [`Trash`]: in den Papierkorb verschieben, nie endgültig löschen
//! - [`read_preset_file`]: Preset-Dateien nur bis zu einer Obergrenze lesen
//! - [`Journal`]: welche Datei eine Aktion wie geändert hat – Grundlage
//!   für Rückgängig ([`crate::history`])
//!
//! Hintergrund: `Planung/04-Fehlerbehandlung.md`.

pub mod atomic;
pub mod backup;
pub mod journal;
pub mod read;
pub mod trash;

pub use atomic::{move_file, write_atomic, write_to_free_name, WriteMode};
pub use backup::{default_backup_root, prune_backups, BackupSession, BACKUPS_TO_KEEP};
pub use journal::{Change, Journal};
pub use read::{read_preset_file, MAX_PRESET_BYTES};
pub use trash::{RecycleBin, Trash};
