//! Alles, was der Nutzer auslösen kann – Import, Auswahl, Verwalten,
//! Ziehen & Ablegen, Tastenkürzel.
//!
//! Die Funktionen prüfen zuerst, ob die Aktion möglich ist, und geben sonst
//! eine verständliche Begründung als Einblendung aus. Dateien ändern sie nie
//! selbst, sondern schicken einen Auftrag an den Hintergrund-Thread.

use std::path::{Path, PathBuf};

use eframe::egui;
use preset_core::preset::{Preset, Section};

use crate::app::{App, Confirm, DragPayload, Filter, Place, Rename, RenameTarget, ToastKind};
use crate::settings;
use crate::texts::{self, tr, tr_format};
use crate::worker::{Command, Manage};

/// Zwei Namen gelten als gleich, wenn sie sich nur in der Schreibweise
/// unterscheiden (wie in `preset_core::library::ops`).
fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

impl App {
    // -----------------------------------------------------------------------
    // Import
    // -----------------------------------------------------------------------

    /// Durchsucht Ordner/ZIPs und öffnet danach die Importvorschau.
    pub fn start_import(&mut self, paths: Vec<PathBuf>, target: Option<Place>, now: f64) {
        if self.scanning || self.import.is_some() || self.managing {
            self.toast(texts::wait_for_action().to_owned(), ToastKind::Info, now);
            return;
        }
        let target = match target {
            Some(Place::Group { section, name }) if !name.is_empty() => Some((section, name)),
            _ => None,
        };
        self.scanning = true;
        self.send(Command::Scan { paths, target });
    }

    /// Führt den Plan des offenen Importdialogs aus.
    pub fn execute_import(&mut self) {
        let Some(dialog) = &mut self.import else {
            return;
        };
        let Some(plan) = dialog.plan.take() else {
            return;
        };
        self.landing = plan
            .items()
            .iter()
            .find(|item| item.selected)
            .map(|item| Place::Group {
                section: item.preset.metadata.preset_type.section(),
                name: plan.group_after_import(item).to_owned(),
            });
        let policy = dialog.policy;
        // Importiert wird immer in die aktiven Presets.
        self.place_active = true;
        self.send(Command::Execute { plan, policy });
    }

