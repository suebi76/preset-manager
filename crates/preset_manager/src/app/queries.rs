//! Abfragen für die Darstellung: was die Liste zeigt, welche Gruppen es
//! gibt, was ausgewählt ist – und dass Ort und Auswahl nach dem Einlesen
//! noch gültig sind.

use std::collections::BTreeSet;
use std::path::PathBuf;

use preset_core::preset::{Preset, Section};

use super::{App, Place};

impl App {
    pub fn place_section(&self) -> Option<Section> {
        match &self.place {
            Some(Place::Group { section, .. }) => Some(*section),
            _ => None,
        }
    }

    pub fn searching(&self) -> bool {
        !self.search.trim().is_empty()
    }

    /// Die Einträge, die die Liste in der Mitte zeigt: Suchtreffer oder die
    /// Einträge der gewählten Gruppe.
    pub fn listed(&self) -> Vec<&Preset> {
        let Some(library) = &self.library else {
            return Vec::new();
        };
        let query = self.search.trim().to_lowercase();
        if !query.is_empty() {
            return library
                .groups()
                .iter()
                .filter(|group| self.filter.shows(group.section))
                .flat_map(|group| &group.presets)
                .filter(|preset| preset.name.to_lowercase().contains(&query))
                .collect();
        }
        match &self.place {
            Some(Place::Group { section, name }) => {
                self.group_presets(*section, name, self.place_active)
            }
            _ => Vec::new(),
        }
    }

    /// Die Einträge einer Gruppe in einem Zustand (aktiv/deaktiviert).
    pub fn group_presets(&self, section: Section, name: &str, active: bool) -> Vec<&Preset> {
        self.library
            .as_ref()
            .and_then(|library| library.group(section, name))
            .map(|group| {
                group
                    .presets
                    .iter()
                    .filter(|preset| preset.active == active)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Die Gruppen eines Bereichs, die Einträge in diesem Zustand haben –
    /// bei den aktiven auch die neuen, leeren Gruppen.
    pub fn group_names_in_state(&self, section: Section, active: bool) -> Vec<String> {
        self.group_names(section)
            .into_iter()
            .filter(|name| {
                (active && self.is_empty_group(section, name))
                    || !self.group_presets(section, name, active).is_empty()
            })
            .collect()
    }

    /// Gibt es deaktivierte Einträge (in den angezeigten Bereichen)?
    pub fn has_inactive(&self) -> bool {
        self.all_presets()
            .any(|preset| !preset.active && self.filter.shows(preset.section()))
    }

    /// Alle Einträge der Bibliothek.
    pub fn all_presets(&self) -> impl Iterator<Item = &Preset> {
        self.library
            .iter()
            .flat_map(|library| library.groups())
            .flat_map(|group| &group.presets)
    }

    /// Die ausgewählten Einträge in Listenreihenfolge.
    pub fn selected_presets(&self) -> Vec<Preset> {
        self.all_presets()
            .filter(|preset| self.selection.contains(&preset.path))
            .cloned()
            .collect()
    }

    /// Der einzige ausgewählte Eintrag, falls genau einer gewählt ist.
    pub fn single_selected(&self) -> Option<&Preset> {
        if self.selection.len() != 1 {
            return None;
        }
        let path = self.selection.iter().next()?;
        self.all_presets().find(|preset| &preset.path == path)
    }

    /// Gibt es diese Gruppe – in den Dateien oder als neue, leere Gruppe?
    pub fn group_exists(&self, section: Section, name: &str) -> bool {
        self.is_empty_group(section, name)
            || self
                .library
                .as_ref()
                .is_some_and(|library| library.group(section, name).is_some())
    }

    pub fn is_empty_group(&self, section: Section, name: &str) -> bool {
        self.empty_groups
            .iter()
            .any(|(s, n)| *s == section && n == name)
    }

    /// Alle Gruppennamen eines Bereichs, sortiert – für Auswahllisten.
    pub fn group_names(&self, section: Section) -> Vec<String> {
        let mut names: Vec<String> = self
            .library
            .iter()
            .flat_map(|library| library.groups_in(section))
            .map(|group| group.name.clone())
            .chain(
                self.empty_groups
                    .iter()
                    .filter(|(s, _)| *s == section)
                    .map(|(_, name)| name.clone()),
            )
            .collect();
        names.sort_by_key(|name| preset_core::library::scan::collation_key(name));
        names.dedup();
        names
    }

    /// Sind gerade Adobe-Programme offen und der Hinweis nicht weggeklickt?
    pub fn show_running_hint(&self) -> bool {
        !self.adobe_apps.is_empty() && !self.hint_dismissed
    }

    /// Wählt eine sichtbare Gruppe, wenn die bisherige fehlt oder ausgefiltert ist.
    pub(crate) fn ensure_valid_place(&mut self) {
        // Ist die Gruppe im angezeigten Zustand leer geworden (z. B. ganz
        // deaktiviert), aber im anderen Zustand noch da, wechselt die Ansicht.
        if let Some(Place::Group { section, name }) = &self.place {
            let here = self.place_active && self.is_empty_group(*section, name)
                || !self
                    .group_presets(*section, name, self.place_active)
                    .is_empty();
            let there = !self
                .group_presets(*section, name, !self.place_active)
                .is_empty();
            if !here && there {
                self.place_active = !self.place_active;
            }
        }
        let valid = match &self.place {
            Some(Place::Group { section, name }) => {
                self.filter.shows(*section)
                    && self
                        .group_names_in_state(*section, self.place_active)
                        .contains(name)
            }
            Some(Place::Problems) => !self.problems.is_empty(),
            None => false,
        };
        if !valid {
            let first = [true, false].into_iter().find_map(|active| {
                [Section::Presets, Section::Profiles]
                    .into_iter()
                    .filter(|section| self.filter.shows(*section))
                    .find_map(|section| {
                        self.group_names_in_state(section, active)
                            .into_iter()
                            .next()
                            .map(|name| (Place::Group { section, name }, active))
                    })
            });
            match first {
                Some((place, active)) => {
                    self.place = Some(place);
                    self.place_active = active;
                }
                None => {
                    self.place = None;
                    self.place_active = true;
                }
            }
        }
        // Auswahl auf Einträge beschränken, die es noch gibt.
        let existing: BTreeSet<PathBuf> = self
            .all_presets()
            .filter(|preset| self.selection.contains(&preset.path))
            .map(|preset| preset.path.clone())
            .collect();
        self.selection = existing;
    }
}
