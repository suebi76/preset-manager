//! In andere Gruppen verschieben (F-26, F-34) und die Reihenfolge
//! festlegen (F-27).

use crate::error::CoreError;
use crate::preset::model::{Preset, Section};
use crate::preset::xmp_edit::MetadataChanges;
use crate::report::{ItemOutcome, OperationReport};

use super::common::{rewrite_in_place, same_name, valid_name};
use super::Context;

/// Verschiebt Einträge in eine andere Gruppe (F-26, F-34). Eine leere
/// `group` heißt „ohne Gruppe“. Die Gruppe darf neu sein (so entsteht eine
/// neue Gruppe, F-33) oder schon existieren.
///
/// Jeder Eintrag bleibt in seinem Bereich: Ein Profil landet in der
/// Profil-Gruppe dieses Namens, ein Preset in der Preset-Gruppe.
///
/// Gibt es in der Zielgruppe schon einen Eintrag desselben Namens, bleibt
/// dieser Eintrag stehen und erscheint mit [`CoreError::NameConflict`] im
/// Bericht.
///
/// # Fehler
///
/// [`CoreError::InvalidName`] für einen ungültigen Gruppennamen – dann ist
/// nichts verändert.
pub fn move_to_group(
    presets: &[&Preset],
    group: &str,
    context: &mut Context<'_>,
) -> Result<OperationReport, CoreError> {
    let group = valid_name(group, true)?;
    let mut names_in_target: Vec<(Section, String)> = [Section::Presets, Section::Profiles]
        .into_iter()
        .filter_map(|section| context.library.group(section, &group))
        .flat_map(|existing| &existing.presets)
        .filter(|existing| !presets.iter().any(|moved| moved.path == existing.path))
        .map(|existing| (existing.section(), existing.name.clone()))
        .collect();

    let mut report = OperationReport::new();
    for preset in presets {
        // Ein Sortierschlüssel gehört zur Reihenfolge der alten Gruppe. In der
        // neuen Gruppe würde er den Eintrag willkürlich nach vorn stellen –
        // deshalb wird er beim Verschieben geleert.
        let changes = MetadataChanges {
            group: Some(group.clone()),
            sort_name: (!preset.sort_name.is_empty()).then(String::new),
            ..MetadataChanges::default()
        };
        let outcome = if preset.group == group {
            ItemOutcome::Skipped {
                reason: "ist schon in dieser Gruppe".to_owned(),
            }
        } else if names_in_target
            .iter()
            .any(|(section, name)| *section == preset.section() && same_name(name, &preset.name))
        {
            ItemOutcome::Failed(CoreError::NameConflict {
                name: preset.name.clone(),
            })
        } else {
            let outcome = rewrite_in_place(&preset.path, &changes, context);
            if matches!(outcome, ItemOutcome::Done) {
                names_in_target.push((preset.section(), preset.name.clone()));
            }
            outcome
        };
        report.record(preset.path.clone(), outcome);
    }
    Ok(report)
}

// ---------------------------------------------------------------------------
// Sortieren
// ---------------------------------------------------------------------------

/// Legt die Reihenfolge fest (F-27): Das erste Preset bekommt den
/// Sortierschlüssel `0001`, das zweite `0002` usw. Camera Raw und Lightroom
/// sortieren danach (Versuch P-2).
///
/// Übergeben werden sollte die vollständige Gruppe in der gewünschten
/// Reihenfolge. Presets, deren Schlüssel schon stimmt, werden nicht
/// angefasst.
pub fn reorder(ordered: &[&Preset], context: &mut Context<'_>) -> OperationReport {
    let width = ordered.len().to_string().len().max(4);
    let mut report = OperationReport::new();
    for (index, preset) in ordered.iter().enumerate() {
        let sort_name = format!("{:0width$}", index + 1);
        let outcome = if preset.sort_name == sort_name {
            ItemOutcome::Skipped {
                reason: "Reihenfolge stimmt bereits".to_owned(),
            }
        } else {
            let changes = MetadataChanges {
                sort_name: Some(sort_name),
                ..MetadataChanges::default()
            };
            rewrite_in_place(&preset.path, &changes, context)
        };
        report.record(preset.path.clone(), outcome);
    }
    report
}
