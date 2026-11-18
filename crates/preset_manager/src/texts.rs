//! Alle Texte der Oberfläche an einer Stelle (F-44).
//!
//! So lässt sich später eine englische Fassung ergänzen, ohne die
//! Oberflächen-Module anzufassen. Fehlermeldungen übersetzt [`describe`] aus
//! den technischen [`CoreError`]-Werten in Sätze für den Nutzer
//! (`Planung/04-Fehlerbehandlung.md`).

use preset_core::CoreError;

pub const APP_TITLE: &str = "Preset Manager";
pub const SETTINGS_TITLE: &str = "Einstellungen";
pub const SETTINGS_TIP: &str = "Einstellungen und Infos (Strg+,)";
pub const CHOOSE_CAMERA_RAW: &str = "Camera-Raw-Ordner wählen …";
pub const CLOSE: &str = "Schließen";
pub const TRADEMARK_NOTE: &str = "Adobe, Photoshop, Lightroom und Camera Raw sind Marken der Adobe Inc. Diese App steht in keiner Verbindung zu Adobe.";
pub const UPDATE_UNREACHABLE: &str =
    "GitHub war gerade nicht erreichbar. Preset Manager sieht später wieder nach.";
pub const UPDATE_INSTALL: &str = "Jetzt installieren";
pub const UPDATE_RESTART: &str = "Jetzt neu starten";
pub const UPDATE_CHECK: &str = "Nach Updates suchen";
/// Beschriftung und Erklärung der Update-Einstellung.
pub fn update_mode(mode: crate::settings::UpdateMode) -> (&'static str, &'static str) {
    use crate::settings::UpdateMode;
    match mode {
        UpdateMode::Notify => (
            "Hinweis zeigen, Installation mit einem Klick (empfohlen)",
            "Preset Manager sieht nach dem Start nach und zeigt unten einen Hinweis, wenn es eine neue Version gibt.",
        ),
        UpdateMode::Automatic => (
            "Automatisch installieren",
            "Neue Versionen werden im Hintergrund geladen und beim Schließen der App installiert.",
        ),
        UpdateMode::Manual => (
            "Nur wenn ich nachsehe",
            "Preset Manager sucht nicht selbst. Über „Nach Updates suchen“ prüfst du es, wann du willst.",
        ),
    }
}

pub const SEARCH_HINT: &str = "Presets und Profile suchen …";
pub const IMPORT_BUTTON: &str = "Presets importieren";
pub const IMPORT_FOLDER: &str = "Ordner wählen …";
pub const IMPORT_ZIP: &str = "ZIP-Dateien wählen …";
pub const FILTER_ALL: &str = "Alle";
pub const FILTER_PRESETS: &str = "Presets";
pub const FILTER_PROFILES: &str = "Profile";
pub const SECTION_PRESETS: &str = "PRESETS";
pub const SECTION_PROFILES: &str = "PROFILE";
pub const SECTION_PROBLEMS: &str = "NICHT LESBAR";
pub const PROBLEMS_GROUP: &str = "Beschädigte Dateien";
pub const NO_GROUP: &str = "(ohne Gruppe)";
pub const LOADING: &str = "Presets werden gelesen …";
pub const DROP_HERE: &str = "Loslassen zum Importieren";
pub const DETAILS: &str = "DETAILS";
pub const GROUP: &str = "GRUPPE";
pub const SHOW_IN_EXPLORER: &str = "Im Explorer zeigen";
pub const ORIGIN_ADOBE: &str = "Über Adobes Importdialog installiert";
pub const ORIGIN_OWN: &str = "Eigene / mit dieser App installiert";
pub const WHERE_PRESETS: &str = "Camera Raw: Vorgaben";
pub const WHERE_PROFILES: &str = "Camera Raw: Bearbeiten → Profil → Durchsuchen";
pub const NOTHING_FOUND: &str = "Nichts gefunden";
pub const NOTHING_FOUND_HINT: &str =
    "Andere Schreibweise versuchen oder Filter auf „Alle“ stellen.";
pub const EMPTY_LIBRARY: &str = "Noch keine Presets installiert";
pub const EMPTY_LIBRARY_HINT: &str =
    "Ordner oder ZIP-Datei ins Fenster ziehen – oder oben auf „Presets importieren“ klicken.";
