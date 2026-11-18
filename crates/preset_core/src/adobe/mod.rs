//! Alles, was mit den Adobe-Programmen selbst zu tun hat.
//!
//! - [`locations`]: Wo liegen die Preset-Ordner?
//! - [`lightroom`]: Welche Kataloge kennt Lightroom Classic?
//! - [`running`]: Läuft gerade Photoshop oder Lightroom?

pub mod lightroom;
pub mod locations;
pub mod running;

pub use locations::{AdobeLocations, FolderKind, FoundFolder, SearchRoots};
pub use running::{running_adobe_apps, AdobeApp};
