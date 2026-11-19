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
//!
//! Er führt auch die [`History`]: Jede Aktion, die Dateien ändert, wird mit
//! ihrem Protokoll gemerkt und lässt sich zurücknehmen (F-29). Nach jeder
//! Aktion meldet [`Event::History`], was „Rückgängig“ und „Wiederholen“
//! gerade tun würden.
//!
//! | Datei | Inhalt |
//! |---|---|
//! | `manage` | Verwalten-Aufträge, Beschriftung für Rückgängig |

mod manage;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui;
use preset_core::adobe::{running_adobe_apps, AdobeApp, AdobeLocations};
use preset_core::history::History;
use preset_core::import::{scan, ConflictPolicy, ImportPlan};
use preset_core::library::Library;
use preset_core::preset::{Preset, Section};
use preset_core::report::{ItemOutcome, OperationReport};
use preset_core::safe_fs::{
    default_backup_root, prune_backups, BackupSession, RecycleBin, BACKUPS_TO_KEEP,
};

use crate::logging;
use crate::texts::{self, tr_format};
use manage::no_backup_root;
use manage::summarize;

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
    /// Die letzte Aktion zurücknehmen.
    Undo,
    /// Die zuletzt zurückgenommene Aktion wiederholen.
    Redo,
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
        /// Der Import lässt sich rückgängig machen.
        undoable: bool,
    },
    /// Ergebnis eines Verwalten-Auftrags.
    Managed {
        /// Zusammenfassung für die Einblendung; `None`, wenn nichts geschah.
        message: Option<String>,
        /// Begründungen für gescheiterte Einträge.
        failures: Vec<String>,
        /// Neue Pfade, die danach ausgewählt sein sollen (nach Umbenennen).
        reselect: Vec<PathBuf>,
        /// Die Aktion lässt sich rückgängig machen.
        undoable: bool,
    },
    /// Was „Rückgängig“ bzw. „Wiederholen“ jetzt tun würden – `None`, wenn
    /// es nichts gibt.
    History {
        undo: Option<String>,
        redo: Option<String>,
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
                    history: History::new(),
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
    /// Was sich zurücknehmen und wiederholen lässt.
    history: History,
}

impl State {
    fn handle(&mut self, command: Command) -> Vec<Event> {
        match command {
            Command::LoadLibrary => vec![self.load()],
            Command::Scan { paths, target } => vec![self.scan_sources(paths, target)],
            Command::Execute { plan, policy } => {
                let done = self.execute(&plan, policy);
                // Entpackte Dateien erst jetzt freigeben, danach neu einlesen.
                drop(plan);
                vec![done, self.load(), self.history_event()]
            }
            Command::Manage(task) => vec![self.manage(task), self.load(), self.history_event()],
            Command::Undo => vec![self.step(true), self.load(), self.history_event()],
            Command::Redo => vec![self.step(false), self.load(), self.history_event()],
            Command::CheckAdobe => vec![Event::AdobeApps(running_adobe_apps())],
        }
    }

    fn history_event(&self) -> Event {
        Event::History {
            undo: self.history.undo_label().map(str::to_owned),
            redo: self.history.redo_label().map(str::to_owned),
        }
    }

    /// Rückgängig (`undo`) oder Wiederholen.
    fn step(&mut self, undo: bool) -> Event {
        let Some(root) = default_backup_root() else {
            return no_backup_root();
        };
        let mut session = BackupSession::new(&root);
        let result = if undo {
            self.history
                .undo(&self.locations, &RecycleBin, &mut session)
        } else {
            self.history
                .redo(&self.locations, &RecycleBin, &mut session)
        };
        let _ = prune_backups(&root, BACKUPS_TO_KEEP);
        let Some((label, report)) = result else {
            return Event::Managed {
                message: None,
                failures: Vec::new(),
                reselect: Vec::new(),
                undoable: false,
            };
        };
        // Was zurückkam, wird ausgewählt – so sieht man, was sich änderte.
        let restored: Vec<PathBuf> = report
            .items()
            .iter()
            .filter(|(path, outcome)| matches!(outcome, ItemOutcome::Done) && path.is_file())
            .map(|(path, _)| path.clone())
            .collect();
        let mut event = summarize(report, |_| {
            if undo {
                tr_format!("Rückgängig: {label}", "Undone: {label}")
            } else {
                tr_format!("Wiederholt: {label}", "Redone: {label}")
            }
        });
        if let Event::Managed { reselect, .. } = &mut event {
            *reselect = restored;
        }
        event
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

    fn execute(&mut self, plan: &ImportPlan, policy: ConflictPolicy) -> Event {
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
        let journal = backups.take_journal();
        let undoable = !journal.is_empty();
        let installed = journal.changes().len();
        self.history.push(
            tr_format!("{} installieren", "Install {}", texts::entries(installed)),
            journal,
        );
        Event::ImportDone {
            report,
            backup_folder: backups.folder().map(PathBuf::from),
            undoable,
        }
    }
}
