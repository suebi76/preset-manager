//! Integrationstests für das Verwalten installierter Presets.
//!
//! Deckt die Pflichtfälle „Verwalten“ aus `Planung/05-Teststrategie.md` ab.
//! Gelöscht wird hier in einen Test-Papierkorb, nie in den echten.

// In Tests darf ein Fehler den Test sofort abbrechen.
#![allow(clippy::unwrap_used)]

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use preset_core::adobe::AdobeLocations;
use preset_core::history::History;
use preset_core::library::{ops, Context, Library};
use preset_core::preset::{read_metadata, Preset, PresetType, Section};
use preset_core::report::{ItemOutcome, OperationReport};
use preset_core::safe_fs::{BackupSession, Trash};
use preset_core::CoreError;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Hilfsmittel
// ---------------------------------------------------------------------------

fn preset(name: &str, group: &str, uuid: u32) -> String {
    format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
    crs:PresetType="Normal"
    crs:UUID="{uuid:032X}"
    crs:Exposure2012="+0.50">
   <crs:Name><rdf:Alt><rdf:li xml:lang="x-default">{name}</rdf:li></rdf:Alt></crs:Name>
   <crs:SortName><rdf:Alt><rdf:li xml:lang="x-default"/></rdf:Alt></crs:SortName>
   <crs:Group><rdf:Alt><rdf:li xml:lang="x-default">{group}</rdf:li></rdf:Alt></crs:Group>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#
    )
}

/// Ein Profil (`crs:PresetType="Look"`) – erscheint in Camera Raw im
/// Profilbrowser, nicht bei den Vorgaben.
fn profile(name: &str, group: &str, uuid: u32) -> String {
    preset(name, group, uuid).replace(r#"crs:PresetType="Normal""#, r#"crs:PresetType="Look""#)
}

/// Ein Papierkorb für Tests: verschiebt in einen eigenen Ordner und merkt
/// sich, was „gelöscht“ wurde.
struct TestTrash {
    folder: PathBuf,
    removed: RefCell<Vec<PathBuf>>,
}

impl Trash for TestTrash {
    fn move_to_trash(&self, path: &Path) -> Result<(), CoreError> {
        let number = self.removed.borrow().len();
        let target = self.folder.join(format!("{number}.xmp"));
        fs::create_dir_all(&self.folder).unwrap();
        fs::rename(path, target).unwrap();
        self.removed.borrow_mut().push(path.to_path_buf());
        Ok(())
    }

    /// Holt die zuletzt unter `original` gelöschte Datei zurück.
    fn restore(&self, original: &Path) -> Result<bool, CoreError> {
        let removed = self.removed.borrow();
        let Some(number) = removed.iter().rposition(|path| path == original) else {
            return Ok(false);
        };
        let stored = self.folder.join(format!("{number}.xmp"));
        if !stored.exists() {
            return Ok(false);
        }
        fs::rename(stored, original).unwrap();
        Ok(true)
    }
}

/// Ein Camera-Raw-Ordner mit:
///
/// ```text
/// Settings/Sommer/Warm.xmp      Sommer / Warm
/// Settings/Sommer/Kalt.xmp      Sommer / Kalt
/// Settings/Sommer/Hell.xmp      Sommer / Hell
/// Settings/Winter.xmp           Winter / Frost
/// Settings/SOMMER.xmp           SOMMER / Laut     (andere Schreibweise!)
/// ```
struct Workspace {
    _temp: TempDir,
    locations: AdobeLocations,
    backups: PathBuf,
    trash: TestTrash,
}

impl Workspace {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let locations = AdobeLocations::from_camera_raw_dir(&temp.path().join("CameraRaw"));
        let settings = &locations.settings;
        fs::create_dir_all(settings.join("Sommer")).unwrap();
        fs::write(
            settings.join("Sommer/Warm.xmp"),
            preset("Warm", "Sommer", 1),
        )
        .unwrap();
        fs::write(
            settings.join("Sommer/Kalt.xmp"),
            preset("Kalt", "Sommer", 2),
        )
        .unwrap();
        fs::write(
            settings.join("Sommer/Hell.xmp"),
            preset("Hell", "Sommer", 3),
        )
        .unwrap();
        fs::write(settings.join("Winter.xmp"), preset("Frost", "Winter", 4)).unwrap();
        fs::write(settings.join("SOMMER.xmp"), preset("Laut", "SOMMER", 5)).unwrap();
        Self {
            backups: temp.path().join("Sicherungen"),
            trash: TestTrash {
                folder: temp.path().join("Papierkorb"),
                removed: RefCell::new(Vec::new()),
            },
            locations,
            _temp: temp,
        }
    }

    fn library(&self) -> Library {
        Library::load(&self.locations).0
    }

    /// Eine neue Sicherungssitzung (eine Aktion).
    fn session(&self) -> BackupSession {
        BackupSession::new(&self.backups)
    }

    fn file(&self, relative: &str) -> PathBuf {
        self.locations.settings.join(relative)
    }

    fn names(&self, group: &str) -> Vec<String> {
        self.library()
            .group(Section::Presets, group)
            .map(|g| g.presets.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default()
    }
}

fn find<'a>(library: &'a Library, name: &str) -> &'a Preset {
    library
        .groups()
        .iter()
        .flat_map(|group| &group.presets)
        .find(|preset| preset.name == name)
        .unwrap()
}

