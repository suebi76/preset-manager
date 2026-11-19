//! Rückmeldungen des Hintergrund-Threads verarbeiten: neuer Stand der
//! Bibliothek, Importplan, Ergebnis einer Aktion, Fehler.

use preset_core::import::ConflictPolicy;
use preset_core::library::Library;
use preset_core::preset::Section;
use preset_core::report::ItemOutcome;

use crate::settings;
use crate::texts::{self, tr, tr_format};
use crate::worker::{Event, Worker};

use super::{file_name, App, Filter, ImportDialog, Place, Rename, RenameTarget, ToastKind};

impl App {
    pub(super) fn handle_events(&mut self, now: f64) {
        let events = self.worker.as_ref().map(Worker::poll).unwrap_or_default();
        for event in events {
            match event {
                Event::Library { library, report } => self.on_library(library, report),
                Event::PlanReady { plan, rejected } => {
                    self.scanning = false;
                    for (path, reason) in &rejected {
                        self.toast(
                            tr_format!(
                                "{} wurde abgelehnt: {reason}",
                                "{} was rejected: {reason}",
                                file_name(path)
                            ),
                            ToastKind::Error,
                            now,
                        );
                    }
                    self.import = Some(ImportDialog {
                        plan: Some(plan),
                        rejected,
                        policy: ConflictPolicy::Skip,
                    });
                }
                Event::ScanFailed { rejected } => {
                    self.scanning = false;
                    for (_path, reason) in rejected {
                        self.toast(reason, ToastKind::Error, now);
                    }
                }
                Event::ImportDone {
                    report,
                    backup_folder,
                    undoable,
                } => {
                    self.import = None;
                    let (done, skipped, failed) = (
                        report.done_count(),
                        report.skipped_count(),
                        report.failed_count(),
                    );
                    let kind = if failed > 0 {
                        ToastKind::Error
                    } else {
                        ToastKind::Success
                    };
                    let mut text = texts::import_result(done, skipped, failed);
                    if done > 0 {
                        text.push_str(tr(
                            " – Photoshop/Lightroom neu starten, damit sie erscheinen",
                            " – restart Photoshop/Lightroom to see them",
                        ));
                        self.hint_dismissed = false;
                    }
                    if undoable && kind == ToastKind::Success {
                        self.toast_undoable(text, now);
                    } else {
                        self.toast(text, kind, now);
                    }
                    if let Some((_, ItemOutcome::Failed(error))) = report
                        .items()
                        .iter()
                        .find(|(_, outcome)| matches!(outcome, ItemOutcome::Failed(_)))
                    {
                        self.toast(texts::describe(error), ToastKind::Error, now);
                    }
                    if let Some(folder) = backup_folder {
                        self.toast(
                            tr_format!(
                                "Ersetzte Dateien gesichert in {}",
                                "Replaced files backed up in {}",
                                folder.display()
                            ),
                            ToastKind::Info,
                            now,
                        );
                    }
                }
                Event::Managed {
                    message,
                    failures,
                    reselect,
                    undoable,
                } => {
                    if let Some(message) = message {
                        if undoable {
                            self.toast_undoable(message, now);
                        } else {
                            self.toast(message, ToastKind::Success, now);
                        }
                        self.hint_dismissed = false;
                    }
                    let failed = failures.len();
                    for failure in failures.into_iter().take(3) {
                        self.toast(failure, ToastKind::Error, now);
                    }
                    if failed > 3 {
                        self.toast(
                            tr_format!(
                                "… und {} weitere Fehler",
                                "… and {} more errors",
                                failed - 3
                            ),
                            ToastKind::Error,
                            now,
                        );
                    }
                    self.reselect = reselect;
                }
                Event::Error(message) => {
                    self.scanning = false;
                    self.managing = false;
                    if let Some(dialog) = &self.import {
                        if dialog.plan.is_none() {
                            self.import = None;
                        }
                    }
                    self.toast(message, ToastKind::Error, now);
                }
                Event::AdobeApps(apps) => self.adobe_apps = apps,
                Event::History { undo, redo } => {
                    self.undo_label = undo;
                    self.redo_label = redo;
                }
            }
        }
    }

    fn on_library(&mut self, library: Library, report: preset_core::report::OperationReport) {
        self.problems = report
            .into_items()
            .into_iter()
            .filter_map(|(path, outcome)| match outcome {
                ItemOutcome::Failed(error) => Some((path, texts::describe(&error))),
                _ => None,
            })
            .collect();

        // Leere Gruppen, in denen inzwischen etwas liegt, sind keine mehr.
        let before = self.empty_groups.len();
        self.empty_groups
            .retain(|(section, name)| library.group(*section, name).is_none());
        if self.empty_groups.len() != before {
            let _ = settings::save_empty_groups(&self.empty_groups);
        }

        self.library = Some(library);
        self.managing = false;

        if let Some(landing) = self.landing.take() {
            self.place = Some(landing);
            self.search.clear();
            if !self
                .filter
                .shows(self.place_section().unwrap_or(Section::Presets))
            {
                self.filter = Filter::All;
            }
        }
        if !self.reselect.is_empty() {
            self.selection = std::mem::take(&mut self.reselect).into_iter().collect();
            self.anchor = self.selection.iter().next().cloned();
            self.show_selection();
        }
        if let Some((section, name)) = self.rename_after_load.take() {
            if self.group_exists(section, &name) {
                self.place = Some(Place::Group {
                    section,
                    name: name.clone(),
                });
                self.rename = Some(Rename {
                    text: name.clone(),
                    target: RenameTarget::Group { section, name },
                    focus: true,
                });
            }
        }
        self.ensure_valid_place();
    }
}
