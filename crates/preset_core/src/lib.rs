//! Logik des Preset Managers.
//!
//! Diese Bibliothek weiß nichts von Fenstern und Schaltflächen. Sie findet die
//! Adobe-Ordner, liest Presets, importiert neue und verwaltet vorhandene. Die
//! Oberfläche (`preset_manager`) ruft sie nur auf.
//!
//! # Module
//!
//! | Modul | Aufgabe |
//! |---|---|
//! | [`adobe`] | Wo liegen die Adobe-Ordner? |
//! | [`preset`] | Was ist ein Preset? |
//! | [`import`] | Presets aus Ordner oder ZIP installieren |
//! | [`library`] | Installierte Presets einlesen und verwalten |
//! | [`safe_fs`] | Dateien sicher schreiben, sichern und löschen |
//! | [`report`] | Ergebnis einer Aktion |
//! | [`preview`] | Vorher/Nachher: ein Foto mit den Werten eines Presets nachrechnen (Annäherung) |
//! | [`error`] | Fehlerarten |
//!
//! Die Planung dazu steht in `Planung/03-Architektur.md`.

// In Tests dürfen Abkürzungen benutzt werden, die im Programmcode verboten sind.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod adobe;
pub mod error;
pub mod import;
pub mod library;
pub mod preset;
pub mod preview;
pub mod report;
pub mod safe_fs;

pub use error::{ArchiveLimit, CoreError, NameProblem};
