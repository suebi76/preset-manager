//! Alle Texte der Oberfläche – auf Deutsch und Englisch (F-44, F-46).
//!
//! - Texte, die mehrfach vorkommen, und alle Fehlermeldungen stehen hier.
//! - Einmalige Beschriftungen stehen zweisprachig dort, wo sie gebraucht
//!   werden: [`tr`]`("Deutsch", "English")` bzw. [`tr_format!`]. Eine Suche
//!   nach `tr(` und `tr_format!` findet also jeden Text der Oberfläche.
//! - Die Sprache gilt für die ganze App und lässt sich jederzeit wechseln
//!   ([`set_language`]); das nächste Bild erscheint in der neuen Sprache.
//!
//! Logdatei und Konsolenwerkzeug bleiben deutsch.

use std::sync::atomic::{AtomicU8, Ordering};

use preset_core::{ArchiveLimit, CoreError, NameProblem};

use crate::app::file_name;

/// Die Sprachen der Oberfläche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    German,
    English,
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Stellt die Sprache für die ganze App um.
pub fn set_language(language: Language) {
    let value = match language {
        Language::German => 0,
        Language::English => 1,
    };
    CURRENT.store(value, Ordering::Relaxed);
}

pub fn language() -> Language {
    match CURRENT.load(Ordering::Relaxed) {
        0 => Language::German,
        _ => Language::English,
    }
}

/// Die Sprache von Windows: Deutsch, wenn Windows deutsch ist, sonst Englisch.
pub fn system_language() -> Language {
    let german = sys_locale::get_locales()
        .next()
        .is_some_and(|locale| locale.to_ascii_lowercase().starts_with("de"));
    if german {
        Language::German
    } else {
        Language::English
    }
}

/// Der Text in der aktuellen Sprache.
pub fn tr(german: &'static str, english: &'static str) -> &'static str {
    match language() {
        Language::German => german,
        Language::English => english,
    }
}

/// Wie `format!`, aber mit einem deutschen und einem englischen Muster:
/// `tr_format!("{n} Einträge", "{n} entries")`.
macro_rules! tr_format {
    ($german:literal, $english:literal $(, $arg:expr)* $(,)?) => {
        match $crate::texts::language() {
            $crate::texts::Language::German => format!($german $(, $arg)*),
            $crate::texts::Language::English => format!($english $(, $arg)*),
        }
    };
}
pub(crate) use tr_format;

/// Erzeugt für jedes Textpaar eine Funktion, die den Text in der aktuellen
/// Sprache liefert.
macro_rules! texts {
    ($($name:ident: $german:literal, $english:literal;)*) => {
        $(
            pub fn $name() -> &'static str {
                tr($german, $english)
            }
        )*
    };
}

pub const APP_TITLE: &str = "Preset Manager";

