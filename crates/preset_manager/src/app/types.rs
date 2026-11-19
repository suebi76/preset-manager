//! Die Bausteine des Zustands: gewählte Stelle, Filter, Umbenennen,
//! Rückfragen, Ziehen, Einblendungen.

use std::path::PathBuf;

use preset_core::import::{ConflictPolicy, ImportPlan};
use preset_core::preset::{Preset, Section};

/// Woher der Camera-Raw-Ordner kommt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderSource {
    /// Unter `%APPDATA%\Adobe\CameraRaw` gefunden.
    Automatic,
    /// Im Einstellungsdialog gewählt.
    Chosen,
    /// Über [`CUSTOM_DIR_VARIABLE`] vorgegeben.
    Environment,
}

/// Welche Bereiche angezeigt werden (Strg+1/2/3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Presets,
    Profiles,
}

impl Filter {
    pub fn shows(self, section: Section) -> bool {
        match self {
            Self::All => true,
            Self::Presets => section == Section::Presets,
            Self::Profiles => section == Section::Profiles,
        }
    }
}

/// Was links ausgewählt ist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Group {
        section: Section,
        name: String,
    },
    /// Die Liste der Dateien, die nicht gelesen werden konnten.
    Problems,
}

/// Der offene Importdialog.
pub struct ImportDialog {
    /// `None`, während der Plan im Hintergrund ausgeführt wird.
    pub plan: Option<Box<ImportPlan>>,
    /// Quellen, die abgelehnt wurden, mit Begründung.
    pub rejected: Vec<(PathBuf, String)>,
    pub policy: ConflictPolicy,
}

/// Was gerade umbenannt wird (F2 oder Doppelklick).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameTarget {
    Preset(PathBuf),
    Group { section: Section, name: String },
}

pub struct Rename {
    pub target: RenameTarget,
    /// Der Text im Eingabefeld.
    pub text: String,
    /// Beim ersten Bild Fokus setzen und Text markieren.
    pub focus: bool,
}

/// Rückfragen vor Aktionen, die sich nicht von selbst erklären.
pub enum Confirm {
    DeletePresets(Vec<Preset>),
    /// Die Einträge einer Gruppe – nur die des angezeigten Zustands (aktiv
    /// oder deaktiviert).
    DeleteGroup {
        name: String,
        count: usize,
        presets: Vec<Preset>,
    },
    Merge {
        section: Section,
        source: String,
        target: String,
        /// Einträge, die verschoben werden (ohne Namenskonflikte).
        presets: Vec<Preset>,
        /// Einträge, die wegen gleichen Namens stehen bleiben.
        clashes: usize,
    },
}

/// Was beim Ziehen innerhalb des Fensters getragen wird.
#[derive(Debug, Clone)]
pub enum DragPayload {
    Items(Vec<Preset>),
    Group { section: Section, name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Info,
    Error,
}

pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub until: f64,
    /// Zeigt „Rückgängig“ in der Einblendung (nur bei der jüngsten Aktion).
    pub undo: bool,
}
