//! Neue Versionen. Wie, hängt von der Fassung ab, für die gebaut wird:
//!
//! | Fassung | Merkmal | Teil |
//! |---|---|---|
//! | GitHub (Setup, portabel) | `self-update` (Standard) | [`github`]: sucht und installiert selbst, über Velopack |
//! | Microsoft Store | ohne Standardmerkmale | [`store`]: sucht nie selbst, der Store aktualisiert |
//!
//! Beide Teile bieten dasselbe `Updater` an; die Oberfläche merkt den
//! Unterschied nur an [`STORE`] und am Status.

#[cfg(feature = "self-update")]
mod github;
#[cfg(not(feature = "self-update"))]
mod store;

#[cfg(feature = "self-update")]
pub use github::Updater;
#[cfg(not(feature = "self-update"))]
pub use store::Updater;

/// Ist das die Fassung für den Microsoft Store?
pub const STORE: bool = cfg!(not(feature = "self-update"));

/// Was der Einstellungsdialog und die Hinweisleiste anzeigen.
// Die Store-Fassung kennt nur `NotInstalled`.
#[cfg_attr(not(feature = "self-update"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Aktualisiert sich nicht selbst: nicht mit dem Installationsprogramm
    /// eingerichtet – oder die Store-Fassung.
    NotInstalled,
    Idle,
    Checking,
    UpToDate,
    Available(String),
    Downloading {
        version: String,
        percent: i16,
    },
    /// Geladen; wird beim nächsten Neustart bzw. Schließen installiert.
    Ready(String),
    Failed(String),
}

/// Was die App tun muss.
#[cfg_attr(not(feature = "self-update"), allow(dead_code))]
pub enum UpdateEvent {
    /// Das Update ist geladen und soll jetzt installiert werden: App beenden.
    RestartNow,
}