texts! {
    settings_title: "Einstellungen", "Settings";
    settings_tip: "Einstellungen und Infos (Strg+,)", "Settings and info (Ctrl+,)";
    choose_camera_raw: "Camera-Raw-Ordner wählen …", "Choose Camera Raw folder …";
    close: "Schließen", "Close";
    trademark_note:
        "Adobe, Photoshop, Lightroom und Camera Raw sind Marken der Adobe Inc. Diese App steht in keiner Verbindung zu Adobe.",
        "Adobe, Photoshop, Lightroom and Camera Raw are trademarks of Adobe Inc. This app is not affiliated with Adobe.";
    update_unreachable:
        "GitHub war gerade nicht erreichbar. Preset Manager sieht später wieder nach.",
        "GitHub could not be reached. Preset Manager will try again later.";
    update_install: "Jetzt installieren", "Install now";
    update_restart: "Jetzt neu starten", "Restart now";
    update_check: "Nach Updates suchen", "Check for updates";
    hide_hint: "Hinweis ausblenden", "Hide this notice";
    wait_for_action: "Bitte warten, bis die laufende Aktion fertig ist.", "Please wait until the current action has finished.";

    search_hint: "Presets und Profile suchen …", "Search presets and profiles …";
    import_button: "Presets importieren", "Import presets";
    import_folder: "Ordner wählen …", "Choose folder …";
    import_zip: "ZIP-Dateien wählen …", "Choose ZIP files …";
    filter_all: "Alle", "All";
    filter_presets: "Presets", "Presets";
    filter_profiles: "Profile", "Profiles";
    section_presets: "PRESETS", "PRESETS";
    section_profiles: "PROFILE", "PROFILES";
    section_problems: "NICHT LESBAR", "UNREADABLE";
    problems_group: "Beschädigte Dateien", "Damaged files";
    no_group: "(ohne Gruppe)", "(no group)";
    loading: "Presets werden gelesen …", "Reading presets …";
    drop_here: "Loslassen zum Importieren", "Drop to import";
    details: "DETAILS", "DETAILS";
    show_in_explorer: "Im Explorer zeigen", "Show in Explorer";
    origin_adobe: "Über Adobes Importdialog installiert", "Installed with Adobe's import dialog";
    origin_own: "Eigene / mit dieser App installiert", "Own / installed with this app";
    where_presets: "Camera Raw: Vorgaben", "Camera Raw: Presets";
    where_profiles: "Camera Raw: Bearbeiten → Profil → Durchsuchen", "Camera Raw: Edit → Profile → Browse";
    nothing_found: "Nichts gefunden", "Nothing found";
    nothing_found_hint:
        "Andere Schreibweise versuchen oder Filter auf „Alle“ stellen.",
        "Try a different spelling or set the filter to “All”.";
    empty_library: "Noch keine Presets installiert", "No presets installed yet";
    empty_library_hint:
        "Ordner oder ZIP-Datei ins Fenster ziehen – oder oben auf „Presets importieren“ klicken.",
        "Drag a folder or ZIP file into the window – or click “Import presets” at the top.";
    section_inactive: "DEAKTIVIERT", "DEACTIVATED";
    inactive_hint:
        "Erscheinen nicht in Photoshop und Lightroom – bleiben hier, bis sie wieder aktiviert werden.",
        "Not shown in Photoshop and Lightroom – they stay here until activated again.";
    deactivate: "Deaktivieren", "Deactivate";
    undo: "Rückgängig", "Undo";
    export_zip: "Als ZIP exportieren …", "Export as ZIP …";
    export_group_zip: "Gruppe als ZIP exportieren …", "Export group as ZIP …";
    activate: "Aktivieren", "Activate";
    deactivate_group: "Gruppe deaktivieren", "Deactivate group";
    activate_group: "Gruppe aktivieren", "Activate group";
    problems_hint:
        "Diese Dateien konnte die App nicht lesen. Camera Raw übergeht sie ebenfalls.",
        "The app could not read these files. Camera Raw skips them as well.";

    import_title: "Presets importieren", "Import presets";
    import_policy: "Wenn eine Datei schon vorhanden ist:", "If a file already exists:";
    policy_skip: "überspringen", "skip";
    policy_replace: "ersetzen (vorher sichern)", "replace (back up first)";
    policy_keep_both: "beide behalten", "keep both";
    status_new: "neu", "new";
    status_conflict: "Datei vorhanden", "file exists";
    status_duplicate: "doppelt im Paket", "duplicate in pack";
    status_installed: "schon installiert", "already installed";
    status_installed_tip:
        "Dasselbe Preset (gleiche UUID) ist schon installiert. Nochmal installieren würde es in Camera Raw doppelt zeigen.",
        "The same preset (same UUID) is already installed. Installing it again would show it twice in Camera Raw.";
    status_conflict_tip:
        "Im Zielordner gibt es schon eine Datei mit diesem Namen – siehe Regel unten.",
        "The target folder already has a file with this name – see the rule below.";
    status_duplicate_tip:
        "Das Paket enthält dieses Preset mehrfach (z. B. für Lightroom und Camera Raw). Es wird nur einmal installiert.",
        "The pack contains this preset more than once (e.g. for Lightroom and Camera Raw). It is installed only once.";
    cancel: "Abbrechen", "Cancel";
    installing: "Wird installiert …", "Installing …";
    scanning: "Quelle wird gelesen …", "Reading source …";
    restart_hint:
        "Änderungen erscheinen in Camera Raw nach einem Neustart.",
        "Changes appear in Camera Raw after a restart.";

    new_group: "Neue Gruppe", "New group";
    new_group_from_selection: "Neue Gruppe aus Auswahl", "New group from selection";
    empty_group_title: "Zieh Einträge hierher", "Drag items here";
    empty_group_hint:
        "Camera Raw zeigt die Gruppe, sobald etwas darin liegt. Bis dahin merkt sich die App sie.",
        "Camera Raw shows the group as soon as it contains something. Until then the app remembers it.";
    mixed_sections:
        "Presets und Profile bitte getrennt verschieben",
        "Please move presets and profiles separately";
    profile_into_preset_group:
        "Profile gehören in eine Profil-Gruppe – Camera Raw zeigt sie im Profilbrowser",
        "Profiles belong in a profile group – Camera Raw shows them in the profile browser";
    preset_into_profile_group: "Presets gehören in eine Preset-Gruppe", "Presets belong in a preset group";
    rename: "Umbenennen", "Rename";
    move_to: "Verschieben nach", "Move to";
    choose_group: "Gruppe wählen …", "Choose group …";
    to_trash: "In den Papierkorb …", "Move to Recycle Bin …";
    group_to_trash: "Gruppe in den Papierkorb …", "Move group to Recycle Bin …";
    remove_empty_group: "Leere Gruppe entfernen", "Remove empty group";
    rename_group: "Gruppe umbenennen", "Rename group";
    merge_hint:
        "Tipp: Eine Gruppe auf eine andere ziehen führt beide zusammen.",
        "Tip: Drag a group onto another one to merge them.";
    trash_note:
        "Die Dateien kommen in den Windows-Papierkorb und lassen sich dort wiederherstellen.",
        "The files go to the Windows Recycle Bin and can be restored from there.";
    confirm_trash: "In den Papierkorb legen", "Move to Recycle Bin";
    confirm_merge: "Zusammenführen", "Merge";
    drag_hint_reorder: "Ziehen zum Verschieben oder Sortieren", "Drag to move or reorder";
    busy_managing: "Änderung wird gespeichert …", "Saving changes …";
    other_folder: "Anderer Ordner", "Other folder";
    preset: "Preset", "Preset";
    profile: "Profil", "Profile";
}

