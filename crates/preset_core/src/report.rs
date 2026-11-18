//! Ergebnis einer Aktion.
//!
//! Eine Aktion betrifft oft viele Dateien. Scheitert eine davon, laufen die
//! übrigen weiter. Der [`OperationReport`] hält für jede Datei fest, was
//! passiert ist, damit die Oberfläche es dem Nutzer zeigen kann.

use std::path::PathBuf;

use crate::error::CoreError;

/// Was mit einer einzelnen Datei passiert ist.
#[derive(Debug)]
pub enum ItemOutcome {
    /// Die Aktion wurde ausgeführt.
    Done,
    /// Die Datei wurde absichtlich ausgelassen, z. B. wegen eines Konflikts.
    Skipped {
        /// Begründung für den Nutzer.
        reason: String,
    },
    /// Die Aktion ist für diese Datei gescheitert.
    Failed(CoreError),
}

/// Sammelt die Ergebnisse aller Dateien einer Aktion.
#[derive(Debug, Default)]
pub struct OperationReport {
    items: Vec<(PathBuf, ItemOutcome)>,
}

impl OperationReport {
    /// Erstellt einen leeren Bericht.
    pub fn new() -> Self {
        Self::default()
    }

    /// Trägt das Ergebnis für eine Datei ein.
    pub fn record(&mut self, path: PathBuf, outcome: ItemOutcome) {
        self.items.push((path, outcome));
    }

    /// Alle Einträge in der Reihenfolge, in der sie verarbeitet wurden.
    pub fn items(&self) -> &[(PathBuf, ItemOutcome)] {
        &self.items
    }

    /// Gibt alle Einträge heraus, z. B. um einen Fehler weiterzureichen.
    pub fn into_items(self) -> Vec<(PathBuf, ItemOutcome)> {
        self.items
    }

    /// Anzahl der erfolgreich verarbeiteten Dateien.
    pub fn done_count(&self) -> usize {
        self.count(|outcome| matches!(outcome, ItemOutcome::Done))
    }

    /// Anzahl der ausgelassenen Dateien.
    pub fn skipped_count(&self) -> usize {
        self.count(|outcome| matches!(outcome, ItemOutcome::Skipped { .. }))
    }

    /// Anzahl der gescheiterten Dateien.
    pub fn failed_count(&self) -> usize {
        self.count(|outcome| matches!(outcome, ItemOutcome::Failed(_)))
    }

    /// `true`, wenn keine Datei gescheitert ist.
    pub fn is_clean(&self) -> bool {
        self.failed_count() == 0
    }

    fn count(&self, matches_outcome: impl Fn(&ItemOutcome) -> bool) -> usize {
        self.items
            .iter()
            .filter(|(_, outcome)| matches_outcome(outcome))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zaehlt_ergebnisse_nach_art() {
        let mut report = OperationReport::new();
        report.record(PathBuf::from("a.xmp"), ItemOutcome::Done);
        report.record(PathBuf::from("b.xmp"), ItemOutcome::Done);
        report.record(
            PathBuf::from("c.xmp"),
            ItemOutcome::Skipped {
                reason: "existiert bereits".to_owned(),
            },
        );
        report.record(
            PathBuf::from("d.xmp"),
            ItemOutcome::Failed(CoreError::NotAPreset {
                path: PathBuf::from("d.xmp"),
            }),
        );

        assert_eq!(report.done_count(), 2);
        assert_eq!(report.skipped_count(), 1);
        assert_eq!(report.failed_count(), 1);
        assert!(!report.is_clean());
    }

    #[test]
    fn leerer_bericht_ist_fehlerfrei() {
        assert!(OperationReport::new().is_clean());
    }
}
