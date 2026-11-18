//! Alles, was mit den Adobe-Programmen selbst zu tun hat.
//!
//! - [`locations`]: Wo liegen die Preset-Ordner?
//! - [`running`]: Läuft gerade Photoshop oder Lightroom?

pub mod locations;
pub mod running;

pub use locations::AdobeLocations;
pub use running::{running_adobe_apps, AdobeApp};