/// Beschriftung und Erklärung der Update-Einstellung.
pub fn update_mode(mode: crate::settings::UpdateMode) -> (&'static str, &'static str) {
    use crate::settings::UpdateMode;
    match mode {
        UpdateMode::Notify => (
            tr(
                "Hinweis zeigen, Installation mit einem Klick (empfohlen)",
                "Show a notice, install with one click (recommended)",
            ),
            tr(
                "Preset Manager sieht nach dem Start nach und zeigt unten einen Hinweis, wenn es eine neue Version gibt.",
                "Preset Manager checks after starting and shows a notice at the bottom when a new version is out.",
            ),
        ),
        UpdateMode::Automatic => (
            tr("Automatisch installieren", "Install automatically"),
            tr(
                "Neue Versionen werden im Hintergrund geladen und beim Schließen der App installiert.",
                "New versions are downloaded in the background and installed when you close the app.",
            ),
        ),
        UpdateMode::Manual => (
            tr("Nur wenn ich nachsehe", "Only when I check"),
            tr(
                "Preset Manager sucht nicht selbst. Über „Nach Updates suchen“ prüfst du es, wann du willst.",
                "Preset Manager does not check by itself. Use “Check for updates” whenever you like.",
            ),
        ),
    }
}

/// Hinweis, dass Adobe-Programme laufen, z. B. „Photoshop und Lightroom
/// Classic laufen – …“.
pub fn running_hint(names: &[&str]) -> String {
    let several = names.len() > 1;
    let names = join_and(names);
    let restart = restart_hint();
    match (language(), several) {
        (Language::German, false) => format!("{names} läuft – {restart}"),
        (Language::German, true) => format!("{names} laufen – {restart}"),
        (Language::English, false) => format!("{names} is running – {restart}"),
        (Language::English, true) => format!("{names} are running – {restart}"),
    }
}

fn and_word() -> &'static str {
    tr(" und ", " and ")
}

/// „A“, „A und B“, „A, B und C“.
pub fn join_and(parts: &[&str]) -> String {
    match parts {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{}{}{last}", rest.join(", "), and_word()),
    }
}

/// Zahl mit Wort in Einzahl oder Mehrzahl.
fn count(count: usize, one: (&str, &str), many: (&str, &str)) -> String {
    let (german, english) = if count == 1 { one } else { many };
    let word = match language() {
        Language::German => german,
        Language::English => english,
    };
    format!("{count} {word}")
}

/// „1 Eintrag“, „3 Einträge“.
pub fn entries(n: usize) -> String {
    count(n, ("Eintrag", "item"), ("Einträge", "items"))
}

pub fn presets(n: usize) -> String {
    count(n, ("Preset", "preset"), ("Presets", "presets"))
}

pub fn profiles(n: usize) -> String {
    count(n, ("Profil", "profile"), ("Profile", "profiles"))
}

