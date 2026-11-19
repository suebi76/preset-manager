//! Rückgängig und Wiederholen (F-29).
//!
//! Jede verändernde Aktion hinterlässt ein Protokoll ([`Journal`]) mit dem
//! Inhalt jeder Datei vorher und nachher. [`revert`] wendet die Umkehrung an:
//! in umgekehrter Reihenfolge, Datei für Datei – und nur dort, wo die Datei
//! noch so ist, wie die Aktion sie hinterlassen hat. Hat Camera Raw oder der
//! Nutzer sie inzwischen geändert, bleibt sie unangetastet und erscheint mit
//! [`CoreError::ChangedSinceAction`] im Bericht.
//!
//! Das Zurücknehmen ist selbst eine Aktion mit eigenem Protokoll. Dieses
//! wieder zurückzunehmen heißt „Wiederholen“ – beides ist derselbe Vorgang.
//!
//! Es gelten dieselben Regeln wie überall (`Planung/04-Fehlerbehandlung.md`):
//! geändert wird nur innerhalb der Adobe-Ordner, überschrieben oder entfernt
//! wird nur nach einer Sicherung, gelöschte Presets gehen in den Papierkorb
//! und kommen von dort zurück.
//!
//! [`History`] hält die Schritte der laufenden Sitzung. Nach einem Neustart
//! der App beginnt sie leer; die Sicherungsordner bleiben davon unberührt.

use std::path::{Path, PathBuf};

use crate::adobe::locations::same_path;
use crate::adobe::AdobeLocations;
use crate::error::CoreError;
use crate::library::ops::{read_file, remove_empty_folders};
use crate::report::{ItemOutcome, OperationReport};
use crate::safe_fs::{
    move_file, write_atomic, BackupSession, Change, Journal, Trash, WriteMode, BACKUPS_TO_KEEP,
};

/// So viele Schritte lassen sich höchstens zurücknehmen – so viele, wie es
/// Sicherungsordner gibt.
pub const MAX_STEPS: usize = BACKUPS_TO_KEEP;

/// Höchstens so viel Speicher belegen die gemerkten Schritte zusammen. Ältere
/// Schritte fallen weg, wenn es mehr wird (der jüngste bleibt immer).
pub const MAX_BYTES: usize = 256 * 1024 * 1024;

/// Ein Schritt, der sich zurücknehmen oder wiederholen lässt.
#[derive(Debug, Clone)]
pub struct Step {
    /// Was getan wurde, für die Oberfläche – z. B. „„Warm“ umbenennen“.
    pub label: String,
    /// Die Änderungen dieses Schritts.
    pub journal: Journal,
}

/// Die Schritte der laufenden Sitzung.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
}

impl History {
    /// Eine leere Geschichte.
    pub fn new() -> Self {
        Self::default()
    }

    /// Merkt sich eine neue Aktion. Was zuvor zurückgenommen wurde, lässt sich
    /// danach nicht mehr wiederholen. Eine Aktion ohne Änderungen wird nicht
    /// gemerkt.
    pub fn push(&mut self, label: impl Into<String>, journal: Journal) {
        if journal.is_empty() {
            return;
        }
        self.redo.clear();
        self.undo.push(Step {
            label: label.into(),
            journal,
        });
        self.trim();
    }

    /// Was „Rückgängig“ zurücknehmen würde.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|step| step.label.as_str())
    }

    /// Was „Wiederholen“ wiederholen würde.
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|step| step.label.as_str())
    }

    /// Nimmt den letzten Schritt zurück. Gibt seine Beschreibung und den
    /// Bericht zurück; `None`, wenn es nichts zurückzunehmen gibt.
    ///
    /// Was sich zurücknehmen ließ, kann danach wiederholt werden.
    pub fn undo(
        &mut self,
        locations: &AdobeLocations,
        trash: &dyn Trash,
        session: &mut BackupSession,
    ) -> Option<(String, OperationReport)> {
        let step = self.undo.pop()?;
        let report = revert(&step.journal, locations, trash, session);
        let reverted = session.take_journal();
        if !reverted.is_empty() {
            self.redo.push(Step {
                label: step.label.clone(),
                journal: reverted,
            });
        }
        Some((step.label, report))
    }

    /// Wiederholt den zuletzt zurückgenommenen Schritt. Gegenstück zu
    /// [`History::undo`].
    pub fn redo(
        &mut self,
        locations: &AdobeLocations,
        trash: &dyn Trash,
        session: &mut BackupSession,
    ) -> Option<(String, OperationReport)> {
        let step = self.redo.pop()?;
        let report = revert(&step.journal, locations, trash, session);
        let repeated = session.take_journal();
        if !repeated.is_empty() {
            self.undo.push(Step {
                label: step.label.clone(),
                journal: repeated,
            });
            self.trim();
        }
        Some((step.label, report))
    }

    /// Vergisst alle Schritte – z. B. wenn ein anderer Camera-Raw-Ordner
    /// gewählt wurde.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Entfernt die ältesten Schritte, bis Anzahl und Speicher passen.
    fn trim(&mut self) {
        // Wiederholen entsteht nur aus zurückgenommenen Schritten und ist
        // deshalb nie größer als das, was hier erlaubt ist.
        let total = |steps: &[Step]| steps.iter().map(|step| step.journal.size()).sum::<usize>();
        while self.undo.len() > MAX_STEPS || (self.undo.len() > 1 && total(&self.undo) > MAX_BYTES)
        {
            self.undo.remove(0);
        }
    }
}

