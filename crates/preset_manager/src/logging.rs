//! Logdatei für die Fehlersuche (`Planung/04-Fehlerbehandlung.md`).
//!
//! - Ort: `%LOCALAPPDATA%\PresetManager\logs\preset-manager.JJJJ-MM-TT.log`
//! - täglich eine neue Datei, die letzten [`LOG_FILES_TO_KEEP`] bleiben
//! - protokolliert wird jeder Auftrag an den Hintergrund-Thread mit Quelle und
//!   Ziel, jedes Ergebnis und jeder Fehler mit der technischen Ursache
//!
//! Die Oberfläche zeigt verständliche Meldungen (`texts::describe`); in die
//! Logdatei kommt zusätzlich der genaue Fehlertext. Kann die Logdatei nicht
//! angelegt werden, läuft die App ohne sie weiter.

use std::path::PathBuf;

use preset_core::report::{ItemOutcome, OperationReport};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

use crate::worker::{Command, Event, Manage};

/// So viele Tagesdateien bleiben liegen.
pub const LOG_FILES_TO_KEEP: usize = 7;

/// Der Ordner für Logdateien; `None`, wenn `%LOCALAPPDATA%` fehlt.
pub fn log_dir() -> Option<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").filter(|value| !value.is_empty())?;
    Some(
        PathBuf::from(local_app_data)
            .join("PresetManager")
            .join("logs"),
    )
}

/// Richtet die Logdatei ein. Der zurückgegebene Wächter muss bis zum
/// Programmende leben – erst beim Verwerfen wird der Rest auf die Platte
/// geschrieben.
pub fn init() -> Option<WorkerGuard> {
    let dir = log_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("preset-manager")
        .filename_suffix("log")
        .max_log_files(LOG_FILES_TO_KEEP)
        .build(&dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok()?;
    log_panics();
    tracing::info!(
        "{} {} gestartet",
        crate::texts::APP_TITLE,
        env!("CARGO_PKG_VERSION")
    );
    Some(guard)
}

/// Schreibt auch Abstürze ins Log, bevor die übliche Meldung erscheint.
fn log_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("Absturz: {info}");
        previous(info);
    }));
}

/// Protokolliert einen Auftrag an den Hintergrund-Thread.
pub fn command(command: &Command) {
    match command {
        // Läuft alle paar Sekunden und nach jeder Aktion – kein Eintrag wert.
        Command::LoadLibrary | Command::CheckAdobe => {}
        Command::Scan { paths, target } => {
            for path in paths {
                tracing::info!("Quelle durchsuchen: {}", path.display());
            }
            if let Some((section, group)) = target {
                tracing::info!("Zielgruppe: {group} ({section:?})");
            }
        }
        Command::Execute { plan, policy } => {
            let selected = plan.items().iter().filter(|item| item.selected).count();
            tracing::info!("Import ausführen: {selected} Einträge, bei Konflikt {policy:?}");
        }
        Command::Manage(task) => manage(task),
    }
}

fn manage(task: &Manage) {
    match task {
        Manage::Delete(presets) => {
            for preset in presets {
                tracing::info!("Löschen (Papierkorb): {}", preset.path.display());
            }
        }
        Manage::RenamePreset { preset, new_name } => {
            tracing::info!("Umbenennen: {} → {new_name}", preset.path.display());
        }
        Manage::RenameGroup { section, old, new } => {
            tracing::info!("Gruppe umbenennen: {old} → {new} ({section:?})");
        }
        Manage::Move { presets, group } => {
            for preset in presets {
                tracing::info!("Verschieben nach „{group}“: {}", preset.path.display());
            }
        }
        Manage::Reorder(presets) => {
            tracing::info!("Reihenfolge festlegen: {} Einträge", presets.len());
        }
        Manage::SetActive {
            presets,
            active,
            group,
        } => {
            let what = if *active {
                "Aktivieren"
            } else {
                "Deaktivieren"
            };
            if let Some(group) = group {
                tracing::info!("{what} mit Verschieben nach „{group}“");
            }
            for preset in presets {
                tracing::info!("{what}: {}", preset.path.display());
            }
        }
    }
}

/// Protokolliert eine Rückmeldung des Hintergrund-Threads.
pub fn event(event: &Event) {
    match event {
        Event::Library { library, report } => {
            let count: usize = library
                .groups()
                .iter()
                .map(|group| group.presets.len())
                .sum();
            tracing::info!("Eingelesen: {count} Einträge");
            report_failures(report);
        }
        Event::PlanReady { plan, rejected } => {
            tracing::info!("Importplan: {} Einträge", plan.items().len());
            for (path, reason) in rejected {
                tracing::warn!("Quelle abgelehnt: {} – {reason}", path.display());
            }
        }
        Event::ScanFailed { rejected } => {
            for (path, reason) in rejected {
                tracing::warn!("Quelle abgelehnt: {} – {reason}", path.display());
            }
        }
        Event::ImportDone {
            report,
            backup_folder,
        } => {
            for (path, outcome) in report.items() {
                match outcome {
                    ItemOutcome::Done => tracing::info!("Installiert: {}", path.display()),
                    ItemOutcome::Skipped { reason } => {
                        tracing::info!("Übersprungen: {} – {reason}", path.display());
                    }
                    ItemOutcome::Failed(error) => {
                        tracing::error!("Fehlgeschlagen: {} – {error}", path.display());
                    }
                }
            }
            if let Some(folder) = backup_folder {
                tracing::info!("Sicherung: {}", folder.display());
            }
        }
        Event::Managed {
            message, failures, ..
        } => {
            if let Some(message) = message {
                tracing::info!("{message}");
            }
            for failure in failures {
                tracing::error!("Fehlgeschlagen: {failure}");
            }
        }
        Event::Error(message) => tracing::error!("{message}"),
        Event::AdobeApps(_) => {}
    }
}

/// Dateien, die beim Einlesen nicht gelesen werden konnten.
fn report_failures(report: &OperationReport) {
    for (path, outcome) in report.items() {
        if let ItemOutcome::Failed(error) = outcome {
            tracing::warn!("Nicht lesbar: {} – {error}", path.display());
        }
    }
}
