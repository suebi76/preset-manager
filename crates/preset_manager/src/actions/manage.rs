//! Verwalten: Aufträge an den Hintergrund-Thread, Rückgängig und
//! Wiederholen, Löschen mit Rückfrage, Umbenennen, neue Gruppen.

use preset_core::preset::{Preset, Section};

use crate::app::{App, Confirm, Filter, Place, Rename, RenameTarget, ToastKind};
use crate::settings;
use crate::texts::{self, tr, tr_format};
use crate::worker::{Command, Manage};

use super::{common_section, same_name};

impl App {
    /// Schickt einen Verwalten-Auftrag, sofern gerade keiner läuft. Gibt
    /// zurück, ob er abgeschickt wurde – nur dann darf der Aufrufer seinen
    /// Folgezustand (Ziel-Gruppe, Umbenennen danach …) setzen.
    pub fn manage(&mut self, task: Manage, now: f64) -> bool {
        self.start_change(Command::Manage(task), now)
    }

    /// Schickt einen Auftrag, der Dateien ändert – nur, wenn gerade keiner
    /// läuft und kein Import offen ist.
    fn start_change(&mut self, command: Command, now: f64) -> bool {
        if self.managing || self.scanning || self.import.is_some() {
            self.toast(texts::wait_for_action().to_owned(), ToastKind::Info, now);
            return false;
        }
        self.managing = true;
        self.send(command);
        true
    }

    // -----------------------------------------------------------------------
    // Rückgängig und Wiederholen (F-29)
    // -----------------------------------------------------------------------

    /// Strg+Z: die letzte Aktion zurücknehmen.
    pub fn undo(&mut self, now: f64) {
        if self.undo_label.is_some() && self.start_change(Command::Undo, now) {
            // Der Knopf in der Einblendung gälte sonst der schon
            // zurückgenommenen Aktion.
            for toast in &mut self.toasts {
                toast.undo = false;
            }
        }
    }

    /// Strg+Y: die zuletzt zurückgenommene Aktion wiederholen.
    pub fn redo(&mut self, now: f64) {
        if self.redo_label.is_some() {
            self.start_change(Command::Redo, now);
        }
    }

    /// Entf: Rückfrage zum Löschen der Auswahl (F-22).
    pub fn request_delete_selection(&mut self) {
        let presets = self.selected_presets();
        if !presets.is_empty() {
            self.confirm = Some(Confirm::DeletePresets(presets));
        }
    }

    /// Gruppe löschen (F-23) – in der angezeigten Ansicht (aktiv oder
    /// deaktiviert).
    pub fn request_delete_group(&mut self, section: Section, name: &str, now: f64) {
        self.request_delete_group_in(section, name, self.place_active, now);
    }

    /// Gruppe löschen: nur die Einträge im Zustand `active`. Leere, neue
    /// Gruppen verschwinden sofort.
    pub fn request_delete_group_in(
        &mut self,
        section: Section,
        name: &str,
        active: bool,
        now: f64,
    ) {
        if self.is_empty_group(section, name) {
            self.empty_groups
                .retain(|(s, n)| !(*s == section && n == name));
            let _ = settings::save_empty_groups(&self.empty_groups);
            self.toast(
                tr_format!(
                    "Leere Gruppe „{name}“ entfernt",
                    "Empty group “{name}” removed"
                ),
                ToastKind::Info,
                now,
            );
            self.ensure_valid_place();
            return;
        }
        let presets: Vec<Preset> = self
            .group_presets(section, name, active)
            .into_iter()
            .cloned()
            .collect();
        self.confirm = Some(Confirm::DeleteGroup {
            name: name.to_owned(),
            count: presets.len(),
            presets,
        });
    }

