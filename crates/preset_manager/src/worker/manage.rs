//! Verwalten-Aufträge ausführen und das Ergebnis für die Oberfläche
//! zusammenfassen; jede Aktion wird in der Geschichte gemerkt (F-29).

use preset_core::library::{ops, Context};
use preset_core::preset::Preset;
use preset_core::report::{ItemOutcome, OperationReport};
use preset_core::safe_fs::{
    default_backup_root, prune_backups, BackupSession, RecycleBin, BACKUPS_TO_KEEP,
};
use preset_core::CoreError;

use crate::texts::{self, tr_format};

use super::{Event, Manage, State};

impl State {
    pub(super) fn manage(&mut self, task: Manage) -> Event {
        let Some(root) = default_backup_root() else {
            return no_backup_root();
        };
        let label = label(&task);
        let mut backups = BackupSession::new(&root);
        let mut context = Context {
            locations: &self.locations,
            library: &self.library,
            backups: &mut backups,
        };

        let event = match task {
            Manage::Delete(presets) => {
                let refs: Vec<&Preset> = presets.iter().collect();
                let report = ops::delete(&refs, &self.locations, &RecycleBin, context.backups);
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
                        undoable: false,
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
                let (report, _) = ops::set_active(&refs, active, &self.locations, context.backups);
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
        let journal = backups.take_journal();
        let undoable = !journal.is_empty();
        self.history.push(label, journal);
        mark_undoable(event, undoable)
    }
}

/// Fehlermeldung, wenn es keinen Sicherungsordner gibt.
pub(super) fn no_backup_root() -> Event {
    Event::Error(
        texts::tr(
            "%LOCALAPPDATA% ist nicht gesetzt – ohne Sicherungsordner wird nichts geändert.",
            "%LOCALAPPDATA% is not set – without a backup folder nothing is changed.",
        )
        .to_owned(),
    )
}

/// Setzt bei einer Verwalten-Rückmeldung, ob sie sich rückgängig machen lässt.
fn mark_undoable(mut event: Event, value: bool) -> Event {
    if let Event::Managed { undoable, .. } = &mut event {
        *undoable = value;
    }
    event
}

/// Wie eine Aktion in „Rückgängig: …“ heißt.
pub(super) fn label(task: &Manage) -> String {
    match task {
        Manage::Delete(presets) => tr_format!("{} löschen", "Delete {}", subject(presets)),
        Manage::RenamePreset { preset, .. } => {
            tr_format!("„{}“ umbenennen", "Rename “{}”", preset.name)
        }
        Manage::RenameGroup { old, .. } => {
            tr_format!("Gruppe „{}“ umbenennen", "Rename group “{}”", old)
        }
        Manage::Move { presets, .. } => tr_format!("{} verschieben", "Move {}", subject(presets)),
        Manage::Reorder(_) => texts::tr("Reihenfolge ändern", "Change order").to_owned(),
        Manage::SetActive {
            presets, active, ..
        } => {
            if *active {
                tr_format!("{} aktivieren", "Activate {}", subject(presets))
            } else {
                tr_format!("{} deaktivieren", "Deactivate {}", subject(presets))
            }
        }
    }
}

/// „„Warm““ für einen Eintrag, „3 Einträge“ für mehrere.
fn subject(presets: &[Preset]) -> String {
    match presets {
        [single] => tr_format!("„{}“", "“{}”", single.name),
        _ => texts::entries(presets.len()),
    }
}

/// Fasst einen Bericht für die Einblendung zusammen.
pub(super) fn summarize(report: OperationReport, message: impl FnOnce(usize) -> String) -> Event {
    let done = report.done_count();
    let failures = failure_texts(report);
    Event::Managed {
        message: (done > 0).then(|| message(done)),
        failures,
        reselect: Vec::new(),
        undoable: false,
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
        undoable: false,
    }
}
