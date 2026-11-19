//! Ziehen & Ablegen (F-34, F-35): in andere Gruppen verschieben,
//! Gruppen zusammenführen, Reihenfolge ändern – mit Begründung, wenn ein
//! Ziel nicht geht.

use std::path::Path;

use preset_core::preset::{Preset, Section};

use crate::app::{App, Confirm, DragPayload, Place, ToastKind};
use crate::texts::{self, tr, tr_format};
use crate::worker::Manage;

use super::{common_section, same_name};

impl App {
    /// Warum diese Einträge nicht in die Gruppe dürfen – `None`, wenn es geht.
    pub fn move_problem(
        &self,
        presets: &[Preset],
        section: Section,
        group: &str,
    ) -> Option<String> {
        self.move_problem_in(presets, section, group, None)
    }

    /// Wie [`Self::move_problem`]; mit `Some(active)` für eine Gruppe in
    /// diesem Zustand – Einträge im anderen Zustand werden dabei umgeschaltet.
    pub fn move_problem_in(
        &self,
        presets: &[Preset],
        section: Section,
        group: &str,
        active: Option<bool>,
    ) -> Option<String> {
        let Some(own_section) = common_section(presets) else {
            return Some(texts::mixed_sections().to_owned());
        };
        if own_section != section {
            return Some(match own_section {
                Section::Profiles => texts::profile_into_preset_group().to_owned(),
                Section::Presets => texts::preset_into_profile_group().to_owned(),
            });
        }
        if presets
            .iter()
            .all(|p| p.group == group && active.is_none_or(|active| p.active == active))
        {
            return Some(tr("Ist schon in dieser Gruppe", "Already in this group").to_owned());
        }
        let library = self.library.as_ref()?;
        let target = library.group(section, group);
        let clash = presets.iter().find(|moved| {
            moved.group != group
                && target.is_some_and(|target| {
                    target.presets.iter().any(|existing| {
                        existing.path != moved.path && same_name(&existing.name, &moved.name)
                    })
                })
        });
        clash.map(|preset| {
            tr_format!(
                "In „{group}“ gibt es schon „{}“",
                "“{group}” already contains “{}”",
                preset.name
            )
        })
    }

    /// Einträge wurden auf eine Gruppe im Zustand `active` gezogen. Liegen sie
    /// im anderen Zustand, werden sie dabei aktiviert bzw. deaktiviert.
    pub fn drop_on_group(
        &mut self,
        payload: &DragPayload,
        section: Section,
        group: &str,
        active: bool,
        now: f64,
    ) {
        match payload {
            DragPayload::Items(presets) => {
                if let Some(problem) = self.move_problem_in(presets, section, group, Some(active)) {
                    self.toast(problem, ToastKind::Error, now);
                    return;
                }
                let switch = presets.iter().any(|preset| preset.active != active);
                let task = if switch {
                    Manage::SetActive {
                        presets: presets.clone(),
                        active,
                        group: presets
                            .iter()
                            .any(|preset| preset.group != group)
                            .then(|| group.to_owned()),
                    }
                } else {
                    Manage::Move {
                        presets: presets.clone(),
                        group: group.to_owned(),
                    }
                };
                if self.manage(task, now) {
                    self.selection.clear();
                }
            }
            DragPayload::Group {
                section: source_section,
                name: source,
            } => self.request_merge(*source_section, source, section, group, now),
        }
    }

    /// Prüfung für Gruppe-auf-Gruppe – `None`, wenn es geht.
    pub fn merge_problem(
        &self,
        source_section: Section,
        source: &str,
        section: Section,
        target: &str,
    ) -> Option<String> {
        if source_section != section {
            return Some(
                tr(
                    "Preset- und Profil-Gruppen lassen sich nicht zusammenführen",
                    "Preset and profile groups cannot be merged",
                )
                .to_owned(),
            );
        }
        if source == target {
            return Some(String::new());
        }
        None
    }

