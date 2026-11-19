//! Navigation und Auswahl (F-21): Gruppe wählen, Filter, Klicken mit
//! Strg/Umschalt, Pfeiltasten.

use std::path::{Path, PathBuf};

use eframe::egui;

use crate::app::{App, Filter, Place};

impl App {
    pub fn select_place(&mut self, place: Place) {
        self.select_place_in(place, true);
    }

    /// Wählt eine Gruppe in einem Zustand: ihre aktiven oder ihre
    /// deaktivierten Einträge (F-48).
    pub fn select_place_in(&mut self, place: Place, active: bool) {
        // Ein offenes Umbenennen gehört zur bisherigen Ansicht.
        self.rename = None;
        self.place_active = active;
        self.place = Some(place);
        self.selection.clear();
        self.anchor = None;
        self.search.clear();
    }

    pub fn set_filter(&mut self, filter: Filter) {
        self.filter = filter;
        self.ensure_valid_place();
    }

    /// Klick auf einen Eintrag: allein auswählen, mit Strg dazu/weg, mit
    /// Umschalt einen Bereich.
    pub fn click_item(&mut self, path: &Path, modifiers: egui::Modifiers) {
        let listed: Vec<PathBuf> = self.listed().iter().map(|p| p.path.clone()).collect();
        if modifiers.shift {
            if let Some(anchor) = &self.anchor {
                let a = listed.iter().position(|p| p == anchor);
                let b = listed.iter().position(|p| p == path);
                if let (Some(a), Some(b)) = (a, b) {
                    let (from, to) = if a <= b { (a, b) } else { (b, a) };
                    if !modifiers.command {
                        self.selection.clear();
                    }
                    self.selection.extend(listed[from..=to].iter().cloned());
                    return;
                }
            }
        }
        if modifiers.command {
            if !self.selection.remove(path) {
                self.selection.insert(path.to_path_buf());
            }
        } else {
            self.selection.clear();
            self.selection.insert(path.to_path_buf());
        }
        self.anchor = Some(path.to_path_buf());
    }

    pub fn select_all_listed(&mut self) {
        let listed: Vec<PathBuf> = self.listed().iter().map(|p| p.path.clone()).collect();
        self.selection.extend(listed);
    }

    /// Pfeil hoch/runter: Auswahl wandert, mit Umschalt wird erweitert.
    pub(super) fn step_selection(&mut self, down: bool, extend: bool) {
        let listed: Vec<PathBuf> = self.listed().iter().map(|p| p.path.clone()).collect();
        if listed.is_empty() {
            return;
        }
        let current = self
            .anchor
            .as_ref()
            .and_then(|anchor| listed.iter().position(|p| p == anchor));
        let next = match current {
            Some(index) if down => (index + 1).min(listed.len() - 1),
            Some(index) => index.saturating_sub(1),
            None => 0,
        };
        let Some(path) = listed.get(next).cloned() else {
            return;
        };
        if !extend {
            self.selection.clear();
        }
        self.selection.insert(path.clone());
        self.anchor = Some(path);
    }
}
