//! Aufräumen beim Deinstallieren der Fassung von GitHub.
//!
//! Velopack startet das Programm kurz vor dem Deinstallieren mit eigenen
//! Angaben ([`velopack::VelopackApp::on_before_uninstall_fast_callback`]).
//! Dann gilt:
//!
//! - Deaktivierte Presets und Profile werden wieder aktiviert – ohne
//!   Preset Manager kämen sie sonst kaum wieder in Photoshop und Lightroom.
//! - Die eigenen Ordner verschwinden: `%APPDATA%\PresetManager`
//!   (Einstellungen, leere Gruppen) und `%LOCALAPPDATA%\PresetManager`
//!   (Sicherungen, Protokoll für Rückgängig, Logdateien) – wie bei der
//!   Fassung aus dem Microsoft Store, deren private Ordner Windows löscht.
//! - Die Presets und Profile selbst bleiben; sie gehören zu Photoshop und
//!   Lightroom. Den Programmordner entfernt Velopack.
//!
//! Die Store-Fassung kennt diesen Schritt nicht: Windows führt beim
//! Deinstallieren eines MSIX-Pakets keinen Code der App aus.

use std::path::{Path, PathBuf};

use preset_core::adobe::locations::same_path;
use preset_core::adobe::AdobeLocations;
use preset_core::library::{ops, Library};
use preset_core::preset::Preset;
use preset_core::safe_fs::BackupSession;

use crate::settings;

/// Alles wieder aktivieren und die eigenen Ordner löschen. Scheitert ein
/// Schritt, geht es mit dem nächsten weiter – das Deinstallieren darf daran
/// nicht hängen bleiben.
pub fn clean_up() {
    let own_dirs: Vec<PathBuf> = [folder("APPDATA"), folder("LOCALAPPDATA")]
        .into_iter()
        .flatten()
        .map(|dir| dir.join("PresetManager"))
        .collect();
    clean_up_in(
        &camera_raw_folders(),
        &own_dirs,
        &std::env::temp_dir().join("preset-manager-uninstall"),
    );
}

/// Wie [`clean_up`], mit allen Orten als Angabe – für Tests. `scratch`
/// nimmt die Sicherungen des Reaktivierens auf und wird danach gelöscht.
fn clean_up_in(camera_raw: &[AdobeLocations], own_dirs: &[PathBuf], scratch: &Path) {
    for locations in camera_raw {
        reactivate_all(locations, scratch);
    }
    for dir in own_dirs.iter().map(PathBuf::as_path).chain([scratch]) {
        if dir.is_dir() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn folder(variable: &str) -> Option<PathBuf> {
    std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Alle Camera-Raw-Ordner, in denen die App deaktiviert haben könnte: der
/// gewählte, der übliche und die neben Lightroom-Katalogen.
fn camera_raw_folders() -> Vec<AdobeLocations> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(chosen) = settings::load_settings().camera_raw_dir {
        dirs.push(chosen);
    }
    if let Ok(locations) = AdobeLocations::resolve(None) {
        dirs.push(locations.camera_raw_dir().to_path_buf());
    }
    dirs.extend(
        AdobeLocations::find_all()
            .into_iter()
            .map(|found| found.camera_raw_dir),
    );
    let mut unique: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        if !unique.iter().any(|known| same_path(known, &dir)) {
            unique.push(dir);
        }
    }
    unique
        .iter()
        .filter_map(|dir| AdobeLocations::resolve(Some(dir)).ok())
        .collect()
}

fn reactivate_all(locations: &AdobeLocations, scratch: &Path) {
    let (library, _) = Library::load(locations);
    let inactive: Vec<&Preset> = library
        .groups()
        .iter()
        .flat_map(|group| &group.presets)
        .filter(|preset| !preset.active)
        .collect();
    if inactive.is_empty() {
        return;
    }
    let mut session = BackupSession::new(scratch);
    let _ = ops::set_active(&inactive, true, locations, &mut session);
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRESET: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:PresetType="Normal" crs:UUID="AAAA" crs:Name="Warm" crs:Group="Sommer" crs:Exposure2012="+0.5"/></rdf:RDF></x:xmpmeta>"#;

    #[test]
    fn reaktiviert_und_loescht_die_eigenen_ordner() {
        let root = tempfile::tempdir().unwrap();
        let camera_raw = root.path().join("CameraRaw");
        let locations = AdobeLocations::from_camera_raw_dir(&camera_raw);
        std::fs::create_dir_all(&locations.settings).unwrap();
        // Ein deaktiviertes Preset.
        let inactive_dir = locations.inactive_settings.join("Sommer");
        std::fs::create_dir_all(&inactive_dir).unwrap();
        std::fs::write(inactive_dir.join("Warm.xmp"), PRESET).unwrap();
        let (before, _) = Library::load(&locations);
        let inactive_before = before
            .groups()
            .iter()
            .flat_map(|group| &group.presets)
            .filter(|preset| !preset.active)
            .count();
        assert_eq!(inactive_before, 1, "Testaufbau: ein deaktiviertes Preset");

        let settings_dir = root.path().join("Roaming").join("PresetManager");
        let data_dir = root.path().join("Local").join("PresetManager");
        for dir in [&settings_dir, &data_dir] {
            std::fs::create_dir_all(dir.join("logs")).unwrap();
            std::fs::write(dir.join("logs").join("a.log"), "x").unwrap();
        }
        let scratch = root.path().join("scratch");

        clean_up_in(
            std::slice::from_ref(&locations),
            &[settings_dir.clone(), data_dir.clone()],
            &scratch,
        );

        let (after, _) = Library::load(&locations);
        let presets: Vec<_> = after
            .groups()
            .iter()
            .flat_map(|group| &group.presets)
            .collect();
        assert_eq!(presets.len(), 1);
        assert!(presets[0].active, "wieder aktiv");
        assert!(!settings_dir.exists());
        assert!(!data_dir.exists());
        assert!(!scratch.exists());
        assert!(camera_raw.exists(), "Camera-Raw-Ordner bleibt");
    }
}
