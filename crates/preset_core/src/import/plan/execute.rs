//! Den Plan ausführen: jede Datei unmittelbar vor dem Schreiben erneut
//! prüfen, Konflikte nach der gewählten Regel lösen, alles protokollieren.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::CoreError;
use crate::preset::xmp::{metadata_from_bytes, PresetMetadata};
use crate::preset::xmp_edit::{replace_uuid, rewrite_metadata, MetadataChanges};
use crate::report::{ItemOutcome, OperationReport};
use crate::safe_fs::read::read_limited_as;
use crate::safe_fs::{
    read_preset_file, write_atomic, write_to_free_name, BackupSession, Change, WriteMode,
};

use super::{ConflictPolicy, ImportPlan, PlannedAction, PlannedImport};

impl ImportPlan {
    /// Führt den Plan aus.
    ///
    /// Scheitert ein Preset, laufen die übrigen weiter. Der Bericht enthält
    /// für jeden Eintrag den tatsächlichen Zielpfad und das Ergebnis.
    /// Ersetzte Dateien liegen danach in `backups` (siehe
    /// [`BackupSession::folder`]).
    pub fn execute(&self, policy: ConflictPolicy, backups: &mut BackupSession) -> OperationReport {
        let mut report = OperationReport::new();
        // Kennungen, deren Preset jetzt im Zielordner liegt (neu installiert
        // oder schon vorhanden). Nur dann darf eine Dublette entfallen.
        let mut present_uuids: HashSet<&str> = HashSet::new();

        for item in &self.items {
            let uuid = item.preset.metadata.uuid.as_deref();
            let (path, outcome) = match &item.action {
                _ if !item.selected => (
                    item.preset.shown_as.clone(),
                    ItemOutcome::Skipped {
                        reason: deselected_reason(item),
                    },
                ),
                PlannedAction::DuplicateInImport { first }
                    if uuid.is_some_and(|uuid| present_uuids.contains(uuid)) =>
                {
                    (
                        item.preset.shown_as.clone(),
                        ItemOutcome::Skipped {
                            reason: format!(
                                "gleiches Preset (gleiche UUID) wie {}",
                                first.display()
                            ),
                        },
                    )
                }
                // Ist die erste Fundstelle gescheitert, wird stattdessen
                // diese Kopie installiert – sonst fehlte das Preset ganz.
                _ => match self.install(item, policy, backups) {
                    Ok(result) => result,
                    Err(error) => (item.target.clone(), ItemOutcome::Failed(error)),
                },
            };
            if !matches!(outcome, ItemOutcome::Failed(_)) {
                present_uuids.extend(uuid);
            }
            report.record(path, outcome);
        }
        report
    }

