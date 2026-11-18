//! Hintergrundarbeit (N-04: die Oberfläche friert nie ein).
//!
//! Alles, was Dateien liest oder schreibt, läuft in einem eigenen Thread. Die
//! Oberfläche schickt ihm [`Command`]s und bekommt [`Event`]s zurück. Nach
//! jedem Ereignis weckt der Thread die Oberfläche auf, damit sie es sofort
//! anzeigt.
//!
//! Der Thread hält seinen eigenen Stand der Bibliothek. Verwalten-Aufträge
//! ([`Manage`]) prüfen gegen diesen Stand (z. B. auf doppelte Namen) und laden
//! danach neu.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui;
use preset_core::adobe::{running_adobe_apps, AdobeApp, AdobeLocations};
use preset_core::import::{scan, ConflictPolicy, ImportPlan};
use preset_core::library::{ops, Context, Library};
use preset_core::preset::{Preset, Section};
use preset_core::report::{ItemOutcome, OperationReport};
use preset_core::safe_fs::{
    default_backup_root, prune_backups, BackupSession, RecycleBin, BACKUPS_TO_KEEP,
};
use preset_core::CoreError;

use crate::logging;
use crate::texts::{self, tr_format};

/// Aufträge an den Hintergrund-Thread.
pub enum Command {
    /// Installierte Presets neu einlesen.
    LoadLibrary,
    /// Quellen durchsuchen und einen Importplan erstellen.
    Scan {
        paths: Vec<PathBuf>,
        /// Gruppe, auf die die Quellen gezogen wurden (F-12).
        target: Option<(Section, String)>,
    },
    /// Einen Importplan ausführen.
    Execute {
        plan: Box<ImportPlan>,
        policy: ConflictPolicy,
    },
    /// Installierte Einträge verwalten.
    Manage(Manage),
    /// Nachsehen, ob Photoshop oder Lightroom läuft.
    CheckAdobe,
}

/// Verwalten-Aufträge (Meilenstein 5).
pub enum Manage {
    /// In den Papierkorb legen.
    Delete(Vec<Preset>),
    RenamePreset {
        preset: Preset,
        new_name: String,
    },
    RenameGroup {
        section: Section,
        old: String,
        new: String,
    },
    /// In eine (auch neue) Gruppe verschieben.
    Move {
        presets: Vec<Preset>,
        group: String,
    },
    /// Reihenfolge einer Gruppe festlegen – vollständige Liste in neuer Folge.
    Reorder(Vec<Preset>),
    /// Aktivieren oder deaktivieren (F-48), auf Wunsch vorher in eine andere
    /// Gruppe verschieben (Ziehen auf eine Gruppe des anderen Zustands).
    SetActive {
        presets: Vec<Preset>,
        active: bool,
        group: Option<String>,
    },
}

/// Rückmeldungen des Hintergrund-Threads.
pub enum Event {
    Library {
        library: Library,
        report: OperationReport,
    },
    PlanReady {
        plan: Box<ImportPlan>,
        /// Quellen, die abgelehnt wurden, mit Begründung.
        rejected: Vec<(PathBuf, String)>,
    },
    /// Keine einzige Quelle war brauchbar.
    ScanFailed {
        rejected: Vec<(PathBuf, String)>,
    },
    ImportDone {
        report: OperationReport,
        backup_folder: Option<PathBuf>,
    },
    /// Ergebnis eines Verwalten-Auftrags.
    Managed {
        /// Zusammenfassung für die Einblendung; `None`, wenn nichts geschah.
        message: Option<String>,
        /// Begründungen für gescheiterte Einträge.
        failures: Vec<String>,
        /// Neue Pfade, die danach ausgewählt sein sollen (nach Umbenennen).
        reselect: Vec<PathBuf>,
    },
    /// Ein Fehler, der keinem Eintrag zugeordnet ist.
    Error(String),
    AdobeApps(Vec<AdobeApp>),
}

/// Verbindung zum Hintergrund-Thread.
pub struct Worker {
    commands: Sender<Command>,
    events: Receiver<Event>,
}

