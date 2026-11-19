//! Das Protokoll einer Aktion: welche Datei wie geändert wurde.
//!
//! Jede verändernde Aktion trägt ihre Änderungen hier ein, mit dem Inhalt
//! vorher und nachher. Daraus lässt sich die Aktion genau zurücknehmen
//! ([`crate::history`]). Preset-Dateien sind klein (meist unter 50 KB), der
//! Inhalt passt deshalb bequem in den Speicher.
//!
//! Jede Änderung hat eine Umkehrung ([`Change::inverse`]): Was angelegt
//! wurde, wird entfernt; was verschoben wurde, kommt zurück. Rückgängig und
//! Wiederholen sind damit dasselbe – die Umkehrung des Protokolls anwenden.

use std::path::{Path, PathBuf};

/// Eine einzelne Änderung an einer Datei.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Der Inhalt einer Datei wurde ersetzt; der Pfad blieb.
    Rewritten {
        /// Die Datei.
        path: PathBuf,
        /// Inhalt vor der Änderung.
        before: Vec<u8>,
        /// Inhalt nach der Änderung.
        after: Vec<u8>,
    },
    /// Eine Datei wurde angelegt.
    Created {
        /// Die neue Datei.
        path: PathBuf,
        /// Ihr Inhalt.
        content: Vec<u8>,
        /// Sie kam aus dem Papierkorb zurück (Umkehrung von „gelöscht“) –
        /// beim Anwenden wird zuerst dort nachgesehen.
        from_trash: bool,
    },
    /// Eine Datei wurde entfernt.
    Removed {
        /// Die entfernte Datei.
        path: PathBuf,
        /// Ihr Inhalt vor dem Entfernen.
        content: Vec<u8>,
        /// In den Papierkorb gelegt (Löschen durch den Nutzer). Sonst wurde
        /// sie gesichert und dann entfernt (z. B. der alte Name beim
        /// Umbenennen).
        to_trash: bool,
    },
    /// Eine Datei wurde verschoben oder umbenannt; der Inhalt blieb.
    Moved {
        /// Wo sie lag.
        from: PathBuf,
        /// Wo sie jetzt liegt.
        to: PathBuf,
    },
}

impl Change {
    /// Die Änderung, die diese aufhebt.
    pub fn inverse(&self) -> Change {
        match self {
            Change::Rewritten {
                path,
                before,
                after,
            } => Change::Rewritten {
                path: path.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Change::Created {
                path,
                content,
                from_trash,
            } => Change::Removed {
                path: path.clone(),
                content: content.clone(),
                to_trash: *from_trash,
            },
            Change::Removed {
                path,
                content,
                to_trash,
            } => Change::Created {
                path: path.clone(),
                content: content.clone(),
                from_trash: *to_trash,
            },
            Change::Moved { from, to } => Change::Moved {
                from: to.clone(),
                to: from.clone(),
            },
        }
    }

    /// Die Datei, um die es geht – bei einer Verschiebung das Ziel.
    pub fn path(&self) -> &Path {
        match self {
            Change::Rewritten { path, .. }
            | Change::Created { path, .. }
            | Change::Removed { path, .. } => path,
            Change::Moved { to, .. } => to,
        }
    }

    /// Wie viel Speicher die Änderung ungefähr belegt.
    fn size(&self) -> usize {
        match self {
            Change::Rewritten { before, after, .. } => before.len() + after.len(),
            Change::Created { content, .. } | Change::Removed { content, .. } => content.len(),
            Change::Moved { .. } => 0,
        }
    }
}

/// Alle Änderungen einer Aktion, in der Reihenfolge, in der sie geschahen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Journal {
    changes: Vec<Change>,
}

impl Journal {
    /// Ein leeres Protokoll.
    pub fn new() -> Self {
        Self::default()
    }

    /// Trägt eine Änderung ein.
    pub fn record(&mut self, change: Change) {
        self.changes.push(change);
    }

    /// Die Änderungen in der Reihenfolge, in der sie geschahen.
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    /// `true`, wenn nichts geändert wurde.
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Ungefährer Speicherbedarf in Byte.
    pub fn size(&self) -> usize {
        self.changes.iter().map(Change::size).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umkehrung_der_umkehrung_ist_das_original() {
        let changes = [
            Change::Rewritten {
                path: "a.xmp".into(),
                before: b"alt".to_vec(),
                after: b"neu".to_vec(),
            },
            Change::Created {
                path: "b.xmp".into(),
                content: b"b".to_vec(),
                from_trash: false,
            },
            Change::Removed {
                path: "c.xmp".into(),
                content: b"c".to_vec(),
                to_trash: true,
            },
            Change::Moved {
                from: "d.xmp".into(),
                to: "e.xmp".into(),
            },
        ];
        for change in changes {
            assert_ne!(change.inverse(), change);
            assert_eq!(change.inverse().inverse(), change);
        }
    }

    #[test]
    fn geloeschtes_kommt_aus_dem_papierkorb_zurueck() {
        let removed = Change::Removed {
            path: "c.xmp".into(),
            content: b"c".to_vec(),
            to_trash: true,
        };
        assert!(matches!(
            removed.inverse(),
            Change::Created {
                from_trash: true,
                ..
            }
        ));
    }

    #[test]
    fn zaehlt_den_speicherbedarf() {
        let mut journal = Journal::new();
        assert!(journal.is_empty());
        journal.record(Change::Rewritten {
            path: "a.xmp".into(),
            before: vec![0; 10],
            after: vec![0; 20],
        });
        journal.record(Change::Moved {
            from: "a".into(),
            to: "b".into(),
        });
        assert_eq!(journal.size(), 30);
        assert_eq!(journal.changes().len(), 2);
    }
}