pub const GROUP_HINT: &str = "Gruppen stehen in den Dateien selbst (crs:Group). Presets und Profile haben getrennte Gruppen – wie in Camera Raw.";
pub const PROBLEMS_HINT: &str =
    "Diese Dateien konnte die App nicht lesen. Camera Raw übergeht sie ebenfalls.";

pub const IMPORT_TITLE: &str = "Presets importieren";
pub const IMPORT_POLICY: &str = "Wenn eine Datei schon vorhanden ist:";
pub const POLICY_SKIP: &str = "überspringen";
pub const POLICY_REPLACE: &str = "ersetzen (vorher sichern)";
pub const POLICY_KEEP_BOTH: &str = "beide behalten";
pub const STATUS_NEW: &str = "neu";
pub const STATUS_CONFLICT: &str = "Datei vorhanden";
pub const STATUS_DUPLICATE: &str = "doppelt im Paket";
pub const STATUS_INSTALLED: &str = "schon installiert";
pub const STATUS_INSTALLED_TIP: &str = "Dasselbe Preset (gleiche UUID) ist schon installiert. Nochmal installieren würde es in Camera Raw doppelt zeigen.";
pub const STATUS_CONFLICT_TIP: &str =
    "Im Zielordner gibt es schon eine Datei mit diesem Namen – siehe Regel unten.";
pub const STATUS_DUPLICATE_TIP: &str =
    "Das Paket enthält dieses Preset mehrfach (z. B. für Lightroom und Camera Raw). Es wird nur einmal installiert.";
pub const CANCEL: &str = "Abbrechen";
pub const INSTALLING: &str = "Wird installiert …";
pub const SCANNING: &str = "Quelle wird gelesen …";
pub const RESTART_HINT: &str = "Änderungen erscheinen in Camera Raw nach einem Neustart.";

pub const NEW_GROUP: &str = "Neue Gruppe";
pub const NEW_GROUP_FROM_SELECTION: &str = "Neue Gruppe aus Auswahl";
pub const EMPTY_GROUP_TITLE: &str = "Zieh Einträge hierher";
pub const EMPTY_GROUP_HINT: &str =
    "Camera Raw zeigt die Gruppe, sobald etwas darin liegt. Bis dahin merkt sich die App sie.";
pub const MIXED_SECTIONS: &str = "Presets und Profile bitte getrennt verschieben";
pub const PROFILE_INTO_PRESET_GROUP: &str =
    "Profile gehören in eine Profil-Gruppe – Camera Raw zeigt sie im Profilbrowser";
pub const PRESET_INTO_PROFILE_GROUP: &str = "Presets gehören in eine Preset-Gruppe";
pub const RENAME: &str = "Umbenennen";
pub const MOVE_TO: &str = "Verschieben nach";
pub const CHOOSE_GROUP: &str = "Gruppe wählen …";
pub const TO_TRASH: &str = "In den Papierkorb …";
pub const GROUP_TO_TRASH: &str = "Gruppe in den Papierkorb …";
pub const REMOVE_EMPTY_GROUP: &str = "Leere Gruppe entfernen";
pub const RENAME_GROUP: &str = "Gruppe umbenennen";
pub const MERGE_HINT: &str = "Tipp: Eine Gruppe auf eine andere ziehen führt beide zusammen.";
pub const TRASH_NOTE: &str =
    "Die Dateien kommen in den Windows-Papierkorb und lassen sich dort wiederherstellen.";
pub const CONFIRM_TRASH: &str = "In den Papierkorb legen";
pub const CONFIRM_MERGE: &str = "Zusammenführen";
pub const DRAG_HINT_REORDER: &str = "Ziehen zum Verschieben oder Sortieren";
pub const BUSY_MANAGING: &str = "Änderung wird gespeichert …";

/// Hinweis, dass Adobe-Programme laufen, z. B. „Photoshop und Lightroom
/// Classic laufen – …“.
pub fn running_hint(names: &[&str]) -> String {
    let verb = if names.len() == 1 { "läuft" } else { "laufen" };
    format!("{} {verb} – {RESTART_HINT}", join_and(names))
}

/// „A“, „A und B“, „A, B und C“.
pub fn join_and(parts: &[&str]) -> String {
    match parts {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{} und {last}", rest.join(", ")),
    }
}

/// „1 Preset“, „3 Presets“, „2 Presets, 1 Profil“.
pub fn count_label(presets: usize, profiles: usize) -> String {
    let presets_text = plural(presets, "Preset", "Presets");
    let profiles_text = plural(profiles, "Profil", "Profile");
    match (presets, profiles) {
        (_, 0) => presets_text,
        (0, _) => profiles_text,
        _ => format!("{presets_text}, {profiles_text}"),
    }
}

