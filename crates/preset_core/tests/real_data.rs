//! Prüfung gegen die echten Presets dieses Rechners – **liest nur**.
//!
//! Läuft nicht automatisch, weil die Dateien nur auf einem Rechner mit
//! Camera Raw vorhanden sind. Aufruf:
//!
//! ```text
//! cargo test -p preset_core --test real_data -- --ignored --nocapture
//! ```
//!
//! Jede Datei wird gelesen, im Speicher geändert und wieder gelesen. Es wird
//! nichts geschrieben.

// In Tests darf ein Fehler den Test sofort abbrechen.
#![allow(clippy::unwrap_used, clippy::panic)]

use preset_core::adobe::AdobeLocations;
use preset_core::library::Library;
use preset_core::preset::{metadata_from_bytes, rewrite_metadata, MetadataChanges};

#[test]
#[ignore = "braucht die echten Adobe-Ordner; liest nur"]
fn aendern_funktioniert_mit_allen_installierten_presets() {
    let locations = AdobeLocations::detect().unwrap();
    let (library, _) = Library::load(&locations);
    let presets: Vec<_> = library.groups().iter().flat_map(|g| &g.presets).collect();
    assert!(!presets.is_empty(), "keine Presets gefunden");

    let mut failures = Vec::new();
    for preset in &presets {
        let original = std::fs::read(&preset.path).unwrap();
        let path = preset.path.as_path();

        // 1. Alle drei Felder ändern und wieder lesen.
        let changes = MetadataChanges {
            name: Some("Neu & <besser> „ü“".to_owned()),
            group: Some("Gruppe \"X\"".to_owned()),
            sort_name: Some("0042".to_owned()),
        };
        let changed = match rewrite_metadata(&original, &changes, path) {
            Ok(changed) => changed,
            Err(error) => {
                failures.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        let read = metadata_from_bytes(&changed, path).unwrap();
        let before = metadata_from_bytes(&original, path).unwrap();
        assert_eq!(read.name, "Neu & <besser> „ü“");
        assert_eq!(read.group, "Gruppe \"X\"");
        assert_eq!(read.sort_name, "0042");
        assert_eq!(read.uuid, before.uuid);
        assert_eq!(read.preset_type, before.preset_type);

        // 2. Hin und zurück: Name und Gruppe auf den alten Wert setzen muss
        //    die Datei Byte für Byte wiederherstellen (sofern beide Felder
        //    vorhanden waren).
        if !before.name.is_empty() && !before.group.is_empty() {
            let there = rewrite_metadata(
                &original,
                &MetadataChanges {
                    name: Some("Zwischenstand".to_owned()),
                    group: Some("Zwischenstand".to_owned()),
                    sort_name: None,
                },
                path,
            )
            .unwrap();
            let back = rewrite_metadata(
                &there,
                &MetadataChanges {
                    name: Some(before.name.clone()),
                    group: Some(before.group.clone()),
                    sort_name: None,
                },
                path,
            )
            .unwrap();
            if back != original {
                failures.push(format!(
                    "{}: Hin und zurück nicht identisch",
                    path.display()
                ));
            }
        }
    }

    println!(
        "{} Presets geprüft, {} Probleme",
        presets.len(),
        failures.len()
    );
    if !failures.is_empty() {
        panic!("{}", failures.join("\n"));
    }
}