pub fn files(n: usize) -> String {
    count(n, ("Datei", "file"), ("Dateien", "files"))
}

pub fn hits(n: usize) -> String {
    count(n, ("Treffer", "match"), ("Treffer", "matches"))
}

/// „1 Preset“, „3 Presets“, „2 Presets, 1 Profil“.
pub fn count_label(preset_count: usize, profile_count: usize) -> String {
    match (preset_count, profile_count) {
        (_, 0) => presets(preset_count),
        (0, _) => profiles(profile_count),
        _ => format!("{}, {}", presets(preset_count), profiles(profile_count)),
    }
}

/// „5 von 9 werden installiert“.
pub fn will_install(count: usize, total: usize) -> String {
    tr_format!(
        "{count} von {total} werden installiert",
        "{count} of {total} will be installed"
    )
}

/// Beschriftung der Installieren-Schaltfläche.
pub fn install_button(count: usize) -> String {
    if count == 0 {
        tr("Nichts zu installieren", "Nothing to install").to_owned()
    } else {
        tr_format!("{count} installieren", "Install {count}")
    }
}

/// Ergebnis eines Imports für die Einblendung.
pub fn import_result(done: usize, skipped: usize, failed: usize) -> String {
    let mut parts = vec![tr_format!("{} installiert", "{} installed", entries(done))];
    if skipped > 0 {
        parts.push(tr_format!("{skipped} übersprungen", "{skipped} skipped"));
    }
    if failed > 0 {
        parts.push(tr_format!("{failed} fehlgeschlagen", "{failed} failed"));
    }
    parts.join(" · ")
}

/// Warum ein Name abgelehnt wurde.
fn name_problem(problem: NameProblem) -> String {
    match problem {
        NameProblem::Empty => tr("darf nicht leer sein", "must not be empty").to_owned(),
        NameProblem::TooLong { max } => tr_format!(
            "ist länger als {max} Zeichen",
            "is longer than {max} characters"
        ),
        NameProblem::ControlCharacters => {
            tr("enthält Steuerzeichen", "contains control characters").to_owned()
        }
    }
}

/// Welche Schutzgrenze ein Archiv überschritten hat.
fn archive_limit(limit: ArchiveLimit) -> String {
    match limit {
        ArchiveLimit::TooManyEntries { max } => {
            tr_format!("mehr als {max} Einträge", "more than {max} entries")
        }
        ArchiveLimit::TooDeep { max } => tr_format!(
            "mehr als {max} ZIP-Ebenen ineinander",
            "more than {max} levels of nested ZIP files"
        ),
        ArchiveLimit::TooLarge { max_mb } => tr_format!(
            "entpackt größer als {max_mb} MB",
            "larger than {max_mb} MB when unpacked"
        ),
    }
}

