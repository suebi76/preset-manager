//! Integrationstest: einen nachgebauten Camera-Raw-Ordner einlesen.
//!
//! Der Ordner wird in einem temporären Verzeichnis aus den Testdaten in
//! `tests/fixtures` zusammengestellt. Der echte Adobe-Ordner wird nie berührt.

// In Tests darf ein Fehler den Test sofort abbrechen.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};

use preset_core::adobe::AdobeLocations;
use preset_core::library::Library;
use preset_core::preset::{PresetOrigin, PresetType, Section};
use preset_core::report::ItemOutcome;
use preset_core::CoreError;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Kopiert eine Testdatei an `target` und legt fehlende Ordner an.
fn place(fixture_name: &str, target: &Path) {
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::copy(fixture(fixture_name), target).unwrap();
}

/// Baut diesen Ordner nach:
///
/// ```text
/// CameraRaw/
/// ├── Settings/
/// │   ├── simple.xmp            Testpaket / Testpreset Warm
/// │   ├── no_group.xmp          (ohne Gruppe) / Ohne Gruppe
/// │   ├── sidecar.xmp           kein Preset
/// │   ├── notiz.txt             keine XMP-Datei
/// │   └── LUTs/Index.dat        Adobe-Cache
/// └── ImportedSettings/
///     └── Paket/
///         ├── special.xmp       Testpaket / Größe …
///         ├── look.xmp          Testpaket / Mit Profil
///         ├── profile.XMP       Testprofile / Testprofil
///         ├── broken.xmp        beschädigt
///         ├── ._special.xmp     macOS-Hilfsdatei, wird übergangen
///         └── __MACOSX/         macOS-Hilfsordner, wird übergangen
/// ```
fn build_camera_raw_dir(root: &Path) -> AdobeLocations {
    let locations = AdobeLocations::from_camera_raw_dir(root);
    let settings = &locations.settings;
    let package = locations.imported_settings.join("Paket");

    place("simple.xmp", &settings.join("simple.xmp"));
    place("no_group.xmp", &settings.join("no_group.xmp"));
    place("sidecar.xmp", &settings.join("sidecar.xmp"));
    fs::write(settings.join("notiz.txt"), "keine XMP-Datei").unwrap();
    fs::create_dir_all(settings.join("LUTs")).unwrap();
    fs::write(settings.join("LUTs").join("Index.dat"), [0u8, 1, 2]).unwrap();

    place("special_chars.xmp", &package.join("special.xmp"));
    place("embedded_look.xmp", &package.join("look.xmp"));
    place("profile.xmp", &package.join("profile.XMP"));
    place("broken.xmp", &package.join("broken.xmp"));

    // macOS-Hilfsdateien sind binär und müssen still übergangen werden.
    fs::write(package.join("._special.xmp"), [0u8, 5, 22, 7, 0xFF]).unwrap();
    place("simple.xmp", &package.join("__MACOSX").join("simple.xmp"));

    locations
}

#[test]
fn liest_gruppen_namen_und_herkunft() {
    let temp = tempfile::tempdir().unwrap();
    let locations = build_camera_raw_dir(temp.path());

    let (library, _report) = Library::load(&locations);

    let group_names: Vec<&str> = library.groups().iter().map(|g| g.name.as_str()).collect();
    assert_eq!(group_names, ["", "Testpaket", "Testprofile"]);
    assert_eq!(library.preset_count(), 5);

    let test_package = library.group(Section::Presets, "Testpaket").unwrap();
    let preset_names: Vec<&str> = test_package
        .presets
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(
        preset_names,
        ["Größe <S> & „Kühl“ 📷", "Mit Profil", "Testpreset Warm"]
    );

    let warm = &test_package.presets[2];
    assert_eq!(warm.origin, PresetOrigin::Settings);
    assert_eq!(warm.path, locations.settings.join("simple.xmp"));
    assert_eq!(
        warm.uuid.as_deref(),
        Some("00000000000000000000000000000001")
    );

    let with_look = &test_package.presets[1];
    assert_eq!(with_look.origin, PresetOrigin::ImportedSettings);
    assert_eq!(
        with_look.uuid.as_deref(),
        Some("00000000000000000000000000000004")
    );

    let profile = &library
        .group(Section::Profiles, "Testprofile")
        .unwrap()
        .presets[0];
    assert_eq!(profile.preset_type, PresetType::Profile);
}

#[test]
fn bericht_nennt_uebersprungene_und_beschaedigte_dateien() {
    let temp = tempfile::tempdir().unwrap();
    let locations = build_camera_raw_dir(temp.path());

    let (_library, report) = Library::load(&locations);

    assert_eq!(report.done_count(), 5);
    assert_eq!(report.skipped_count(), 1);
    assert_eq!(report.failed_count(), 1);

    let skipped = report
        .items()
        .iter()
        .find(|(_, outcome)| matches!(outcome, ItemOutcome::Skipped { .. }))
        .unwrap();
    assert_eq!(skipped.0, locations.settings.join("sidecar.xmp"));

    let failed = report
        .items()
        .iter()
        .find(|(_, outcome)| matches!(outcome, ItemOutcome::Failed(_)))
        .unwrap();
    assert_eq!(
        failed.0,
        locations.imported_settings.join("Paket").join("broken.xmp")
    );
    assert!(matches!(
        failed.1,
        ItemOutcome::Failed(CoreError::MalformedXmp { .. })
    ));
}

#[test]
fn einlesen_veraendert_keine_datei() {
    let temp = tempfile::tempdir().unwrap();
    let locations = build_camera_raw_dir(temp.path());
    let before = snapshot(temp.path());

    let _ = Library::load(&locations);

    assert_eq!(snapshot(temp.path()), before);
}

/// Alle Dateien unter `root` mit ihrem Inhalt, sortiert nach Pfad.
fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push((path.clone(), fs::read(&path).unwrap()));
            }
        }
    }
    files.sort();
    files
}
