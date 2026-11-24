//! Installierte Presets einlesen und verwalten.
//!
//! - [`scan`]: [`Library::load`] liest alle Presets aus den Adobe-Ordnern und
//!   ordnet sie nach Gruppen.
//! - [`ops`]: löschen, umbenennen, in andere Gruppe verschieben, Reihenfolge
//!   ändern.
//! - [`profiles`]: welche Profile es gibt – für „Profil fehlt“.

pub mod ops;
pub mod profiles;
pub mod scan;

pub use ops::Context;
pub use profiles::{adobe_profile_dirs, profile_status, ProfileIndex, ProfileStatus};
pub use scan::{Library, PresetGroup};