/// Alle Dateien unter `folder`, relativ und mit `/` getrennt, sortiert.
fn files_below(folder: &Path) -> Vec<String> {
    let mut files: Vec<String> = walkdir::WalkDir::new(folder)
        .into_iter()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            entry
                .path()
                .strip_prefix(folder)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    files.sort();
    files
}

// ---------------------------------------------------------------------------
// Löschen
// ---------------------------------------------------------------------------

#[test]
fn loeschen_legt_dateien_in_den_papierkorb() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let warm = find(&library, "Warm");
    let kalt = find(&library, "Kalt");

    let report = ops::delete(
        &[warm, kalt],
        &workspace.locations,
        &workspace.trash,
        &mut workspace.session(),
    );

    assert_eq!(report.done_count(), 2);
    assert_eq!(workspace.trash.removed.borrow().len(), 2);
    assert_eq!(workspace.names("Sommer"), ["Hell"]);
}

#[test]
fn doppelt_ausgewaehltes_preset_wird_einmal_geloescht() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let warm = find(&library, "Warm");

    let report = ops::delete(
        &[warm, warm],
        &workspace.locations,
        &workspace.trash,
        &mut workspace.session(),
    );

    assert_eq!(report.done_count(), 1);
    assert!(report.is_clean());
}

#[test]
fn gruppe_auf_gleichen_namen_umbenennen_schreibt_nichts() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let mut backups = BackupSession::new(&workspace.backups);

    let report = ops::rename_group(
        Section::Presets,
        "Sommer",
        " Sommer ",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    )
    .unwrap();

    assert!(report.items().is_empty());
    assert!(backups.folder().is_none());
}

#[test]
fn leerer_paketordner_wird_entfernt_adobe_ordner_nie() {
    let workspace = Workspace::new();

    let report = ops::delete_group(
        Section::Presets,
        "Sommer",
        &workspace.library(),
        &workspace.locations,
        &workspace.trash,
        &mut workspace.session(),
    )
    .unwrap();

    assert_eq!(report.done_count(), 3);
    assert!(
        !workspace.file("Sommer").exists(),
        "leerer Paketordner bleibt nicht liegen"
    );
    assert!(
        workspace.locations.settings.exists(),
        "Settings selbst bleibt"
    );
    assert_eq!(
        files_below(&workspace.locations.settings),
        ["SOMMER.xmp", "Winter.xmp"]
    );
}

#[test]
fn loeschen_ausserhalb_der_adobe_ordner_wird_abgelehnt() {
    let workspace = Workspace::new();
    let outside = workspace._temp.path().join("Fremd.xmp");
    fs::write(&outside, preset("Fremd", "x", 9)).unwrap();
    let mut stranger = find(&workspace.library(), "Warm").clone();
    stranger.path = outside.clone();

    let report = ops::delete(
        &[&stranger],
        &workspace.locations,
        &workspace.trash,
        &mut workspace.session(),
    );

    assert!(matches!(
        report.items()[0].1,
        ItemOutcome::Failed(CoreError::OutsideAdobeFolder { .. })
    ));
    assert!(outside.exists());
    assert!(workspace.trash.removed.borrow().is_empty());
}

#[test]
fn geaenderte_datei_wird_nicht_geloescht() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let warm = find(&library, "Warm");
    // Zwischen Anzeige und Löschen liegt dort plötzlich etwas anderes.
    fs::write(&warm.path, "keine Preset-Datei").unwrap();

    let report = ops::delete(
        &[warm],
        &workspace.locations,
        &workspace.trash,
        &mut workspace.session(),
    );

    assert_eq!(report.failed_count(), 1);
    assert!(warm.path.exists());
}