impl Worker {
    /// Startet den Thread. `ctx` dient nur zum Aufwecken der Oberfläche.
    pub fn start(ctx: egui::Context, locations: AdobeLocations) -> Self {
        let (command_sender, command_receiver) = mpsc::channel::<Command>();
        let (event_sender, event_receiver) = mpsc::channel::<Event>();
        thread::Builder::new()
            .name("preset-worker".to_owned())
            .spawn(move || {
                let mut state = State {
                    locations,
                    library: Library::default(),
                };
                for command in command_receiver {
                    logging::command(&command);
                    for event in state.handle(command) {
                        logging::event(&event);
                        if event_sender.send(event).is_err() {
                            return; // Oberfläche ist geschlossen.
                        }
                        ctx.request_repaint();
                    }
                }
            })
            .ok();
        Self {
            commands: command_sender,
            events: event_receiver,
        }
    }

    /// Schickt einen Auftrag. Ist der Thread beendet, passiert nichts.
    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    /// Alle bisher eingetroffenen Rückmeldungen.
    pub fn poll(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }
}

struct State {
    locations: AdobeLocations,
    /// Zuletzt eingelesener Stand – Grundlage für Namensprüfungen.
    library: Library,
}

impl State {
    fn handle(&mut self, command: Command) -> Vec<Event> {
        match command {
            Command::LoadLibrary => vec![self.load()],
            Command::Scan { paths, target } => vec![self.scan_sources(paths, target)],
            Command::Execute { plan, policy } => {
                let done = execute(&plan, policy);
                // Entpackte Dateien erst jetzt freigeben, danach neu einlesen.
                drop(plan);
                vec![done, self.load()]
            }
            Command::Manage(task) => vec![self.manage(task), self.load()],
            Command::CheckAdobe => vec![Event::AdobeApps(running_adobe_apps())],
        }
    }

    fn load(&mut self) -> Event {
        let (library, report) = Library::load(&self.locations);
        self.library = library.clone();
        Event::Library { library, report }
    }

    fn scan_sources(&mut self, paths: Vec<PathBuf>, target: Option<(Section, String)>) -> Event {
        let mut scanned = Vec::new();
        let mut rejected = Vec::new();
        for path in paths {
            match scan(&path) {
                Ok(source) => scanned.push(source),
                Err(error) => rejected.push((path, texts::describe(&error))),
            }
        }
        if scanned.is_empty() {
            return Event::ScanFailed { rejected };
        }

        let (library, _) = Library::load(&self.locations);
        let mut plan = ImportPlan::new(scanned, &self.locations, &library);
        if let Some((section, group)) = target {
            if let Err(error) = plan.set_target_group(section, &group) {
                return Event::Error(texts::describe(&error));
            }
        }
        Event::PlanReady {
            plan: Box::new(plan),
            rejected,
        }
    }

