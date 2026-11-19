//! Aktivieren und deaktivieren (F-48): einzeln, Auswahl, ganze Gruppe.

use preset_core::preset::{Preset, Section};

use crate::app::{App, ToastKind};
use crate::texts::tr;
use crate::worker::Manage;

impl App {
    /// Aktiviert oder deaktiviert Einträge. Wer schon im Zustand ist, bleibt.
    pub fn set_presets_active(&mut self, presets: Vec<Preset>, active: bool, now: f64) {
        let presets: Vec<Preset> = presets.into_iter().filter(|p| p.active != active).collect();
        if presets.is_empty() {
            let text = if active {
                tr("Schon aktiv.", "Already active.")
            } else {
                tr("Schon deaktiviert.", "Already deactivated.")
            };
            self.toast(text.to_owned(), ToastKind::Info, now);
            return;
        }
        let task = Manage::SetActive {
            presets,
            active,
            group: None,
        };
        if self.manage(task, now) {
            self.selection.clear();
        }
    }

    /// Strg+D: die Auswahl umschalten – sind alle aktiv, werden sie
    /// deaktiviert, sonst aktiviert.
    pub fn toggle_selection_active(&mut self, now: f64) {
        let presets = self.selected_presets();
        if presets.is_empty() {
            return;
        }
        let deactivate = presets.iter().all(|preset| preset.active);
        self.set_presets_active(presets, !deactivate, now);
    }

    /// Alle Einträge einer Gruppe aktivieren oder deaktivieren.
    pub fn set_group_active(&mut self, section: Section, name: &str, active: bool, now: f64) {
        let presets: Vec<Preset> = self
            .group_presets(section, name, !active)
            .into_iter()
            .cloned()
            .collect();
        self.set_presets_active(presets, active, now);
    }
}
