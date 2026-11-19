//! Alles, was der Nutzer auslösen kann – Import, Auswahl, Verwalten,
//! Ziehen & Ablegen, Tastenkürzel.
//!
//! Die Funktionen prüfen zuerst, ob die Aktion möglich ist, und geben sonst
//! eine verständliche Begründung als Einblendung aus. Dateien ändern sie nie
//! selbst, sondern schicken einen Auftrag an den Hintergrund-Thread.
//!
//! Jede Datei behandelt ein Thema:
//!
//! | Datei | Thema |
//! |---|---|
//! | `import` | Import starten |
//! | `selection` | Gruppe wählen, Filter, Auswahl |
//! | `manage` | Verwalten, Rückgängig, Löschen, Umbenennen, neue Gruppen |
//! | `export` | Als ZIP exportieren |
//! | `activation` | Aktivieren und deaktivieren |
//! | `drag` | Ziehen & Ablegen |
//! | `shortcuts` | Tastenkürzel |

mod activation;
mod drag;
mod export;
mod import;
mod manage;
mod selection;
mod shortcuts;

// Zwei Namen gelten als gleich, wenn sie sich nur in der Schreibweise
// unterscheiden – dieselbe Regel wie beim Verwalten im Kern.
use preset_core::library::ops::same_name;
use preset_core::preset::{Preset, Section};

/// Vorschlag für den Dateinamen: die gemeinsame Gruppe, sonst „Presets“.
pub fn suggested_name(presets: &[Preset]) -> String {
    match presets.first() {
        Some(first)
            if !first.group.is_empty() && presets.iter().all(|p| p.group == first.group) =>
        {
            first.group.clone()
        }
        _ => "Presets".to_owned(),
    }
}

/// Der gemeinsame Bereich aller Einträge – `None` bei gemischter Auswahl.
pub fn common_section(presets: &[Preset]) -> Option<Section> {
    let first = presets.first()?.section();
    presets
        .iter()
        .all(|preset| preset.section() == first)
        .then_some(first)
}