/// Übersetzt einen Fehler in einen Satz für den Nutzer.
pub fn describe(error: &CoreError) -> String {
    use std::error::Error;
    let cause = error
        .source()
        .map(|source| format!(" ({source})"))
        .unwrap_or_default();
    match error {
        CoreError::AdobeFolderNotFound { searched } => {
            let searched = searched.display();
            tr_format!(
                "Camera Raw wurde nicht gefunden. Erwartet in: {searched}",
                "Camera Raw was not found. Expected in: {searched}"
            )
        }
        CoreError::NotACameraRawFolder { path } => {
            let path = path.display();
            tr_format!(
                "{path} ist kein Camera-Raw-Ordner (er enthält weder „Settings“ noch „ImportedSettings“).",
                "{path} is not a Camera Raw folder (it contains neither “Settings” nor “ImportedSettings”)."
            )
        }
        CoreError::AdobeOwnFolder { path } => {
            let path = path.display();
            tr_format!(
                "In {path} liegen Adobes mitgelieferte Presets und Profile – diesen Ordner verwaltet Preset Manager nicht. Eigene Presets gehören in den Benutzerordner (Automatisch suchen).",
                "{path} holds the presets and profiles that come with Adobe’s apps – Preset Manager does not manage this folder. Your own presets belong in your user folder (Search automatically)."
            )
        }
        CoreError::Io { path, .. } => {
            let path = path.display();
            tr_format!(
                "„{path}“ konnte nicht gelesen oder geschrieben werden{cause}.",
                "“{path}” could not be read or written{cause}."
            )
        }
        CoreError::InvalidArchive { path, reason } => {
            let name = file_name(path);
            tr_format!(
                "Das Archiv „{name}“ kann nicht gelesen werden ({reason}).",
                "The archive “{name}” cannot be read ({reason})."
            )
        }
        CoreError::UnsafeArchiveEntry { entry } => tr_format!(
            "Das Archiv enthält einen unzulässigen Pfad („{entry}“) und wurde nicht entpackt.",
            "The archive contains an invalid path (“{entry}”) and was not unpacked."
        ),
        CoreError::ArchiveTooLarge { path, limit } => {
            let name = file_name(path);
            let limit = archive_limit(*limit);
            tr_format!(
                "Das Archiv „{name}“ wurde abgelehnt: {limit}.",
                "The archive “{name}” was rejected: {limit}."
            )
        }
        CoreError::UnsupportedSource { path } => {
            let name = file_name(path);
            tr_format!(
                "„{name}“ ist weder ein Ordner noch eine ZIP-Datei.",
                "“{name}” is neither a folder nor a ZIP file."
            )
        }
        CoreError::NotAPreset { path } => {
            let name = file_name(path);
            tr_format!("„{name}“ ist kein Preset.", "“{name}” is not a preset.")
        }
        CoreError::MalformedXmp { path, .. } => {
            let name = file_name(path);
            tr_format!("„{name}“ ist beschädigt.", "“{name}” is damaged.")
        }
        CoreError::InvalidName { name, problem } => {
            let problem = name_problem(*problem);
            tr_format!(
                "Der Name „{name}“ ist nicht zulässig: {problem}.",
                "The name “{name}” is not allowed: {problem}."
            )
        }
        CoreError::NameConflict { name } => tr_format!(
            "In dieser Gruppe gibt es bereits einen Eintrag namens „{name}“.",
            "This group already contains an item named “{name}”."
        ),
        CoreError::GroupNotFound { name } => tr_format!(
            "Die Gruppe „{name}“ gibt es nicht mehr.",
            "The group “{name}” no longer exists."
        ),
        CoreError::NoPresetsFound { source_path } => {
            let name = file_name(source_path);
            tr_format!(
                "In „{name}“ wurden keine Presets gefunden.",
                "No presets were found in “{name}”."
            )
        }
        CoreError::ChangedSinceAction { path } => {
            let name = file_name(path);
            tr_format!(
                "„{name}“ wurde inzwischen geändert (z. B. in Camera Raw) und bleibt, wie sie ist.",
                "“{name}” has been changed since (e.g. in Camera Raw) and is left as it is."
            )
        }
        CoreError::Trash { path, reason } => {
            let name = file_name(path);
            tr_format!(
                "„{name}“ konnte nicht in den Papierkorb gelegt werden ({reason}).",
                "“{name}” could not be moved to the Recycle Bin ({reason})."
            )
        }
        CoreError::AlreadyExists { path } => {
            let name = file_name(path);
            tr_format!("„{name}“ gibt es schon.", "“{name}” already exists.")
        }
        CoreError::OutsideAdobeFolder { .. } => tr(
            "Interner Fehler – das Ziel lag außerhalb der Adobe-Ordner. Es wurde nichts geschrieben.",
            "Internal error – the target was outside the Adobe folders. Nothing was written.",
        )
        .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Die Tests teilen sich die globale Sprache; deshalb ein Test für beide.
    #[test]
    fn spricht_deutsch_und_englisch() {
        set_language(Language::German);
        assert_eq!(join_and(&["A"]), "A");
        assert_eq!(join_and(&["A", "B"]), "A und B");
        assert_eq!(join_and(&["A", "B", "C"]), "A, B und C");
        assert_eq!(count_label(1, 0), "1 Preset");
        assert_eq!(count_label(0, 3), "3 Profile");
        assert_eq!(count_label(2, 1), "2 Presets, 1 Profil");
        assert_eq!(
            running_hint(&["Photoshop"]),
            format!("Photoshop läuft – {}", restart_hint())
        );
        assert!(running_hint(&["Photoshop", "Lightroom Classic"]).contains("laufen"));

        set_language(Language::English);
        assert_eq!(join_and(&["A", "B", "C"]), "A, B and C");
        assert_eq!(count_label(2, 1), "2 presets, 1 profile");
        assert_eq!(entries(1), "1 item");
        assert!(running_hint(&["Photoshop", "Lightroom Classic"]).contains("are running"));
        assert_eq!(
            describe(&CoreError::InvalidName {
                name: String::new(),
                problem: NameProblem::Empty
            }),
            "The name “” is not allowed: must not be empty."
        );
        set_language(Language::German);
    }
}
