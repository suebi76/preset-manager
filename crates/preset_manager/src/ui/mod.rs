//! Die Oberfläche – je Bereich des Fensters ein Modul.
//!
//! ```text
//! ┌ topbar ─────────────────────────────────────────────┐
//! ├ groups ──┬ list ───────────────────────┬ details ───┤
//! │          │                             │            │
//! ├──────────┴─────────────────────────────┴────────────┤
//! └ overlays::hint_bar ─────────────────────────────────┘
//!   + import_dialog, confirm_dialog, settings_dialog,
//!     overlays (Ablegen, Einblendungen)
//! ```
//!
//! Die Module zeichnen nur und ändern den Zustand in [`crate::app::App`];
//! Dateien fassen sie nie an.

pub mod before_after;
pub mod confirm_dialog;
pub mod details;
pub mod groups;
pub mod import_dialog;
pub mod list;
pub mod overlays;
pub mod preset_view;
pub mod settings_dialog;
pub mod theme;
pub mod topbar;
pub mod widgets;