/// Nimmt die Änderungen eines Protokolls zurück – die jüngste zuerst.
///
/// Jede Datei wird für sich behandelt: Scheitert eine, laufen die übrigen
/// weiter. Was zurückgenommen wurde, steht danach im Protokoll von
/// `session`; daraus lässt sich die Rücknahme wiederum zurücknehmen.
pub fn revert(
    journal: &Journal,
    locations: &AdobeLocations,
    trash: &dyn Trash,
    session: &mut BackupSession,
) -> OperationReport {
    let mut report = OperationReport::new();
    for change in journal.changes().iter().rev() {
        let inverse = change.inverse();
        let path = inverse.path().to_path_buf();
        let outcome = match apply(inverse, locations, trash, session) {
            Ok(Applied::Done) => ItemOutcome::Done,
            Ok(Applied::AlreadyThere) => ItemOutcome::Skipped {
                reason: "ist schon so".to_owned(),
            },
            Err(error) => ItemOutcome::Failed(error),
        };
        report.record(path, outcome);
    }
    report
}

/// Ergebnis einer angewandten Änderung.
enum Applied {
    Done,
    /// Die Datei war bereits im gewünschten Zustand.
    AlreadyThere,
}

/// Wendet eine Änderung an, wenn die Datei noch im erwarteten Zustand ist,
/// und trägt sie dann in `session` ein.
fn apply(
    change: Change,
    locations: &AdobeLocations,
    trash: &dyn Trash,
    session: &mut BackupSession,
) -> Result<Applied, CoreError> {
    let applied = match &change {
        Change::Rewritten {
            path,
            before,
            after,
        } => rewrite(path, before, after, locations, session)?,
        Change::Created {
            path,
            content,
            from_trash,
        } => create(path, content, *from_trash, locations, trash, session)?,
        Change::Removed {
            path,
            content,
            to_trash,
        } => remove(path, content, *to_trash, locations, trash, session)?,
        Change::Moved { from, to } => relocate(from, to, locations)?,
    };
    if matches!(applied, Applied::Done) {
        session.record(change);
    }
    Ok(applied)
}

/// Ersetzt den Inhalt `before` durch `after`.
fn rewrite(
    path: &Path,
    before: &[u8],
    after: &[u8],
    locations: &AdobeLocations,
    session: &mut BackupSession,
) -> Result<Applied, CoreError> {
    locations.ensure_contains(path)?;
    let current = read_expected(path)?;
    if current == after {
        return Ok(Applied::AlreadyThere);
    }
    if current != before {
        return Err(changed(path));
    }
    back_up(path, locations, session)?;
    write_atomic(path, after, WriteMode::Replace)?;
    Ok(Applied::Done)
}

/// Legt die Datei wieder an – aus dem Papierkorb, wenn sie von dort kommt,
/// sonst mit dem gemerkten Inhalt.
fn create(
    path: &Path,
    content: &[u8],
    from_trash: bool,
    locations: &AdobeLocations,
    trash: &dyn Trash,
    session: &mut BackupSession,
) -> Result<Applied, CoreError> {
    locations.ensure_contains(path)?;
    if path.exists() {
        return if read_file(path)? == content {
            Ok(Applied::AlreadyThere)
        } else {
            Err(CoreError::AlreadyExists {
                path: path.to_path_buf(),
            })
        };
    }
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|source| CoreError::Io {
            path: folder.to_path_buf(),
            source,
        })?;
    }
    // Kann der Papierkorb nicht gelesen werden, wird die Datei eben aus dem
    // gemerkten Inhalt neu geschrieben – das Ergebnis ist dasselbe.
    let restored = from_trash && trash.restore(path).unwrap_or(false);
    if !restored {
        write_atomic(path, content, WriteMode::CreateNew)?;
    } else if read_file(path)? != content {
        // Im Papierkorb lag eine ältere Fassung gleichen Namens. Sie wird
        // gesichert und durch den gemerkten Inhalt ersetzt.
        back_up(path, locations, session)?;
        write_atomic(path, content, WriteMode::Replace)?;
    }
    Ok(Applied::Done)
}