pub fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// „5 von 9 werden installiert“.
pub fn will_install(count: usize, total: usize) -> String {
    format!("{count} von {total} werden installiert")
}

/// Beschriftung der Installieren-Schaltfläche.
pub fn install_button(count: usize) -> String {
    if count == 0 {
        "Nichts zu installieren".to_owned()
    } else {
        format!("{count} installieren")
    }
}

/// Ergebnis eines Imports für die Einblendung.
pub fn import_result(done: usize, skipped: usize, failed: usize) -> String {
    let mut parts = vec![format!(
        "{} installiert",
        plural(done, "Eintrag", "Einträge")
    )];
    if skipped > 0 {
        parts.push(format!("{skipped} übersprungen"));
    }
    if failed > 0 {
        parts.push(format!("{failed} fehlgeschlagen"));
    }
    parts.join(" · ")
}

/// Übersetzt einen Fehler in einen Satz für den Nutzer.
pub fn describe(error: &CoreError) -> String {
    use std::error::Error;
    let cause = error
        .source()
        .map(|source| format!(" ({source})"))
        .unwrap_or_default();
    match error {
        CoreError::AdobeFolderNotFound { searched } => format!(
            "Camera Raw wurde nicht gefunden. Erwartet in: {}",
            searched.display()
        ),
        CoreError::NotACameraRawFolder { path } => format!(
            "{} ist kein Camera-Raw-Ordner (er enthält weder „Settings“ noch „ImportedSettings“).",
            path.display()
        ),
        CoreError::Io { path, .. } => {
            format!("„{}“ konnte nicht gelesen oder geschrieben werden{cause}.", path.display())
        }
        CoreError::InvalidArchive { path, reason } => format!(
            "Das Archiv „{}“ kann nicht gelesen werden ({reason}).",
            file_name(path)
        ),
        CoreError::UnsafeArchiveEntry { entry } => format!(
            "Das Archiv enthält einen unzulässigen Pfad („{entry}“) und wurde nicht entpackt."
        ),
        CoreError::ArchiveTooLarge { path, reason } => format!(
            "Das Archiv „{}“ wurde abgelehnt: {reason}.",
            file_name(path)
        ),
        CoreError::UnsupportedSource { path } => format!(
            "„{}“ ist weder ein Ordner noch eine ZIP-Datei.",
            file_name(path)
        ),
        CoreError::NotAPreset { path } => {
            format!("„{}“ ist kein Preset.", file_name(path))
        }
        CoreError::MalformedXmp { path, .. } => {
            format!("„{}“ ist beschädigt.", file_name(path))
        }
        CoreError::InvalidName { name, reason } => {
            format!("Der Name „{name}“ ist nicht zulässig: {reason}.")
        }
        CoreError::NameConflict { name } => {
            format!("In dieser Gruppe gibt es bereits einen Eintrag namens „{name}“.")
        }
        CoreError::GroupNotFound { name } => format!("Die Gruppe „{name}“ gibt es nicht mehr."),
        CoreError::NoPresetsFound { source_path } => format!(
            "In „{}“ wurden keine Presets gefunden.",
            file_name(source_path)
        ),
        CoreError::Trash { path, reason } => format!(
            "„{}“ konnte nicht in den Papierkorb gelegt werden ({reason}).",
            file_name(path)
        ),
        CoreError::AlreadyExists { path } => {
            format!("„{}“ gibt es schon.", file_name(path))
        }
        CoreError::OutsideAdobeFolder { .. } => {
            "Interner Fehler – das Ziel lag außerhalb der Adobe-Ordner. Es wurde nichts geschrieben."
                .to_owned()
        }
    }
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbindet_aufzaehlungen() {
        assert_eq!(join_and(&["A"]), "A");
        assert_eq!(join_and(&["A", "B"]), "A und B");
        assert_eq!(join_and(&["A", "B", "C"]), "A, B und C");
    }

    #[test]
    fn zaehlt_mit_richtiger_einzahl() {
        assert_eq!(count_label(1, 0), "1 Preset");
        assert_eq!(count_label(0, 3), "3 Profile");
        assert_eq!(count_label(2, 1), "2 Presets, 1 Profil");
        assert_eq!(
            running_hint(&["Photoshop"]),
            format!("Photoshop läuft – {RESTART_HINT}")
        );
    }
}