    /// Ordnerauswahl (blockiert, solange der Windows-Dialog offen ist).
    pub fn pick_folder(&mut self, now: f64) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title(texts::import_folder())
            .pick_folder()
        {
            self.start_import(vec![folder], None, now);
        }
    }

    /// ZIP-Auswahl, mehrere Dateien möglich (F-04).
    pub fn pick_zips(&mut self, now: f64) {
        if let Some(files) = rfd::FileDialog::new()
            .set_title(texts::import_zip())
            .add_filter(tr("ZIP-Archive", "ZIP archives"), &["zip"])
            .pick_files()
        {
            self.start_import(files, None, now);
        }
    }

    // -----------------------------------------------------------------------
    // Navigation und Auswahl (F-21)
    // -----------------------------------------------------------------------

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
    fn step_selection(&mut self, down: bool, extend: bool) {
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

    // -----------------------------------------------------------------------
    // Verwalten
    // -----------------------------------------------------------------------

    /// Schickt einen Verwalten-Auftrag, sofern gerade keiner läuft. Gibt
    /// zurück, ob er abgeschickt wurde – nur dann darf der Aufrufer seinen
    /// Folgezustand (Ziel-Gruppe, Umbenennen danach …) setzen.
    pub fn manage(&mut self, task: Manage, now: f64) -> bool {
        if self.managing || self.scanning || self.import.is_some() {
            self.toast(texts::wait_for_action().to_owned(), ToastKind::Info, now);
            return false;
        }
        self.managing = true;
        self.send(Command::Manage(task));
        true
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
    fn move_into_new_group(&mut self, presets: Vec<Preset>, section: Section, now: f64) {
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

    // -----------------------------------------------------------------------
    // Als ZIP exportieren (F-37)
    // -----------------------------------------------------------------------

    /// Fragt nach dem Ziel und speichert die Einträge als ZIP. `name` ist der
    /// vorgeschlagene Dateiname ohne Endung.
    pub fn export_presets(&mut self, presets: Vec<Preset>, name: &str, now: f64) {
        if presets.is_empty() {
            return;
        }
        let suggested = format!(
            "{}.zip",
            preset_core::import::naming::safe_component(name, "Presets")
        );
        // Der Dialog hält das Fenster an; die Zeit dafür zählt nicht zur
        // Anzeigedauer der Einblendung.
        let opened = std::time::Instant::now();
        let Some(target) = rfd::FileDialog::new()
            .set_title(tr("Als ZIP exportieren", "Export as ZIP"))
            .set_file_name(suggested)
            .add_filter(tr("ZIP-Archiv", "ZIP archive"), &["zip"])
            .save_file()
        else {
            return;
        };
        let now = now + opened.elapsed().as_secs_f64();
        let target = if target.extension().is_none() {
            target.with_extension("zip")
        } else {
            target
        };
        let refs: Vec<&Preset> = presets.iter().collect();
        match preset_core::export::export_zip(&refs, &target) {
            Ok(report) => {
                let file = target
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut text = tr_format!(
                    "{} als „{file}“ gespeichert",
                    "{} saved as “{file}”",
                    texts::entries(report.done_count())
                );
                if report.failed_count() > 0 {
                    text.push_str(&tr_format!(
                        " – {} nicht lesbar, ausgelassen",
                        " – {} unreadable, left out",
                        report.failed_count()
                    ));
                }
                tracing::info!(
                    "Als ZIP exportiert: {} Einträge nach {}",
                    report.done_count(),
                    target.display()
                );
                self.toast(text, ToastKind::Success, now);
            }
            Err(error) => self.toast(texts::describe(&error), ToastKind::Error, now),
        }
    }

    /// Strg+E: die Auswahl exportieren – ohne Auswahl die angezeigte Gruppe.
    pub fn export_selection_or_group(&mut self, now: f64) {
        let selected = self.selected_presets();
        if !selected.is_empty() {
            let name = suggested_name(&selected);
            self.export_presets(selected, &name, now);
        } else if let Some(Place::Group { section, name }) = self.place.clone() {
            let presets: Vec<Preset> = self
                .group_presets(section, &name, self.place_active)
                .into_iter()
                .cloned()
                .collect();
            self.export_presets(presets, &name, now);
        }
    }

    // -----------------------------------------------------------------------
    // Aktivieren und deaktivieren (F-48)
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // Ziehen & Ablegen (F-34, F-35)
    // -----------------------------------------------------------------------

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
    fn nudge(&mut self, up: bool, now: f64) {
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

    // -----------------------------------------------------------------------
    // Tastenkürzel (07-Oberflaeche.md)
    // -----------------------------------------------------------------------

    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context, now: f64) {
        use egui::{Key, Modifiers};
        // Nur ein tatsächlich fokussiertes Eingabefeld sperrt die Kürzel. Ein
        // Umbenennen, dessen Feld gerade nicht sichtbar ist (weggescrollt),
        // darf die Tastatur nicht dauerhaft blockieren.
        let typing = ctx.text_edit_focused();
        let keys = ctx.input_mut(|input| {
            let mut take = |modifiers, key| input.consume_key(modifiers, key);
            Keys {
                find: take(Modifiers::COMMAND, Key::F),
                settings: take(Modifiers::COMMAND, Key::Comma),
                all_filter: take(Modifiers::COMMAND, Key::Num1),
                presets: take(Modifiers::COMMAND, Key::Num2),
                profiles: take(Modifiers::COMMAND, Key::Num3),
                zip: take(Modifiers::COMMAND | Modifiers::SHIFT, Key::I),
                select_all: !typing && take(Modifiers::COMMAND, Key::A),
                new_group: !typing && take(Modifiers::COMMAND, Key::G),
                toggle_active: !typing && take(Modifiers::COMMAND, Key::D),
                export: !typing && take(Modifiers::COMMAND, Key::E),
                delete: !typing && take(Modifiers::NONE, Key::Delete),
                rename: !typing && take(Modifiers::NONE, Key::F2),
                nudge_up: !typing && take(Modifiers::ALT, Key::ArrowUp),
                nudge_down: !typing && take(Modifiers::ALT, Key::ArrowDown),
                up: !typing && take(Modifiers::NONE, Key::ArrowUp),
                down: !typing && take(Modifiers::NONE, Key::ArrowDown),
                extend_up: !typing && take(Modifiers::SHIFT, Key::ArrowUp),
                extend_down: !typing && take(Modifiers::SHIFT, Key::ArrowDown),
                escape: !typing && take(Modifiers::NONE, Key::Escape),
            }
        });

        if keys.find {
            self.focus_search = true;
        }
        if keys.settings {
            self.settings_open = true;
        }
        if keys.all_filter {
            self.set_filter(Filter::All);
        }
        if keys.presets {
            self.set_filter(Filter::Presets);
        }
        if keys.profiles {
            self.set_filter(Filter::Profiles);
        }
        if keys.zip {
            self.pick_zips(now);
        }
        if keys.select_all {
            self.select_all_listed();
        }
        if keys.new_group {
            self.group_from_selection(now);
        }
        if keys.toggle_active {
            self.toggle_selection_active(now);
        }
        if keys.export {
            self.export_selection_or_group(now);
        }
        if keys.delete {
            self.request_delete_selection();
        }
        if keys.rename {
            if let Some(preset) = self.single_selected().cloned() {
                self.start_rename_preset(&preset);
            } else if let (true, Some(Place::Group { section, name })) =
                (self.selection.is_empty(), self.place.clone())
            {
                self.start_rename_group(section, &name);
            }
        }
        if keys.nudge_up || keys.nudge_down {
            self.nudge(keys.nudge_up, now);
        }
        if keys.up || keys.down {
            self.step_selection(keys.down, false);
        }
        if keys.extend_up || keys.extend_down {
            self.step_selection(keys.extend_down, true);
        }
        if keys.escape {
            self.selection.clear();
        }
    }
}

struct Keys {
    find: bool,
    settings: bool,
    all_filter: bool,
    presets: bool,
    profiles: bool,
    zip: bool,
    select_all: bool,
    new_group: bool,
    toggle_active: bool,
    export: bool,
    delete: bool,
    rename: bool,
    nudge_up: bool,
    nudge_down: bool,
    up: bool,
    down: bool,
    extend_up: bool,
    extend_down: bool,
    escape: bool,
}

/// Vorschlag für den Dateinamen: die gemeinsame Gruppe, sonst „Presets“.
pub fn suggested_name(presets: &[Preset]) -> String {
    match presets.first() {
        Some(first)
            if !first.group.is_empty() && presets.iter().all(|p| p.group == first.group) =>
        {
            first.group.clone()
        }
        _ => "Presets".to_owned(),
    }
}

/// Der gemeinsame Bereich aller Einträge – `None` bei gemischter Auswahl.
pub fn common_section(presets: &[Preset]) -> Option<Section> {
    let first = presets.first()?.section();
    presets
        .iter()
        .all(|preset| preset.section() == first)
        .then_some(first)
}