#[test]
fn unbekannte_gruppe_meldet_fehler() {
    let workspace = Workspace::new();

    let result = ops::delete_group(
        Section::Presets,
        "Gibt es nicht",
        &workspace.library(),
        &workspace.locations,
        &workspace.trash,
        &mut workspace.session(),
    );

    assert!(matches!(result, Err(CoreError::GroupNotFound { .. })));
}

// ---------------------------------------------------------------------------
// Umbenennen
// ---------------------------------------------------------------------------

#[test]
fn preset_umbenennen_aendert_name_und_dateiname_und_sichert() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let warm = find(&library, "Warm");
    let original = fs::read(&warm.path).unwrap();
    let mut backups = BackupSession::new(&workspace.backups);

    let new_path = ops::rename_preset(
        warm,
        "Sonne & Strand",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    )
    .unwrap();

    assert_eq!(new_path, workspace.file("Sommer/Sonne & Strand.xmp"));
    assert!(!warm.path.exists(), "alte Datei ist weg");
    assert_eq!(read_metadata(&new_path).unwrap().name, "Sonne & Strand");
    assert_eq!(read_metadata(&new_path).unwrap().uuid, warm.uuid);
    let backup = backups.folder().unwrap().join("Settings/Sommer/Warm.xmp");
    assert_eq!(fs::read(backup).unwrap(), original);
}

#[test]
fn umbenennen_auf_vorhandenen_namen_aendert_nichts() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let warm = find(&library, "Warm");
    let before = files_below(&workspace.locations.settings);
    let mut backups = BackupSession::new(&workspace.backups);

    let result = ops::rename_preset(
        warm,
        "KALT",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    );

    assert!(matches!(result, Err(CoreError::NameConflict { .. })));
    assert_eq!(files_below(&workspace.locations.settings), before);
    assert_eq!(read_metadata(&warm.path).unwrap().name, "Warm");
    assert!(
        backups.folder().is_none(),
        "keine Sicherung, weil nichts geändert"
    );
}

#[test]
fn umbenennen_nur_in_der_schreibweise() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let warm = find(&library, "Warm");
    let mut backups = BackupSession::new(&workspace.backups);

    let new_path = ops::rename_preset(
        warm,
        "WARM",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    )
    .unwrap();

    assert_eq!(read_metadata(&new_path).unwrap().name, "WARM");
    assert!(files_below(&workspace.locations.settings).contains(&"Sommer/WARM.xmp".to_owned()));
    assert_eq!(workspace.names("Sommer").len(), 3);
}

#[test]
fn ungueltige_namen_werden_abgelehnt() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let warm = find(&library, "Warm");
    let mut backups = BackupSession::new(&workspace.backups);
    let mut context = Context {
        locations: &workspace.locations,
        library: &library,
        backups: &mut backups,
    };

    for name in ["", "   ", "Zeile\nZwei"] {
        assert!(matches!(
            ops::rename_preset(warm, name, &mut context),
            Err(CoreError::InvalidName { .. })
        ));
    }
}

#[test]
fn gruppe_umbenennen_aendert_genau_diese_gruppe() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let mut backups = BackupSession::new(&workspace.backups);

    let report = ops::rename_group(
        Section::Presets,
        "Sommer",
        "Sommer 2026",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    )
    .unwrap();

    assert_eq!(report.done_count(), 3);
    assert_eq!(workspace.names("Sommer 2026"), ["Hell", "Kalt", "Warm"]);
    assert!(workspace
        .library()
        .group(Section::Presets, "Sommer")
        .is_none());
    assert_eq!(
        workspace.names("SOMMER"),
        ["Laut"],
        "andere Schreibweise unberührt"
    );
    assert_eq!(workspace.names("Winter"), ["Frost"]);
    // Dateien bleiben, wo sie sind – die Gruppe steht in der Datei.
    assert!(workspace.file("Sommer/Warm.xmp").exists());
}

#[test]
fn gruppe_umbenennen_auf_vorhandene_gruppe_wird_abgelehnt() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let mut backups = BackupSession::new(&workspace.backups);

    let result = ops::rename_group(
        Section::Presets,
        "Sommer",
        "Winter",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    );

    assert!(matches!(result, Err(CoreError::NameConflict { .. })));
    assert_eq!(workspace.names("Sommer").len(), 3);
}