    /// Die Rückfrage wurde bestätigt.
    pub fn confirm_accepted(&mut self, now: f64) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };
        match confirm {
            Confirm::DeletePresets(presets) => {
                // Auswahl erst leeren, wenn der Auftrag wirklich läuft.
                if self.manage(Manage::Delete(presets), now) {
                    self.selection.clear();
                }
            }
            Confirm::DeleteGroup { presets, .. } => {
                self.manage(Manage::Delete(presets), now);
            }
            Confirm::Merge {
                section,
                target,
                presets,
                ..
            } => {
                if presets.is_empty() {
                    self.toast(
                        tr(
                            "Nichts zusammenzuführen – alle Einträge gibt es in der Zielgruppe schon.",
                            "Nothing to merge – all items already exist in the target group.",
                        )
                        .to_owned(),
                        ToastKind::Info,
                        now,
                    );
                    return;
                }
                let move_task = Manage::Move {
                    presets,
                    group: target.clone(),
                };
                if self.manage(move_task, now) {
                    self.landing = Some(Place::Group {
                        section,
                        name: target,
                    });
                }
            }
        }
    }

    /// F2 / Doppelklick auf einen Eintrag (F-24).
    pub fn start_rename_preset(&mut self, preset: &Preset) {
        self.rename = Some(Rename {
            target: RenameTarget::Preset(preset.path.clone()),
            text: preset.name.clone(),
            focus: true,
        });
    }

    /// F2 / Doppelklick auf eine Gruppe (F-25).
    pub fn start_rename_group(&mut self, section: Section, name: &str) {
        self.rename = Some(Rename {
            target: RenameTarget::Group {
                section,
                name: name.to_owned(),
            },
            text: name.to_owned(),
            focus: true,
        });
    }

    /// Enter oder Klick daneben: Umbenennen übernehmen.
    pub fn commit_rename(&mut self, now: f64) {
        let Some(rename) = self.rename.take() else {
            return;
        };
        let text = rename.text.trim().to_owned();
        match rename.target {
            RenameTarget::Preset(path) => {
                let Some(preset) = self.all_presets().find(|p| p.path == path).cloned() else {
                    return;
                };
                if text == preset.name {
                    return;
                }
                if text.is_empty() {
                    self.toast(
                        tr(
                            "Der Name darf nicht leer sein.",
                            "The name must not be empty.",
                        )
                        .to_owned(),
                        ToastKind::Error,
                        now,
                    );
                    return;
                }
                self.manage(
                    Manage::RenamePreset {
                        preset,
                        new_name: text,
                    },
                    now,
                );
            }
            RenameTarget::Group { section, name } => {
                self.commit_group_rename(section, &name, text, now)
            }
        }
    }

    fn commit_group_rename(&mut self, section: Section, old: &str, new: String, now: f64) {
        if new == old {
            return;
        }
        if new.is_empty() {
            self.toast(
                tr(
                    "Der Gruppenname darf nicht leer sein.",
                    "The group name must not be empty.",
                )
                .to_owned(),
                ToastKind::Error,
                now,
            );
            return;
        }
        if self.group_exists(section, &new) {
            self.toast(
                tr_format!(
                    "Die Gruppe „{new}“ gibt es schon – zum Zusammenführen die Gruppe darauf ziehen.",
                    "The group “{new}” already exists – drag the group onto it to merge."
                ),
                ToastKind::Error,
                now,
            );
            return;
        }
        if self.is_empty_group(section, old) {
            for entry in &mut self.empty_groups {
                if entry.0 == section && entry.1 == old {
                    entry.1.clone_from(&new);
                }
            }
            let _ = settings::save_empty_groups(&self.empty_groups);
            self.place = Some(Place::Group { section, name: new });
            return;
        }
        let task = Manage::RenameGroup {
            section,
            old: old.to_owned(),
            new: new.clone(),
        };
        if self.manage(task, now) {
            self.landing = Some(Place::Group { section, name: new });
        }
    }

    /// „＋“: neue, leere Gruppe anlegen und gleich umbenennen (F-33).
    pub fn create_empty_group(&mut self, section: Section) {
        let name = self.unique_group_name(section);
        self.empty_groups.push((section, name.clone()));
        let _ = settings::save_empty_groups(&self.empty_groups);
        if !self.filter.shows(section) {
            self.filter = Filter::All;
        }
        self.select_place(Place::Group {
            section,
            name: name.clone(),
        });
        self.start_rename_group(section, &name);
    }

    fn unique_group_name(&self, section: Section) -> String {
        let base = texts::new_group();
        let mut name = base.to_owned();
        let mut number = 2;
        while self
            .group_names(section)
            .iter()
            .any(|existing| same_name(existing, &name))
        {
            name = format!("{base} {number}");
            number += 1;
        }
        name
    }

    /// Strg+G: neue Gruppe aus der Auswahl (F-33).
    pub fn group_from_selection(&mut self, now: f64) {
        let presets = self.selected_presets();
        if presets.is_empty() {
            return;
        }
        let Some(section) = common_section(&presets) else {
            self.toast(texts::mixed_sections().to_owned(), ToastKind::Error, now);
            return;
        };
        self.move_into_new_group(presets, section, now);
    }

    /// Verschiebt in eine neu benannte Gruppe und lässt sie danach umbenennen.
    pub(super) fn move_into_new_group(&mut self, presets: Vec<Preset>, section: Section, now: f64) {
        let name = self.unique_group_name(section);
        let task = Manage::Move {
            presets,
            group: name.clone(),
        };
        if self.manage(task, now) {
            self.selection.clear();
            self.rename_after_load = Some((section, name.clone()));
            self.landing = Some(Place::Group { section, name });
        }
    }
}