    fn manage(&mut self, task: Manage) -> Event {
        let Some(root) = default_backup_root() else {
            return Event::Error(
                texts::tr(
                    "%LOCALAPPDATA% ist nicht gesetzt – ohne Sicherungsordner wird nichts geändert.",
                    "%LOCALAPPDATA% is not set – without a backup folder nothing is changed.",
                )
                .to_owned(),
            );
        };
        let mut backups = BackupSession::new(&root);
        let mut context = Context {
            locations: &self.locations,
            library: &self.library,
            backups: &mut backups,
        };

        let event = match task {
            Manage::Delete(presets) => {
                let refs: Vec<&Preset> = presets.iter().collect();
                let report = ops::delete(&refs, &self.locations, &RecycleBin);
                summarize(report, |done| {
                    tr_format!(
                        "{} in den Papierkorb gelegt",
                        "{} moved to the Recycle Bin",
                        texts::entries(done)
                    )
                })
            }
            Manage::RenamePreset { preset, new_name } => {
                match ops::rename_preset(&preset, &new_name, &mut context) {
                    Ok(path) => Event::Managed {
                        message: Some(tr_format!(
                            "Umbenannt in „{}“",
                            "Renamed to “{}”",
                            new_name.trim()
                        )),
                        failures: Vec::new(),
                        reselect: vec![path],
                    },
                    Err(error) => failed(&error),
                }
            }
            Manage::RenameGroup { section, old, new } => {
                match ops::rename_group(section, &old, &new, &mut context) {
                    Ok(report) => summarize(report, |done| {
                        tr_format!(
                            "Gruppe umbenannt in „{}“ – {} angepasst",
                            "Group renamed to “{}” – {} updated",
                            new.trim(),
                            texts::entries(done)
                        )
                    }),
                    Err(error) => failed(&error),
                }
            }
            Manage::Move { presets, group } => {
                let refs: Vec<&Preset> = presets.iter().collect();
                match ops::move_to_group(&refs, &group, &mut context) {
                    Ok(report) => summarize(report, |done| {
                        let target = if group.trim().is_empty() {
                            texts::no_group().to_owned()
                        } else {
                            tr_format!("„{}“", "“{}”", group.trim())
                        };
                        tr_format!(
                            "{} nach {target} verschoben",
                            "{} moved to {target}",
                            texts::entries(done)
                        )
                    }),
                    Err(error) => failed(&error),
                }
            }
            Manage::Reorder(presets) => {
                let refs: Vec<&Preset> = presets.iter().collect();
                let report = ops::reorder(&refs, &mut context);
                summarize(report, |_| {
                    texts::tr("Reihenfolge gespeichert", "Order saved").to_owned()
                })
            }
            Manage::SetActive {
                presets,
                active,
                group,
            } => {
                let refs: Vec<&Preset> = presets.iter().collect();
                let mut move_failures = Vec::new();
                if let Some(group) = &group {
                    // Verschieben ändert nur den Inhalt, nicht den Pfad.
                    match ops::move_to_group(&refs, group, &mut context) {
                        Ok(report) => move_failures.extend(failure_texts(report)),
                        Err(error) => return failed(&error),
                    }
                }
                let (report, _) = ops::set_active(&refs, active, &self.locations);
                let mut event = summarize(report, |done| {
                    if active {
                        tr_format!(
                            "{} aktiviert – nach einem Neustart wieder in Photoshop und Lightroom",
                            "{} activated – back in Photoshop and Lightroom after a restart",
                            texts::entries(done)
                        )
                    } else {
                        tr_format!(
                            "{} deaktiviert – nach einem Neustart nicht mehr in Photoshop und Lightroom",
                            "{} deactivated – gone from Photoshop and Lightroom after a restart",
                            texts::entries(done)
                        )
                    }
                });
                if let Event::Managed { failures, .. } = &mut event {
                    failures.splice(0..0, move_failures);
                }
                event
            }
        };
        let _ = prune_backups(&root, BACKUPS_TO_KEEP);
        event
    }
}

/// Fasst einen Bericht für die Einblendung zusammen.
fn summarize(report: OperationReport, message: impl FnOnce(usize) -> String) -> Event {
    let done = report.done_count();
    let failures = failure_texts(report);
    Event::Managed {
        message: (done > 0).then(|| message(done)),
        failures,
        reselect: Vec::new(),
    }
}

/// Die Begründungen der gescheiterten Einträge eines Berichts.
fn failure_texts(report: OperationReport) -> Vec<String> {
    report
        .into_items()
        .into_iter()
        .filter_map(|(_, outcome)| match outcome {
            ItemOutcome::Failed(error) => Some(texts::describe(&error)),
            _ => None,
        })
        .collect()
}

fn failed(error: &CoreError) -> Event {
    Event::Managed {
        message: None,
        failures: vec![texts::describe(error)],
        reselect: Vec::new(),
    }
}

fn execute(plan: &ImportPlan, policy: ConflictPolicy) -> Event {
    let Some(root) = default_backup_root() else {
        return Event::Error(
            texts::tr(
                "%LOCALAPPDATA% ist nicht gesetzt – ohne Sicherungsordner wird nichts installiert.",
                "%LOCALAPPDATA% is not set – without a backup folder nothing is installed.",
            )
            .to_owned(),
        );
    };
    let mut backups = BackupSession::new(&root);
    let report = plan.execute(policy, &mut backups);
    // Alte Sicherungen aufräumen; scheitert das, bleibt eben eine mehr liegen.
    let _ = prune_backups(&root, BACKUPS_TO_KEEP);
    Event::ImportDone {
        report,
        backup_folder: backups.folder().map(PathBuf::from),
    }
}