// ---------------------------------------------------------------------------
// Presets und Profile getrennt
// ---------------------------------------------------------------------------

/// Legt zusätzlich ein Profil „Film“ in einer Profil-Gruppe „Sommer“ an –
/// gleicher Gruppenname wie die Preset-Gruppe.
fn with_profile(workspace: &Workspace) -> PathBuf {
    let path = workspace.file("Film.xmp");
    fs::write(&path, profile("Film", "Sommer", 7)).unwrap();
    path
}

#[test]
fn gruppe_umbenennen_laesst_gleichnamige_profilgruppe_in_ruhe() {
    let workspace = Workspace::new();
    let film = with_profile(&workspace);
    let library = workspace.library();
    let mut backups = BackupSession::new(&workspace.backups);

    ops::rename_group(
        Section::Presets,
        "Sommer",
        "Sommer 2026",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    )
    .unwrap();

    assert_eq!(read_metadata(&film).unwrap().group, "Sommer");
    let reloaded = workspace.library();
    assert!(reloaded.group(Section::Profiles, "Sommer").is_some());
    assert!(reloaded.group(Section::Presets, "Sommer").is_none());
}

#[test]
fn gruppe_loeschen_betrifft_nur_ihren_bereich() {
    let workspace = Workspace::new();
    let film = with_profile(&workspace);

    let report = ops::delete_group(
        Section::Profiles,
        "Sommer",
        &workspace.library(),
        &workspace.locations,
        &workspace.trash,
        &mut workspace.session(),
    )
    .unwrap();

    assert_eq!(report.done_count(), 1);
    assert!(!film.exists());
    assert_eq!(workspace.names("Sommer").len(), 3, "Presets bleiben");
}

#[test]
fn profil_wird_in_profilgruppe_verschoben_und_legt_sie_an() {
    let workspace = Workspace::new();
    with_profile(&workspace);
    let library = workspace.library();
    let film = library.group(Section::Profiles, "Sommer").unwrap().presets[0].clone();
    assert_eq!(film.preset_type, PresetType::Profile);
    let mut backups = BackupSession::new(&workspace.backups);

    // „Winter“ gibt es nur als Preset-Gruppe; das Profil bekommt eine neue
    // Profil-Gruppe gleichen Namens.
    ops::move_to_group(
        &[&film],
        "Winter",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    )
    .unwrap();

    let reloaded = workspace.library();
    assert_eq!(
        reloaded.group(Section::Profiles, "Winter").unwrap().presets[0].name,
        "Film"
    );
    assert_eq!(
        workspace.names("Winter"),
        ["Frost"],
        "Preset-Gruppe unverändert"
    );
}

#[test]
fn gleiche_namen_in_verschiedenen_bereichen_sind_kein_konflikt() {
    let workspace = Workspace::new();
    fs::write(
        workspace.file("Warm-Profil.xmp"),
        profile("Warm", "Winter", 8),
    )
    .unwrap();
    let library = workspace.library();
    let warm_preset = library
        .group(Section::Presets, "Sommer")
        .unwrap()
        .presets
        .iter()
        .find(|p| p.name == "Warm")
        .unwrap();
    let mut backups = BackupSession::new(&workspace.backups);

    let report = ops::move_to_group(
        &[warm_preset],
        "Winter",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    )
    .unwrap();

    assert_eq!(report.done_count(), 1, "{report:?}");
}

// ---------------------------------------------------------------------------
// Verschieben
// ---------------------------------------------------------------------------

#[test]
fn verschieben_in_andere_gruppe() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let warm = find(&library, "Warm");
    let kalt = find(&library, "Kalt");
    let mut backups = BackupSession::new(&workspace.backups);

    let report = ops::move_to_group(
        &[warm, kalt],
        "Winter",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    )
    .unwrap();

    assert_eq!(report.done_count(), 2);
    assert_eq!(workspace.names("Winter"), ["Frost", "Kalt", "Warm"]);
    assert_eq!(workspace.names("Sommer"), ["Hell"]);
}