    /// Installiert ein Preset und gibt den tatsächlichen Zielpfad zurück.
    fn install(
        &self,
        item: &PlannedImport,
        policy: ConflictPolicy,
        backups: &mut BackupSession,
    ) -> Result<(PathBuf, ItemOutcome), CoreError> {
        let target = &item.target;
        self.check_target(target)?;

        // Die Datei wird unmittelbar vor dem Schreiben noch einmal geprüft.
        // So kann nie etwas anderes als ein lesbares Preset installiert werden.
        let content = read_limited_as(&item.preset.file, &item.preset.shown_as)?;
        let metadata = metadata_from_bytes(&content, &item.preset.shown_as)?;

        // Zielgruppe gewählt? Dann bekommt die Kopie diese Gruppe. Die
        // Quelldatei selbst bleibt unverändert.
        let group = self.group_after_import(item);
        let content = if group == metadata.group {
            content
        } else {
            let changes = MetadataChanges {
                group: Some(group.to_owned()),
                ..MetadataChanges::default()
            };
            rewrite_metadata(&content, &changes, &item.preset.shown_as)?
        };

        if let Some(folder) = target.parent() {
            std::fs::create_dir_all(folder).map_err(|source| CoreError::Io {
                path: folder.to_path_buf(),
                source,
            })?;
        }

        match write_atomic(target, &content, WriteMode::CreateNew) {
            Ok(()) => {
                backups.record(Change::Created {
                    path: target.clone(),
                    content,
                    from_trash: false,
                });
                return Ok((target.clone(), ItemOutcome::Done));
            }
            Err(CoreError::AlreadyExists { .. }) => {}
            Err(error) => return Err(error),
        }

        match policy {
            ConflictPolicy::Skip => Ok((
                target.clone(),
                ItemOutcome::Skipped {
                    reason: "eine Datei mit diesem Namen ist schon installiert".to_owned(),
                },
            )),
            ConflictPolicy::Replace => {
                let relative = self.locations.relative_path(target)?;
                // Für Rückgängig. Lässt sich die vorhandene Datei nicht lesen
                // (beschädigt, zu groß), wird sie trotzdem gesichert und
                // ersetzt – nur zurücknehmen lässt sich das dann nicht.
                let before = read_preset_file(target).ok();
                backups.back_up(target, relative)?;
                write_atomic(target, &content, WriteMode::Replace)?;
                if let Some(before) = before {
                    backups.record(Change::Rewritten {
                        path: target.clone(),
                        before,
                        after: content,
                    });
                }
                Ok((target.clone(), ItemOutcome::Done))
            }
            ConflictPolicy::KeepBoth => {
                let (written, copy) = self.install_copy(target, &content, &item.preset.shown_as)?;
                backups.record(Change::Created {
                    path: written.clone(),
                    content: copy,
                    from_trash: false,
                });
                Ok((written, ItemOutcome::Done))
            }
        }
    }

    /// „Beide behalten“: schreibt das Preset mit neuer Kennung unter den
    /// ersten freien nummerierten Namen. Gibt Pfad und geschriebenen Inhalt
    /// zurück.
    fn install_copy(
        &self,
        target: &Path,
        content: &[u8],
        shown_as: &Path,
    ) -> Result<(PathBuf, Vec<u8>), CoreError> {
        let new_uuid = uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .to_ascii_uppercase();
        let content = replace_uuid(content, &new_uuid, shown_as)?;
        check_uuid(&content, shown_as, &new_uuid)?;

        // `target` ist belegt; die Varianten liegen im selben, schon
        // geprüften Ordner.
        let written = write_to_free_name(target, &content)?;
        Ok((written, content))
    }

    /// Letzte Sicherung gegen Programmierfehler: Der Import schreibt nur
    /// innerhalb von `Settings` (Entscheidung E-4).
    fn check_target(&self, target: &Path) -> Result<(), CoreError> {
        if !target.starts_with(&self.locations.settings) {
            return Err(CoreError::OutsideAdobeFolder {
                path: target.to_path_buf(),
            });
        }
        self.locations.ensure_contains(target)
    }
}

/// Begründung im Bericht für einen abgewählten Eintrag.
fn deselected_reason(item: &PlannedImport) -> String {
    match item.installed_with_same_uuid.first() {
        Some(other) => format!(
            "abgewählt, gleiches Preset schon installiert: {}",
            other.display()
        ),
        None => "abgewählt".to_owned(),
    }
}

/// Prüft, dass die geänderte Datei lesbar ist und die neue Kennung trägt.
fn check_uuid(content: &[u8], shown_as: &Path, expected: &str) -> Result<(), CoreError> {
    let metadata: PresetMetadata = metadata_from_bytes(content, shown_as)?;
    match metadata.uuid.as_deref() {
        Some(uuid) if uuid == expected => Ok(()),
        // Eine Datei ohne Kennung bleibt ohne Kennung – das ist in Ordnung.
        None => Ok(()),
        Some(_) => Err(CoreError::MalformedXmp {
            path: shown_as.to_path_buf(),
            reason: "neue UUID wurde nicht übernommen".to_owned(),
        }),
    }
}
