//! Was ein Preset ist und wie es auf der Platte aussieht.
//!
//! | Modul | Aufgabe |
//! |---|---|
//! | [`model`] | Datentypen |
//! | [`xmp`] | Kennfelder aus einer Preset-Datei lesen |
//! | [`xmp_edit`] | Name, Gruppe, Sortierschlüssel und Kennung gezielt ändern, ohne den Rest der Datei anzufassen |
//! | [`settings`] | Alle Camera-Raw-Werte einer Datei lesen |
//! | [`adjustments`] | Die Werte deuten: Bereiche, Kurven, Farben, Aussagen in Worten |

pub mod adjustments;
pub mod model;
pub mod settings;
pub mod xmp;
pub mod xmp_edit;

pub use model::{Preset, PresetKind, PresetOrigin, PresetType, Section};
pub use settings::{read_settings, PresetSettings};
pub use xmp::{metadata_from_bytes, read_metadata, PresetMetadata};
pub use xmp_edit::{replace_uuid, rewrite_metadata, MetadataChanges};
