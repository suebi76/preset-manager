//! Was ein Preset ist und wie es auf der Platte aussieht.
//!
//! | Modul | Aufgabe |
//! |---|---|
//! | [`model`] | Datentypen |
//! | [`xmp`] | Kennfelder aus einer Preset-Datei lesen |
//! | [`xmp_edit`] | Name, Gruppe, Sortierschlüssel und Kennung gezielt ändern, ohne den Rest der Datei anzufassen |

pub mod model;
pub mod xmp;
pub mod xmp_edit;

pub use model::{Preset, PresetKind, PresetOrigin, PresetType, Section};
pub use xmp::{metadata_from_bytes, read_metadata, PresetMetadata};
pub use xmp_edit::{replace_uuid, rewrite_metadata, MetadataChanges};
