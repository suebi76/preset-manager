//! Installierte Presets einlesen und verwalten.
//!
//! - [`scan`]: [`Library::load`] liest alle Presets aus den Adobe-Ordnern und
//!   ordnet sie nach Gruppen.
//! - [`ops`]: löschen, umbenennen, in andere Gruppe verschieben, Reihenfolge
//!   ändern.

pub mod ops;
pub mod scan;

pub use ops::Context;
pub use scan::{Library, PresetGroup};
