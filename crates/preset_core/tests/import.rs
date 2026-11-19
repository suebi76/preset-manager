//! Integrationstests für den Import aus Ordnern und ZIP-Archiven.
//!
//! Deckt die Pflichtfälle „Import“ aus `Planung/05-Teststrategie.md` ab.
//! Alle Archive werden hier im Code erzeugt – auch die bösartigen. So ist im
//! Test selbst nachzulesen, was darin steckt, und es liegen keine
//! gefährlichen Dateien im Repository.
//!
//! Gearbeitet wird ausschließlich in temporären Ordnern.

// In Tests darf ein Fehler den Test sofort abbrechen.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use preset_core::adobe::AdobeLocations;
use preset_core::history::revert;
use preset_core::import::{
    scan, scan_with_limits, ArchiveLimits, ConflictPolicy, ImportPlan, PlannedAction, ScannedSource,
};
use preset_core::library::Library;
use preset_core::preset::{read_metadata, Section};
use preset_core::report::{ItemOutcome, OperationReport};
use preset_core::safe_fs::{BackupSession, RecycleBin};
use preset_core::CoreError;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Hilfsmittel
// ---------------------------------------------------------------------------

/// Ein minimales, gültiges Preset.
fn preset(name: &str, group: &str, uuid: &str) -> Vec<u8> {
    format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
    crs:PresetType="Normal"
    crs:UUID="{uuid}"
    crs:Exposure2012="+0.50">
   <crs:Name><rdf:Alt><rdf:li xml:lang="x-default">{name}</rdf:li></rdf:Alt></crs:Name>
   <crs:Group><rdf:Alt><rdf:li xml:lang="x-default">{group}</rdf:li></rdf:Alt></crs:Group>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#
    )
    .into_bytes()
}

/// Ein Foto-Sidecar: gültiges XMP, aber kein Preset.
fn sidecar() -> Vec<u8> {
    br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:Exposure2012="+1.00"/>
</rdf:RDF></x:xmpmeta>"#
        .to_vec()
}

fn uuid(number: u32) -> String {
    format!("{number:032X}")
}