#[test]
fn verschieben_mit_namenskonflikt_laesst_dieses_preset_stehen() {
    let workspace = Workspace::new();
    fs::write(
        workspace.file("Winter-Warm.xmp"),
        preset("Warm", "Winter", 6),
    )
    .unwrap();
    let library = workspace.library();
    let warm = library
        .group(Section::Presets, "Sommer")
        .unwrap()
        .presets
        .iter()
        .find(|p| p.name == "Warm")
        .unwrap();
    let kalt = find(&library, "Kalt");
    let mut backups = BackupSession::new(&workspace.backups);

    let report = ops::move_to_group(
        &[warm, kalt],
        "Winter",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    )
    .unwrap();

    assert_eq!(report.done_count(), 1);
    assert!(matches!(
        report.items()[0].1,
        ItemOutcome::Failed(CoreError::NameConflict { .. })
    ));
    assert_eq!(workspace.names("Sommer"), ["Hell", "Warm"]);
}

#[test]
fn verschieben_leert_den_sortierschluessel() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let ordered: Vec<&Preset> = library
        .group(Section::Presets, "Sommer")
        .unwrap()
        .presets
        .iter()
        .collect();
    let mut backups = BackupSession::new(&workspace.backups);
    ops::reorder(
        &ordered,
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    );
    let sorted = workspace.library();
    let warm = find(&sorted, "Warm");
    assert!(!warm.sort_name.is_empty());

    ops::move_to_group(
        &[warm],
        "Winter",
        &mut Context {
            locations: &workspace.locations,
            library: &sorted,
            backups: &mut backups,
        },
    )
    .unwrap();

    let moved = workspace.library();
    assert_eq!(find(&moved, "Warm").sort_name, "");
    assert_eq!(
        workspace.names("Winter"),
        ["Frost", "Warm"],
        "alphabetisch eingeordnet"
    );
}

#[test]
fn verschieben_ohne_gruppe() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let frost = find(&library, "Frost");
    let mut backups = BackupSession::new(&workspace.backups);

    ops::move_to_group(
        &[frost],
        "",
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    )
    .unwrap();

    assert_eq!(workspace.names(""), ["Frost"]);
}

// ---------------------------------------------------------------------------
// Sortieren
// ---------------------------------------------------------------------------

#[test]
fn reihenfolge_bleibt_nach_neuem_einlesen_erhalten() {
    let workspace = Workspace::new();
    let library = workspace.library();
    assert_eq!(workspace.names("Sommer"), ["Hell", "Kalt", "Warm"]);
    let wanted = [
        find(&library, "Warm"),
        find(&library, "Hell"),
        find(&library, "Kalt"),
    ];
    let mut backups = BackupSession::new(&workspace.backups);

    let report = ops::reorder(
        &wanted,
        &mut Context {
            locations: &workspace.locations,
            library: &library,
            backups: &mut backups,
        },
    );

    assert_eq!(report.done_count(), 3);
    assert_eq!(workspace.names("Sommer"), ["Warm", "Hell", "Kalt"]);
    let sort_names: Vec<String> = workspace
        .library()
        .group(Section::Presets, "Sommer")
        .unwrap()
        .presets
        .iter()
        .map(|p| p.sort_name.clone())
        .collect();
    assert_eq!(sort_names, ["0001", "0002", "0003"]);
}

#[test]
fn erneutes_sortieren_in_gleicher_reihenfolge_aendert_nichts() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let wanted: Vec<&Preset> = library
        .group(Section::Presets, "Sommer")
        .unwrap()
        .presets
        .iter()
        .collect();
    let mut backups = BackupSession::new(&workspace.backups);
    let mut context = Context {
        locations: &workspace.locations,
        library: &library,
        backups: &mut backups,
    };
    ops::reorder(&wanted, &mut context);

    let reloaded = workspace.library();
    let again: Vec<&Preset> = reloaded
        .group(Section::Presets, "Sommer")
        .unwrap()
        .presets
        .iter()
        .collect();
    let mut backups = BackupSession::new(&workspace.backups);
    let report = ops::reorder(
        &again,
        &mut Context {
            locations: &workspace.locations,
            library: &reloaded,
            backups: &mut backups,
        },
    );

    assert_eq!(report.done_count(), 0);
    assert_eq!(report.skipped_count(), 3);
}

// ---------------------------------------------------------------------------
// Aktivieren und deaktivieren (F-48)
// ---------------------------------------------------------------------------