/// Entfernt die Datei, wenn sie noch den gemerkten Inhalt hat.
fn remove(
    path: &Path,
    content: &[u8],
    to_trash: bool,
    locations: &AdobeLocations,
    trash: &dyn Trash,
    session: &mut BackupSession,
) -> Result<Applied, CoreError> {
    locations.ensure_contains(path)?;
    if !path.exists() {
        return Ok(Applied::AlreadyThere);
    }
    if read_file(path)? != content {
        return Err(changed(path));
    }
    if to_trash {
        trash.move_to_trash(path)?;
    } else {
        back_up(path, locations, session)?;
        std::fs::remove_file(path).map_err(|source| CoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    }
    remove_empty_folders(path, locations);
    Ok(Applied::Done)
}

/// Verschiebt die Datei von `from` nach `to`.
fn relocate(from: &Path, to: &Path, locations: &AdobeLocations) -> Result<Applied, CoreError> {
    locations.ensure_contains(from)?;
    locations.ensure_contains(to)?;
    // Nur andere Groß-/Kleinschreibung: Windows sieht beide Namen als
    // dieselbe Datei; umbenannt wird trotzdem.
    if same_path(from, to) {
        if !from.exists() {
            return Err(changed(from));
        }
        std::fs::rename(from, to).map_err(|source| CoreError::Io {
            path: from.to_path_buf(),
            source,
        })?;
        return Ok(Applied::Done);
    }
    if !from.exists() {
        return if to.exists() {
            Ok(Applied::AlreadyThere)
        } else {
            Err(changed(from))
        };
    }
    if to.exists() {
        return Err(CoreError::AlreadyExists {
            path: to.to_path_buf(),
        });
    }
    if let Some(folder) = to.parent() {
        std::fs::create_dir_all(folder).map_err(|source| CoreError::Io {
            path: folder.to_path_buf(),
            source,
        })?;
    }
    move_file(from, to)?;
    remove_empty_folders(from, locations);
    Ok(Applied::Done)
}

/// Liest eine Datei, die da sein müsste. Fehlt sie, wurde sie seit der
/// Aktion entfernt.
fn read_expected(path: &Path) -> Result<Vec<u8>, CoreError> {
    if !path.exists() {
        return Err(changed(path));
    }
    read_file(path)
}

fn back_up(
    path: &Path,
    locations: &AdobeLocations,
    session: &mut BackupSession,
) -> Result<PathBuf, CoreError> {
    let relative = locations.relative_path(path)?;
    session.back_up(path, relative)
}

fn changed(path: &Path) -> CoreError {
    CoreError::ChangedSinceAction {
        path: path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safe_fs::Change;

    fn step(size: usize) -> Journal {
        let mut journal = Journal::new();
        journal.record(Change::Created {
            path: "a.xmp".into(),
            content: vec![0; size],
            from_trash: false,
        });
        journal
    }

    #[test]
    fn leere_aktionen_werden_nicht_gemerkt() {
        let mut history = History::new();
        history.push("nichts", Journal::new());
        assert_eq!(history.undo_label(), None);
    }

    #[test]
    fn behaelt_hoechstens_max_steps() {
        let mut history = History::new();
        for number in 0..MAX_STEPS + 5 {
            history.push(format!("Schritt {number}"), step(1));
        }
        assert_eq!(history.undo.len(), MAX_STEPS);
        assert_eq!(history.undo[0].label, "Schritt 5");
    }

    #[test]
    fn der_juengste_schritt_bleibt_auch_wenn_er_gross_ist() {
        let mut history = History::new();
        history.push("klein", step(10));
        history.push("riesig", step(MAX_BYTES + 1));
        assert_eq!(history.undo_label(), Some("riesig"));
        assert_eq!(history.undo.len(), 1);
    }
}