/// Baut ein ZIP-Archiv im Speicher.
fn zip_bytes(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in entries {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn write_file(path: &Path, content: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Ein leerer Camera-Raw-Ordner und ein Ordner für Quellen und Sicherungen.
struct Workspace {
    _temp: TempDir,
    locations: AdobeLocations,
    sources: PathBuf,
    backups: PathBuf,
}

impl Workspace {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let locations = AdobeLocations::from_camera_raw_dir(&temp.path().join("CameraRaw"));
        fs::create_dir_all(&locations.settings).unwrap();
        Self {
            sources: temp.path().join("Quellen"),
            backups: temp.path().join("Sicherungen"),
            locations,
            _temp: temp,
        }
    }

    fn source(&self, relative: &str) -> PathBuf {
        self.sources.join(relative)
    }

    fn plan(&self, sources: Vec<ScannedSource>) -> ImportPlan {
        let (library, _) = Library::load(&self.locations);
        ImportPlan::new(sources, &self.locations, &library)
    }

    fn import(&self, source: &Path, policy: ConflictPolicy) -> (ImportPlan, OperationReport) {
        let plan = self.plan(vec![scan(source).unwrap()]);
        let mut backups = BackupSession::new(&self.backups);
        let report = plan.execute(policy, &mut backups);
        (plan, report)
    }

    fn installed(&self, relative: &str) -> PathBuf {
        self.locations.settings.join(relative)
    }
}

/// Dateien unter `folder`, relativ und mit `/` getrennt, sortiert.
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

fn name_of(path: &Path) -> String {
    read_metadata(path).unwrap().name
}

// ---------------------------------------------------------------------------
// Quellen durchsuchen
// ---------------------------------------------------------------------------

#[test]
fn ordner_mit_unterordnern_liefert_alle_presets() {
    let workspace = Workspace::new();
    let folder = workspace.source("Sommer Pack");
    write_file(
        &folder.join("Warm.xmp"),
        &preset("Warm", "Sommer", &uuid(1)),
    );
    write_file(
        &folder.join("XMP").join("Kalt.XMP"),
        &preset("Kalt", "Sommer", &uuid(2)),
    );
    write_file(
        &folder.join("XMP").join("tief").join("Neutral.xmp"),
        &preset("Neutral", "Sommer", &uuid(3)),
    );
    write_file(&folder.join("Foto.xmp"), &sidecar());
    write_file(&folder.join("Anleitung.pdf"), b"%PDF");
    write_file(
        &folder.join("__MACOSX").join("Warm.xmp"),
        &preset("x", "x", &uuid(9)),
    );
    write_file(&folder.join("._Warm.xmp"), &[0, 5, 22, 7]);

    let scanned = scan(&folder).unwrap();

    assert_eq!(scanned.package_name, "Sommer Pack");
    let mut names: Vec<&str> = scanned
        .presets
        .iter()
        .map(|p| p.metadata.name.as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["Kalt", "Neutral", "Warm"]);
    assert_eq!(
        scanned.report.skipped_count(),
        1,
        "Sidecar wird übersprungen"
    );
    assert_eq!(scanned.report.failed_count(), 0);
}

#[test]
fn zip_verschachteltes_zip_und_macos_zip_liefern_richtige_anzahl() {
    let workspace = Workspace::new();

    let pack = workspace.source("Pack.zip");
    write_file(
        &pack,
        &zip_bytes(&[
            ("Pack/A.xmp", preset("A", "Pack", &uuid(1))),
            ("Pack/XMP/B.xmp", preset("B", "Pack", &uuid(2))),
            ("Pack/Vorschau.jpg", vec![0xFF, 0xD8]),
        ]),
    );

    let inner = zip_bytes(&[("C.xmp", preset("C", "Bonus", &uuid(3)))]);
    let middle = zip_bytes(&[
        ("Bonus.zip", inner),
        ("D.xmp", preset("D", "Bonus", &uuid(4))),
    ]);
    let nested = workspace.source("Nested.zip");
    write_file(
        &nested,
        &zip_bytes(&[("Mitte.zip", middle), ("E.xmp", preset("E", "x", &uuid(5)))]),
    );

    let mac = workspace.source("Mac.zip");
    write_file(
        &mac,
        &zip_bytes(&[
            ("F.xmp", preset("F", "Mac", &uuid(6))),
            ("__MACOSX/._F.xmp", vec![0, 5, 22, 7]),
            ("._G.xmp", vec![0, 5, 22, 7]),
        ]),
    );

    assert_eq!(scan(&pack).unwrap().presets.len(), 2);
    assert_eq!(scan(&nested).unwrap().presets.len(), 3);
    let mac_scan = scan(&mac).unwrap();
    assert_eq!(mac_scan.presets.len(), 1);
    assert!(mac_scan.report.is_clean());

    let shown: Vec<PathBuf> = scan(&nested)
        .unwrap()
        .presets
        .iter()
        .map(|p| p.shown_as.clone())
        .collect();
    assert!(shown.contains(&nested.join("Mitte.zip").join("Bonus.zip").join("C.xmp")));
}

#[test]
fn zip_in_einem_ordner_wird_mit_durchsucht() {
    let workspace = Workspace::new();
    let folder = workspace.source("Download");
    write_file(&folder.join("lose.xmp"), &preset("Lose", "x", &uuid(1)));
    write_file(
        &folder.join("Paket.zip"),
        &zip_bytes(&[("Im Zip.xmp", preset("Im Zip", "x", &uuid(2)))]),
    );

    let scanned = scan(&folder).unwrap();

    assert_eq!(scanned.presets.len(), 2);
}

#[test]
fn quelle_ohne_presets_meldet_no_presets_found() {
    let workspace = Workspace::new();
    let folder = workspace.source("Leer");
    write_file(&folder.join("Foto.xmp"), &sidecar());
    let archive = workspace.source("Bilder.zip");
    write_file(&archive, &zip_bytes(&[("Bild.jpg", vec![0xFF, 0xD8])]));

    assert!(matches!(
        scan(&folder),
        Err(CoreError::NoPresetsFound { .. })
    ));
    assert!(matches!(
        scan(&archive),
        Err(CoreError::NoPresetsFound { .. })
    ));
}

#[test]
fn andere_dateien_und_fehlende_quellen_werden_abgelehnt() {
    let workspace = Workspace::new();
    let rar = workspace.source("Paket.rar");
    write_file(&rar, b"Rar!");
    let broken_zip = workspace.source("Kaputt.zip");
    write_file(&broken_zip, b"das ist kein zip");

    assert!(matches!(
        scan(&rar),
        Err(CoreError::UnsupportedSource { .. })
    ));
    assert!(matches!(
        scan(&broken_zip),
        Err(CoreError::InvalidArchive { .. })
    ));
    assert!(matches!(
        scan(&workspace.source("gibt es nicht")),
        Err(CoreError::Io { .. })
    ));
}

// ---------------------------------------------------------------------------
// Schutz vor bösartigen Archiven
// ---------------------------------------------------------------------------

#[test]
fn zip_slip_wird_abgelehnt_und_nichts_ausserhalb_geschrieben() {
    let workspace = Workspace::new();
    let mut evil_entries = vec!["../../evil.xmp", "/absolut.xmp", "a/../../evil.xmp"];
    if cfg!(windows) {
        evil_entries.extend(["..\\..\\evil.xmp", "C:\\evil.xmp", "C:evil.xmp"]);
    }

    for evil in evil_entries {
        let archive = workspace.source("Boese.zip");
        write_file(
            &archive,
            &zip_bytes(&[
                ("harmlos.xmp", preset("Harmlos", "x", &uuid(1))),
                (evil, preset("Böse", "x", &uuid(2))),
            ]),
        );

        let result = scan(&archive);

        assert!(
            matches!(result, Err(CoreError::UnsafeArchiveEntry { .. })),
            "{evil} wurde nicht abgelehnt: {result:?}"
        );
    }
    assert!(!workspace.sources.join("evil.xmp").exists());
    assert!(!workspace
        .sources
        .parent()
        .unwrap()
        .join("evil.xmp")
        .exists());
    assert!(files_below(&workspace.locations.settings).is_empty());
}

#[test]
fn harmloser_punkt_punkt_pfad_innerhalb_des_archivs_ist_erlaubt() {
    let workspace = Workspace::new();
    let archive = workspace.source("Ok.zip");
    write_file(
        &archive,
        &zip_bytes(&[("a/../b.xmp", preset("B", "x", &uuid(1)))]),
    );

    assert_eq!(scan(&archive).unwrap().presets.len(), 1);
}

#[test]
fn zu_grosses_archiv_wird_abgelehnt() {
    let workspace = Workspace::new();
    let archive = workspace.source("Gross.zip");
    // 2 MB Nullen – stark komprimierbar, wie bei einer ZIP-Bombe.
    let mut big = preset("Gross", "x", &uuid(1));
    big.extend(std::iter::repeat_n(b' ', 2 * 1024 * 1024));
    write_file(&archive, &zip_bytes(&[("Gross.xmp", big)]));
    assert!(fs::metadata(&archive).unwrap().len() < 100 * 1024);

    let limits = ArchiveLimits {
        max_unpacked_bytes: 1024 * 1024,
        ..ArchiveLimits::default()
    };

    assert!(matches!(
        scan_with_limits(&archive, &limits),
        Err(CoreError::ArchiveTooLarge { .. })
    ));
    assert!(scan(&archive).is_ok(), "mit Standardgrenzen erlaubt");
}

#[test]
fn zu_viele_eintraege_und_zu_tiefe_verschachtelung_werden_abgelehnt() {
    let workspace = Workspace::new();

    let many = workspace.source("Viele.zip");
    let entries: Vec<(String, Vec<u8>)> = (0..20)
        .map(|n| (format!("{n}.xmp"), preset("n", "x", &uuid(n))))
        .collect();
    let entry_refs: Vec<(&str, Vec<u8>)> = entries
        .iter()
        .map(|(name, content)| (name.as_str(), content.clone()))
        .collect();
    write_file(&many, &zip_bytes(&entry_refs));

    let mut nested = zip_bytes(&[("tief.xmp", preset("Tief", "x", &uuid(1)))]);
    for level in 0..3 {
        nested = zip_bytes(&[(&format!("Ebene{level}.zip"), nested)]);
    }
    let deep = workspace.source("Tief.zip");
    write_file(&deep, &nested);

    let few_entries = ArchiveLimits {
        max_entries: 10,
        ..ArchiveLimits::default()
    };
    assert!(matches!(
        scan_with_limits(&many, &few_entries),
        Err(CoreError::ArchiveTooLarge { .. })
    ));
    // Vier Ebenen: Tief.zip > Ebene2.zip > Ebene1.zip > Ebene0.zip
    assert!(matches!(
        scan(&deep),
        Err(CoreError::ArchiveTooLarge { .. })
    ));
}

#[test]
fn unzulaessige_windows_namen_im_archiv_werden_bereinigt() {
    let workspace = Workspace::new();
    let archive = workspace.source("Namen.zip");
    write_file(
        &archive,
        &zip_bytes(&[
            ("CON.xmp", preset("Con", "x", &uuid(1))),
            ("Frage?.xmp", preset("Frage", "x", &uuid(2))),
            ("Punkt.xmp.", preset("Punkt", "x", &uuid(3))),
        ]),
    );

    let (_plan, report) = workspace.import(&archive, ConflictPolicy::Skip);

    assert_eq!(report.done_count(), 3, "{report:?}");
    assert_eq!(
        files_below(&workspace.locations.settings),
        ["Namen/Frage_.xmp", "Namen/Punkt.xmp", "Namen/_CON.xmp"]
    );
}

// ---------------------------------------------------------------------------
// Plan und Ausführung
// ---------------------------------------------------------------------------

#[test]
fn installiert_paket_in_eigenen_unterordner() {
    let workspace = Workspace::new();
    let archive = workspace.source("Sommerlicht Presets.zip");
    write_file(
        &archive,
        &zip_bytes(&[
            (
                "Sommerlicht/Bondi.xmp",
                preset("Bondi", "Sommerlicht", &uuid(1)),
            ),
            (
                "Sommerlicht/Byron.xmp",
                preset("Byron", "Sommerlicht", &uuid(2)),
            ),
        ]),
    );

    let (plan, report) = workspace.import(&archive, ConflictPolicy::Skip);

    assert_eq!(plan.summary().new_files, 2);
    assert_eq!(report.done_count(), 2);
    assert!(report.is_clean());
    assert_eq!(
        files_below(&workspace.locations.settings),
        [
            "Sommerlicht Presets/Bondi.xmp",
            "Sommerlicht Presets/Byron.xmp"
        ]
    );
    // Die installierte Datei ist Byte für Byte die aus dem Archiv.
    assert_eq!(
        fs::read(workspace.installed("Sommerlicht Presets/Bondi.xmp")).unwrap(),
        preset("Bondi", "Sommerlicht", &uuid(1))
    );

    let (library, _) = Library::load(&workspace.locations);
    assert_eq!(
        library
            .group(Section::Presets, "Sommerlicht")
            .unwrap()
            .presets
            .len(),
        2
    );
}

#[test]
fn import_in_eine_zielgruppe_aendert_nur_die_kopie() {
    let workspace = Workspace::new();
    let folder = workspace.source("Paket");
    write_file(
        &folder.join("Warm.xmp"),
        &preset("Warm", "Aus der Datei", &uuid(1)),
    );
    let look = String::from_utf8(preset("Film", "Profile aus der Datei", &uuid(2)))
        .unwrap()
        .replace(r#"crs:PresetType="Normal""#, r#"crs:PresetType="Look""#);
    write_file(&folder.join("Film.xmp"), look.as_bytes());
    let original = fs::read(folder.join("Warm.xmp")).unwrap();

    let mut plan = workspace.plan(vec![scan(&folder).unwrap()]);
    plan.set_target_group(Section::Presets, "Meine Auswahl")
        .unwrap();
    let groups: Vec<&str> = plan
        .items()
        .iter()
        .map(|i| plan.group_after_import(i))
        .collect();
    assert_eq!(groups, ["Profile aus der Datei", "Meine Auswahl"]);
    let report = plan.execute(
        ConflictPolicy::Skip,
        &mut BackupSession::new(&workspace.backups),
    );

    assert_eq!(report.done_count(), 2);
    let installed = read_metadata(&workspace.installed("Paket/Warm.xmp")).unwrap();
    assert_eq!(installed.group, "Meine Auswahl");
    assert_eq!(installed.name, "Warm");
    let profile = read_metadata(&workspace.installed("Paket/Film.xmp")).unwrap();
    assert_eq!(
        profile.group, "Profile aus der Datei",
        "Profil behält seine Gruppe"
    );
    assert_eq!(
        fs::read(folder.join("Warm.xmp")).unwrap(),
        original,
        "Quelle unverändert"
    );
}

#[test]
fn ungueltige_zielgruppe_wird_abgelehnt() {
    let workspace = Workspace::new();
    let folder = workspace.source("Paket");
    write_file(&folder.join("Warm.xmp"), &preset("Warm", "x", &uuid(1)));
    let mut plan = workspace.plan(vec![scan(&folder).unwrap()]);

    assert!(matches!(
        plan.set_target_group(Section::Presets, "  "),
        Err(CoreError::InvalidName { .. })
    ));
}

#[test]
fn planen_veraendert_nichts() {
    let workspace = Workspace::new();
    let folder = workspace.source("Paket");
    write_file(&folder.join("A.xmp"), &preset("A", "x", &uuid(1)));

    let plan = workspace.plan(vec![scan(&folder).unwrap()]);

    assert_eq!(plan.items().len(), 1);
    assert_eq!(plan.items()[0].action, PlannedAction::Install);
    assert_eq!(plan.items()[0].target, workspace.installed("Paket/A.xmp"));
    assert!(files_below(&workspace.locations.settings).is_empty());
}

#[test]
fn gleiche_dateinamen_aus_verschiedenen_unterordnern_bekommen_nummern() {
    let workspace = Workspace::new();
    let folder = workspace.source("Paket");
    write_file(
        &folder.join("Hell").join("01.xmp"),
        &preset("Hell 01", "x", &uuid(1)),
    );
    write_file(
        &folder.join("Dunkel").join("01.xmp"),
        &preset("Dunkel 01", "x", &uuid(2)),
    );

    let (_plan, report) = workspace.import(&folder, ConflictPolicy::Skip);

    assert_eq!(report.done_count(), 2);
    assert_eq!(
        files_below(&workspace.locations.settings),
        ["Paket/01 (2).xmp", "Paket/01.xmp"]
    );
}

#[test]
fn doppeltes_preset_im_import_wird_nur_einmal_installiert() {
    let workspace = Workspace::new();
    let folder = workspace.source("Paket");
    // Viele Pakete liefern dieselben Presets für Lightroom und Camera Raw.
    write_file(
        &folder.join("Lightroom").join("Warm.xmp"),
        &preset("Warm", "x", &uuid(1)),
    );
    write_file(
        &folder.join("Camera Raw").join("Warm.xmp"),
        &preset("Warm", "x", &uuid(1)),
    );

    let (plan, report) = workspace.import(&folder, ConflictPolicy::Skip);

    assert_eq!(plan.summary().duplicates_in_import, 1);
    assert_eq!(report.done_count(), 1);
    assert_eq!(report.skipped_count(), 1);
    assert_eq!(
        files_below(&workspace.locations.settings),
        ["Paket/Warm.xmp"]
    );
}

#[test]
fn dublette_wird_installiert_wenn_die_erste_fundstelle_scheitert() {
    let workspace = Workspace::new();
    let folder = workspace.source("Paket");
    let first = folder.join("A Lightroom").join("Warm.xmp");
    write_file(&first, &preset("Warm", "x", &uuid(1)));
    write_file(
        &folder.join("B Camera Raw").join("Warm.xmp"),
        &preset("Warm", "x", &uuid(1)),
    );
    let plan = workspace.plan(vec![scan(&folder).unwrap()]);

    // Die erste Fundstelle wird nach der Vorschau unbrauchbar.
    write_file(&first, b"<kaputt");
    let report = plan.execute(
        ConflictPolicy::Skip,
        &mut BackupSession::new(&workspace.backups),
    );

    assert_eq!(report.failed_count(), 1);
    assert_eq!(report.done_count(), 1, "die zweite Kopie springt ein");
    assert_eq!(
        files_below(&workspace.locations.settings),
        ["Paket/Warm.xmp"]
    );
}

#[test]
fn hinweis_wenn_kennung_schon_anderswo_installiert_ist() {
    let workspace = Workspace::new();
    let elsewhere = workspace
        .locations
        .imported_settings
        .join("Alt")
        .join("Warm.xmp");
    write_file(&elsewhere, &preset("Warm", "x", &uuid(1)));
    let folder = workspace.source("Paket");
    write_file(&folder.join("Warm.xmp"), &preset("Warm", "x", &uuid(1)));

    let plan = workspace.plan(vec![scan(&folder).unwrap()]);

    assert_eq!(plan.items()[0].installed_with_same_uuid, [elsewhere]);
    assert_eq!(plan.summary().already_installed_elsewhere, 1);
}

#[test]
fn schon_anderswo_installierte_presets_sind_abgewaehlt() {
    let workspace = Workspace::new();
    write_file(
        &workspace
            .locations
            .imported_settings
            .join("Alt")
            .join("Warm.xmp"),
        &preset("Warm", "x", &uuid(1)),
    );
    let folder = workspace.source("Paket");
    write_file(&folder.join("Warm.xmp"), &preset("Warm", "x", &uuid(1)));
    write_file(&folder.join("Kalt.xmp"), &preset("Kalt", "x", &uuid(2)));

    let plan = workspace.plan(vec![scan(&folder).unwrap()]);
    let selected: Vec<(&str, bool)> = plan
        .items()
        .iter()
        .map(|item| (item.preset.metadata.name.as_str(), item.selected))
        .collect();
    assert_eq!(selected, [("Kalt", true), ("Warm", false)]);
    assert_eq!(plan.summary().deselected, 1);

    let report = plan.execute(
        ConflictPolicy::Skip,
        &mut BackupSession::new(&workspace.backups),
    );

    assert_eq!(report.done_count(), 1);
    assert_eq!(report.skipped_count(), 1);
    assert_eq!(
        files_below(&workspace.locations.settings),
        ["Paket/Kalt.xmp"]
    );
}

#[test]
fn abgewaehlte_presets_lassen_sich_wieder_auswaehlen() {
    let workspace = Workspace::new();
    write_file(
        &workspace
            .locations
            .imported_settings
            .join("Alt")
            .join("Warm.xmp"),
        &preset("Warm", "x", &uuid(1)),
    );
    let folder = workspace.source("Paket");
    write_file(&folder.join("Warm.xmp"), &preset("Warm", "x", &uuid(1)));
    let mut plan = workspace.plan(vec![scan(&folder).unwrap()]);

    assert!(plan.set_selected(0, true));
    assert!(!plan.set_selected(5, true), "unbekannter Eintrag");
    let report = plan.execute(
        ConflictPolicy::Skip,
        &mut BackupSession::new(&workspace.backups),
    );

    assert_eq!(report.done_count(), 1);
    assert_eq!(
        files_below(&workspace.locations.settings),
        ["Paket/Warm.xmp"]
    );
}

#[test]
fn neue_presets_koennen_abgewaehlt_werden() {
    let workspace = Workspace::new();
    let folder = workspace.source("Paket");
    write_file(&folder.join("Warm.xmp"), &preset("Warm", "x", &uuid(1)));
    let mut plan = workspace.plan(vec![scan(&folder).unwrap()]);

    plan.set_selected(0, false);
    let report = plan.execute(
        ConflictPolicy::Skip,
        &mut BackupSession::new(&workspace.backups),
    );

    assert_eq!(report.skipped_count(), 1);
    assert!(files_below(&workspace.locations.settings).is_empty());
}

#[test]
fn eine_defekte_datei_unter_zehn() {
    let workspace = Workspace::new();
    let folder = workspace.source("Zehn");
    for number in 1..=9 {
        write_file(
            &folder.join(format!("{number}.xmp")),
            &preset(&format!("Nr. {number}"), "Zehn", &uuid(number)),
        );
    }
    let mut broken = preset("Kaputt", "Zehn", &uuid(10));
    broken.truncate(broken.len() / 2);
    write_file(&folder.join("10.xmp"), &broken);

    let scanned = scan(&folder).unwrap();
    assert_eq!(scanned.report.failed_count(), 1);
    let (_plan, report) = workspace.import(&folder, ConflictPolicy::Skip);

    assert_eq!(report.done_count(), 9);
    assert_eq!(files_below(&workspace.locations.settings).len(), 9);
}

#[test]
fn mehrere_quellen_auf_einmal() {
    let workspace = Workspace::new();
    let folder = workspace.source("Ordner");
    write_file(&folder.join("A.xmp"), &preset("A", "x", &uuid(1)));
    let archive = workspace.source("Archiv.zip");
    write_file(
        &archive,
        &zip_bytes(&[("B.xmp", preset("B", "y", &uuid(2)))]),
    );

    let plan = workspace.plan(vec![scan(&folder).unwrap(), scan(&archive).unwrap()]);
    let report = plan.execute(
        ConflictPolicy::Skip,
        &mut BackupSession::new(&workspace.backups),
    );

    assert_eq!(report.done_count(), 2);
    assert_eq!(
        files_below(&workspace.locations.settings),
        ["Archiv/B.xmp", "Ordner/A.xmp"]
    );
}

// ---------------------------------------------------------------------------
// Konfliktregeln
// ---------------------------------------------------------------------------

/// Installiert „Warm“ (alt) und bereitet eine Quelle mit einem neueren
/// „Warm“ unter demselben Dateinamen vor.
fn conflict_setup() -> (Workspace, PathBuf) {
    let workspace = Workspace::new();
    write_file(
        &workspace.installed("Paket/Warm.xmp"),
        &preset("Warm alt", "x", &uuid(1)),
    );
    let folder = workspace.source("Paket");
    write_file(&folder.join("Warm.xmp"), &preset("Warm neu", "x", &uuid(1)));
    (workspace, folder)
}

#[test]
fn konflikt_ueberspringen_laesst_vorhandene_datei_stehen() {
    let (workspace, folder) = conflict_setup();

    let (plan, report) = workspace.import(&folder, ConflictPolicy::Skip);

    assert_eq!(plan.items()[0].action, PlannedAction::Conflict);
    assert_eq!(report.skipped_count(), 1);
    assert_eq!(name_of(&workspace.installed("Paket/Warm.xmp")), "Warm alt");
    assert!(!workspace.backups.exists(), "ohne Ersetzen keine Sicherung");
}

#[test]
fn konflikt_ersetzen_sichert_vorher() {
    let (workspace, folder) = conflict_setup();

    let plan = workspace.plan(vec![scan(&folder).unwrap()]);
    let mut backups = BackupSession::new(&workspace.backups);
    let report = plan.execute(ConflictPolicy::Replace, &mut backups);

    assert_eq!(report.done_count(), 1);
    assert_eq!(name_of(&workspace.installed("Paket/Warm.xmp")), "Warm neu");
    let backup = backups
        .folder()
        .unwrap()
        .join("Settings")
        .join("Paket")
        .join("Warm.xmp");
    assert_eq!(name_of(&backup), "Warm alt");
}

#[test]
fn konflikt_beide_behalten_vergibt_neuen_namen_und_neue_uuid() {
    let (workspace, folder) = conflict_setup();

    let (_plan, report) = workspace.import(&folder, ConflictPolicy::KeepBoth);

    assert_eq!(report.done_count(), 1);
    assert_eq!(
        files_below(&workspace.locations.settings),
        ["Paket/Warm (2).xmp", "Paket/Warm.xmp"]
    );
    let old = read_metadata(&workspace.installed("Paket/Warm.xmp")).unwrap();
    let new = read_metadata(&workspace.installed("Paket/Warm (2).xmp")).unwrap();
    assert_eq!(old.name, "Warm alt");
    assert_eq!(new.name, "Warm neu");
    assert_eq!(old.uuid.as_deref(), Some(uuid(1).as_str()));
    let new_uuid = new.uuid.unwrap();
    assert_ne!(new_uuid, uuid(1));
    assert_eq!(new_uuid.len(), 32);
    assert!(new_uuid
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()));
    assert_eq!(
        report.items()[0].0,
        workspace.installed("Paket/Warm (2).xmp")
    );
}

#[test]
fn konflikt_wird_beim_ausfuehren_neu_geprueft() {
    let workspace = Workspace::new();
    let folder = workspace.source("Paket");
    write_file(&folder.join("Warm.xmp"), &preset("Warm neu", "x", &uuid(1)));
    let plan = workspace.plan(vec![scan(&folder).unwrap()]);
    assert_eq!(plan.items()[0].action, PlannedAction::Install);

    // Zwischen Vorschau und Ausführen taucht die Datei auf.
    write_file(
        &workspace.installed("Paket/Warm.xmp"),
        &preset("Warm alt", "x", &uuid(1)),
    );
    let report = plan.execute(
        ConflictPolicy::Skip,
        &mut BackupSession::new(&workspace.backups),
    );

    assert_eq!(report.skipped_count(), 1);
    assert_eq!(name_of(&workspace.installed("Paket/Warm.xmp")), "Warm alt");
}

#[test]
fn quelle_wird_vor_dem_schreiben_erneut_geprueft() {
    let workspace = Workspace::new();
    let folder = workspace.source("Paket");
    write_file(&folder.join("Warm.xmp"), &preset("Warm", "x", &uuid(1)));
    let plan = workspace.plan(vec![scan(&folder).unwrap()]);

    // Die Quelldatei wird nach der Vorschau beschädigt.
    write_file(&folder.join("Warm.xmp"), b"<kaputt");
    let report = plan.execute(
        ConflictPolicy::Skip,
        &mut BackupSession::new(&workspace.backups),
    );

    assert_eq!(report.failed_count(), 1);
    assert!(matches!(
        report.items()[0].1,
        ItemOutcome::Failed(CoreError::MalformedXmp { .. })
    ));
    assert!(files_below(&workspace.locations.settings).is_empty());
}

// ---------------------------------------------------------------------------
// Rückgängig (F-29)
// ---------------------------------------------------------------------------

#[test]
fn import_rueckgaengig_stellt_den_alten_stand_her() {
    let (workspace, folder) = conflict_setup();
    write_file(&folder.join("Neu.xmp"), &preset("Neu", "x", &uuid(7)));
    let old = fs::read(workspace.installed("Paket/Warm.xmp")).unwrap();
    let plan = workspace.plan(vec![scan(&folder).unwrap()]);
    let mut session = BackupSession::new(&workspace.backups);
    let report = plan.execute(ConflictPolicy::Replace, &mut session);
    assert_eq!(report.done_count(), 2);
    let journal = session.take_journal();
    assert_eq!(journal.changes().len(), 2, "ersetzt und neu angelegt");

    // Neu angelegte Dateien werden gesichert und entfernt, nicht in den
    // Papierkorb gelegt – der echte Papierkorb wird hier nie berührt.
    let report = revert(
        &journal,
        &workspace.locations,
        &RecycleBin,
        &mut BackupSession::new(&workspace.backups),
    );

    assert!(report.is_clean(), "{report:?}");
    assert_eq!(
        files_below(&workspace.locations.settings),
        ["Paket/Warm.xmp"]
    );
    assert_eq!(
        fs::read(workspace.installed("Paket/Warm.xmp")).unwrap(),
        old
    );
}