#[test]
fn deaktivieren_verschiebt_in_den_eigenen_ordner_und_zurueck() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let warm = find(&library, "Warm").clone();
    let original = fs::read(&warm.path).unwrap();
    assert!(warm.active);

    let (report, moved) = ops::set_active(
        &[&warm],
        false,
        &workspace.locations,
        &mut workspace.session(),
    );

    assert!(report.is_clean(), "{report:?}");
    let inactive = workspace
        .locations
        .inactive_settings
        .join("Sommer/Warm.xmp");
    assert_eq!(moved, vec![inactive.clone()]);
    assert!(!warm.path.exists());
    assert_eq!(fs::read(&inactive).unwrap(), original, "Inhalt unverändert");
    let reloaded = workspace.library();
    let warm_inactive = find(&reloaded, "Warm");
    assert!(!warm_inactive.active);
    assert_eq!(warm_inactive.group, "Sommer", "Gruppe bleibt");
    assert!(find(&reloaded, "Kalt").active);

    let (report, moved) = ops::set_active(
        &[warm_inactive],
        true,
        &workspace.locations,
        &mut workspace.session(),
    );

    assert!(report.is_clean(), "{report:?}");
    assert_eq!(moved, vec![warm.path.clone()]);
    assert_eq!(fs::read(&warm.path).unwrap(), original);
    assert!(!inactive.exists());
    assert!(find(&workspace.library(), "Warm").active);
}

#[test]
fn leerer_paketordner_verschwindet_und_kommt_wieder() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let summer: Vec<&Preset> = library
        .group(Section::Presets, "Sommer")
        .unwrap()
        .presets
        .iter()
        .collect();

    let (report, _) = ops::set_active(
        &summer,
        false,
        &workspace.locations,
        &mut workspace.session(),
    );

    assert!(report.is_clean(), "{report:?}");
    assert!(
        !workspace.file("Sommer").exists(),
        "leerer Paketordner entfernt"
    );
    assert!(workspace.locations.settings.exists(), "Settings bleibt");
    let readme = workspace
        .locations
        .inactive_settings
        .parent()
        .unwrap()
        .join("LIES MICH - README.txt");
    assert!(
        readme.exists(),
        "Erklärung liegt im Ordner der deaktivierten"
    );

    let reloaded = workspace.library();
    let inactive: Vec<&Preset> = reloaded
        .group(Section::Presets, "Sommer")
        .unwrap()
        .presets
        .iter()
        .collect();
    let (report, _) = ops::set_active(
        &inactive,
        true,
        &workspace.locations,
        &mut workspace.session(),
    );
    assert!(report.is_clean(), "{report:?}");
    assert_eq!(workspace.names("Sommer"), vec!["Hell", "Kalt", "Warm"]);
}

#[test]
fn belegtes_ziel_bleibt_unangetastet() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let frost = find(&library, "Frost").clone();
    let blocker = workspace.locations.inactive_settings.join("Winter.xmp");
    fs::create_dir_all(blocker.parent().unwrap()).unwrap();
    fs::write(&blocker, "fremd").unwrap();

    let (report, moved) = ops::set_active(
        &[&frost],
        false,
        &workspace.locations,
        &mut workspace.session(),
    );

    assert!(moved.is_empty());
    assert!(matches!(
        report.items().first().map(|(_, outcome)| outcome),
        Some(ItemOutcome::Failed(CoreError::AlreadyExists { .. }))
    ));
    assert!(frost.path.exists(), "Original bleibt");
    assert_eq!(fs::read_to_string(&blocker).unwrap(), "fremd");
}

#[test]
fn schon_im_gewuenschten_zustand_wird_uebersprungen() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let warm = find(&library, "Warm");

    let (report, moved) = ops::set_active(
        &[warm, warm],
        true,
        &workspace.locations,
        &mut workspace.session(),
    );

    assert!(moved.is_empty());
    assert_eq!(report.items().len(), 1, "doppelt gewählt zählt einmal");
    assert!(matches!(
        report.items().first().map(|(_, outcome)| outcome),
        Some(ItemOutcome::Skipped { .. })
    ));
}

#[test]
fn deaktivierte_lassen_sich_weiter_verwalten() {
    let workspace = Workspace::new();
    let library = workspace.library();
    let frost = find(&library, "Frost").clone();
    ops::set_active(
        &[&frost],
        false,
        &workspace.locations,
        &mut workspace.session(),
    );
    let reloaded = workspace.library();
    let frost = find(&reloaded, "Frost").clone();
    let mut backups = BackupSession::new(workspace.backups.clone());
    let mut context = Context {
        locations: &workspace.locations,
        library: &reloaded,
        backups: &mut backups,
    };

    let renamed = ops::rename_preset(&frost, "Eis", &mut context).unwrap();

    assert!(renamed.starts_with(&workspace.locations.inactive_settings));
    let reloaded = workspace.library();
    assert!(!find(&reloaded, "Eis").active);
}