    fn request_merge(
        &mut self,
        source_section: Section,
        source: &str,
        section: Section,
        target: &str,
        now: f64,
    ) {
        if let Some(problem) = self.merge_problem(source_section, source, section, target) {
            if !problem.is_empty() {
                self.toast(problem, ToastKind::Error, now);
            }
            return;
        }
        let Some(library) = &self.library else {
            return;
        };
        let source_presets: Vec<Preset> = library
            .group(section, source)
            .map(|group| group.presets.clone())
            .unwrap_or_default();
        let target_names: Vec<&str> = library
            .group(section, target)
            .map(|group| group.presets.iter().map(|p| p.name.as_str()).collect())
            .unwrap_or_default();
        let (clashing, movable): (Vec<Preset>, Vec<Preset>) =
            source_presets.into_iter().partition(|preset| {
                target_names
                    .iter()
                    .any(|name| same_name(name, &preset.name))
            });
        self.confirm = Some(Confirm::Merge {
            section,
            source: source.to_owned(),
            target: target.to_owned(),
            presets: movable,
            clashes: clashing.len(),
        });
    }

    /// Einträge wurden auf „＋ Neue Gruppe“ gezogen.
    pub fn drop_on_new_group(&mut self, presets: &[Preset], section: Section, now: f64) {
        match common_section(presets) {
            Some(own) if own == section => {}
            Some(_) => {
                let problem = self.move_problem(presets, section, "").unwrap_or_default();
                self.toast(problem, ToastKind::Error, now);
                return;
            }
            None => {
                self.toast(texts::mixed_sections().to_owned(), ToastKind::Error, now);
                return;
            }
        }
        self.move_into_new_group(presets.to_vec(), section, now);
    }

    /// Einträge wurden innerhalb der Liste abgelegt: neue Reihenfolge (F-27).
    /// `before` ist der Eintrag, vor dem sie landen; `None` heißt „ans Ende“.
    pub fn drop_reorder(&mut self, moved: &[Preset], before: Option<&Path>, now: f64) {
        let Some(Place::Group { section, name }) = self.place.clone() else {
            return;
        };
        let before_order: Vec<Preset> = self
            .group_presets(section, &name, self.place_active)
            .into_iter()
            .cloned()
            .collect();
        if before_order.is_empty() {
            return;
        }
        if moved
            .iter()
            .any(|p| p.group != name || p.section() != section)
        {
            self.toast(
                tr(
                    "Zum Verschieben in eine andere Gruppe auf eine Gruppe links ziehen.",
                    "To move to another group, drag onto a group on the left.",
                )
                .to_owned(),
                ToastKind::Info,
                now,
            );
            return;
        }
        let moved_paths: Vec<&Path> = moved.iter().map(|p| p.path.as_path()).collect();
        let mut order: Vec<Preset> = before_order
            .iter()
            .filter(|p| !moved_paths.contains(&p.path.as_path()))
            .cloned()
            .collect();
        let index = before
            .and_then(|before| order.iter().position(|p| p.path == before))
            .unwrap_or(order.len());
        // Die verschobenen Einträge behalten untereinander ihre Reihenfolge.
        let moved_sorted: Vec<Preset> = before_order
            .iter()
            .filter(|p| moved_paths.contains(&p.path.as_path()))
            .cloned()
            .collect();
        order.splice(index..index, moved_sorted);
        self.send_reorder(&before_order, order, now);
    }

    /// Alt+↑ / Alt+↓: Auswahl um eine Stelle verschieben.
    pub(super) fn nudge(&mut self, up: bool, now: f64) {
        if self.searching() {
            return;
        }
        let Some(Place::Group { section, name }) = self.place.clone() else {
            return;
        };
        let current: Vec<Preset> = self
            .group_presets(section, &name, self.place_active)
            .into_iter()
            .cloned()
            .collect();
        let mut order = current.clone();
        let indices: Vec<usize> = if up {
            (0..order.len()).collect()
        } else {
            (0..order.len()).rev().collect()
        };
        for i in indices {
            let j = if up { i.checked_sub(1) } else { Some(i + 1) };
            let Some(j) = j.filter(|j| *j < order.len()) else {
                continue;
            };
            let (a, b) = (&order[i].path, &order[j].path);
            if self.selection.contains(a) && !self.selection.contains(b) {
                order.swap(i, j);
            }
        }
        self.send_reorder(&current, order, now);
    }

    fn send_reorder(&mut self, before: &[Preset], order: Vec<Preset>, now: f64) {
        let unchanged = before.iter().zip(&order).all(|(a, b)| a.path == b.path);
        if unchanged {
            return;
        }
        self.manage(Manage::Reorder(order), now);
    }
}
