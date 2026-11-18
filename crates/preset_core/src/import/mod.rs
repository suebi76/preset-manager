//! Presets aus einem Ordner oder ZIP installieren.
//!
//! Der Ablauf ist immer derselbe:
//!
//! ```text
//! Ordner / ZIP ──scan()──▶ ScannedSource ──ImportPlan::new()──▶ ImportPlan ──execute()──▶ OperationReport
//!                (liest)                     (liest, Vorschau)               (schreibt)
//! ```
//!
//! | Modul | Aufgabe |
//! |---|---|
//! | [`source`] | Ordner und ZIP einheitlich nach Presets durchsuchen |
//! | [`archive`] | ZIP sicher in einen temporären Ordner entpacken |
//! | [`plan`] | Plan erstellen (was wohin, welche Konflikte) und ausführen |
//! | [`naming`] | gültige Windows-Datei- und Ordnernamen bilden |
//!
//! Installiert wird nur nach `CameraRaw\Settings`, nie nach
//! `ImportedSettings` – der Ordner gehört Adobes eigenem Importdialog
//! (Entscheidung E-4 in `Planung/03-Architektur.md`).

pub mod archive;
pub mod naming;
pub mod plan;
pub mod source;

pub use archive::ArchiveLimits;
pub use plan::{ConflictPolicy, ImportPlan, PlanSummary, PlannedAction, PlannedImport};
pub use source::{scan, scan_with_limits, FoundPreset, ScannedSource};