// ---------------------------------------------------------------------------
// Rückgängig und Wiederholen (F-29)
// ---------------------------------------------------------------------------

/// Führt `action` als eine Aktion aus und merkt sie in `history`.
fn remember(
    workspace: &Workspace,
    history: &mut History,
    label: &str,
    action: impl FnOnce(&mut BackupSession),
) {
    let mut session = workspace.session();
    action(&mut session);
    history.push(label, session.take_journal());
}

fn undo(workspace: &Workspace, history: &mut History) -> OperationReport {
    history
        .undo(
            &workspace.locations,
            &workspace.trash,
            &mut workspace.session(),
        )
        .unwrap()
        .1
}

fn redo(workspace: &Workspace, history: &mut History) -> OperationReport {
    history
        .redo(
            &workspace.locations,
            &workspace.trash,
            &mut workspace.session(),
        )
        .unwrap()
        .1
}

/// Alle Dateien unter dem Camera-Raw-Ordner außer dem Hinweistext der
/// deaktivierten Presets.
fn all_files(workspace: &Workspace) -> Vec<String> {
    files_below(workspace.locations.camera_raw_dir())
        .into_iter()
        .filter(|file| !file.ends_with("README.txt"))
        .collect()
}

#[test]
fn umbenennen_rueckgaengig_und_wiederholen() {
    let workspace = Workspace::new();
    let original_path = workspace.file("Sommer/Warm.xmp");
    let original = fs::read(&original_path).unwrap();
    let mut history = History::new();
    let library = workspace.library();
    let warm = find(&library, "Warm").clone();
    remember(&workspace, &mut history, "Warm umbenennen", |session| {
        let mut context = Context {
            locations: &workspace.locations,
            library: &library,
            backups: session,
        };
        ops::rename_preset(&warm, "Glut", &mut context).unwrap();
    });
    assert_eq!(history.undo_label(), Some("Warm umbenennen"));

    let report = undo(&workspace, &mut history);

    assert!(report.is_clean(), "{report:?}");
    assert_eq!(
        fs::read(&original_path).unwrap(),
        original,
        "byte-gleich zurück"
    );
    assert!(!workspace.file("Sommer/Glut.xmp").exists());
    assert_eq!(history.undo_label(), None);
    assert_eq!(history.redo_label(), Some("Warm umbenennen"));

    let report = redo(&workspace, &mut history);

    assert!(report.is_clean(), "{report:?}");
    assert!(!original_path.exists());
    assert_eq!(
        find(&workspace.library(), "Glut").path,
        workspace.file("Sommer/Glut.xmp")
    );
    assert_eq!(history.undo_label(), Some("Warm umbenennen"));
}

#[test]
fn gruppe_umbenennen_rueckgaengig() {
    let workspace = Workspace::new();
    let before: Vec<(String, Vec<u8>)> = files_below(&workspace.locations.settings)
        .into_iter()
        .map(|file| {
            let content = fs::read(workspace.file(&file)).unwrap();
            (file, content)
        })
        .collect();
    let mut history = History::new();
    let library = workspace.library();
    remember(&workspace, &mut history, "Gruppe", |session| {
        let mut context = Context {
            locations: &workspace.locations,
            library: &library,
            backups: session,
        };
        ops::rename_group(Section::Presets, "Sommer", "Herbst", &mut context).unwrap();
    });
    assert_eq!(workspace.names("Herbst").len(), 3);

    let report = undo(&workspace, &mut history);

    assert_eq!(report.done_count(), 3);
    for (file, content) in before {
        assert_eq!(fs::read(workspace.file(&file)).unwrap(), content, "{file}");
    }
}

#[test]
fn loeschen_rueckgaengig_holt_aus_dem_papierkorb() {
    let workspace = Workspace::new();
    let path = workspace.file("Winter.xmp");
    let original = fs::read(&path).unwrap();
    let mut history = History::new();
    let library = workspace.library();
    let frost = find(&library, "Frost");
    remember(&workspace, &mut history, "Frost löschen", |session| {
        let report = ops::delete(&[frost], &workspace.locations, &workspace.trash, session);
        assert!(report.is_clean());
    });
    assert!(!path.exists());

    let report = undo(&workspace, &mut history);

    assert!(report.is_clean(), "{report:?}");
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(
        files_below(&workspace.trash.folder).is_empty(),
        "aus dem Papierkorb geholt, nicht kopiert"
    );

    redo(&workspace, &mut history);

    assert!(!path.exists(), "wieder im Papierkorb");
    assert_eq!(workspace.trash.removed.borrow().len(), 2);
}

#[test]
fn deaktivieren_rueckgaengig() {
    let workspace = Workspace::new();
    let start = all_files(&workspace);
    let mut history = History::new();
    let library = workspace.library();
    let summer: Vec<&Preset> = library
        .group(Section::Presets, "Sommer")
        .unwrap()
        .presets
        .iter()
        .collect();
    remember(&workspace, &mut history, "deaktivieren", |session| {
        ops::set_active(&summer, false, &workspace.locations, session);
    });
    assert!(!workspace.file("Sommer").exists());

    let report = undo(&workspace, &mut history);

    assert!(report.is_clean(), "{report:?}");
    assert_eq!(all_files(&workspace), start);
    assert!(workspace
        .library()
        .groups()
        .iter()
        .all(|group| group.presets.iter().all(|preset| preset.active)));
}

#[test]
fn inzwischen_geaenderte_dateien_bleiben_unangetastet() {
    let workspace = Workspace::new();
    let mut history = History::new();
    let library = workspace.library();
    remember(&workspace, &mut history, "Gruppe", |session| {
        let mut context = Context {
            locations: &workspace.locations,
            library: &library,
            backups: session,
        };
        ops::rename_group(Section::Presets, "Sommer", "Herbst", &mut context).unwrap();
    });
    // Camera Raw (oder der Nutzer) ändert eine der Dateien danach.
    let edited = workspace.file("Sommer/Kalt.xmp");
    let edited_content = fs::read_to_string(&edited)
        .unwrap()
        .replace("+0.50", "+1.00");
    fs::write(&edited, &edited_content).unwrap();

    let report = undo(&workspace, &mut history);

    assert_eq!(report.done_count(), 2);
    assert_eq!(report.failed_count(), 1);
    assert!(report.items().iter().any(|(path, outcome)| path == &edited
        && matches!(
            outcome,
            ItemOutcome::Failed(CoreError::ChangedSinceAction { .. })
        )));
    assert_eq!(
        fs::read_to_string(&edited).unwrap(),
        edited_content,
        "unverändert"
    );
    assert_eq!(workspace.names("Sommer").len(), 2);
}

#[test]
fn neue_aktion_verwirft_wiederholen() {
    let workspace = Workspace::new();
    let mut history = History::new();
    let library = workspace.library();
    let warm = find(&library, "Warm").clone();
    let kalt = find(&library, "Kalt").clone();
    remember(&workspace, &mut history, "eins", |session| {
        ops::set_active(&[&warm], false, &workspace.locations, session);
    });
    undo(&workspace, &mut history);
    assert_eq!(history.redo_label(), Some("eins"));

    remember(&workspace, &mut history, "zwei", |session| {
        ops::set_active(&[&kalt], false, &workspace.locations, session);
    });

    assert_eq!(history.redo_label(), None);
    assert_eq!(history.undo_label(), Some("zwei"));
}

#[test]
fn mehrere_schritte_nacheinander_zuruecknehmen() {
    let workspace = Workspace::new();
    let start = all_files(&workspace);
    let mut history = History::new();

    let library = workspace.library();
    let warm = find(&library, "Warm").clone();
    remember(&workspace, &mut history, "verschieben", |session| {
        let mut context = Context {
            locations: &workspace.locations,
            library: &library,
            backups: session,
        };
        ops::move_to_group(&[&warm], "Winter", &mut context).unwrap();
    });
    let library = workspace.library();
    let warm = find(&library, "Warm").clone();
    remember(&workspace, &mut history, "deaktivieren", |session| {
        ops::set_active(&[&warm], false, &workspace.locations, session);
    });
    let library = workspace.library();
    let warm = find(&library, "Warm").clone();
    remember(&workspace, &mut history, "umbenennen", |session| {
        let mut context = Context {
            locations: &workspace.locations,
            library: &library,
            backups: session,
        };
        ops::rename_preset(&warm, "Glut", &mut context).unwrap();
    });

    for _ in 0..3 {
        let report = undo(&workspace, &mut history);
        assert!(report.is_clean(), "{report:?}");
    }

    assert_eq!(history.undo_label(), None);
    assert_eq!(all_files(&workspace), start);
    assert_eq!(find(&workspace.library(), "Warm").group, "Sommer");
}
