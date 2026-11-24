//! Wo der Explorer die Dateien der Store-Fassung sieht.
//!
//! Die Store-Fassung läuft als MSIX-Paket. Neue Ordner, die sie direkt unter
//! AppData anlegt – ihre eigenen, `Roaming\PresetManager` und
//! `Local\PresetManager` –, legt Windows in einem privaten Ordner des Pakets
//! ab: `%LOCALAPPDATA%\Packages\<Paketfamilie>\LocalCache\Roaming\…` bzw.
//! `…\LocalCache\Local\…`. Die App selbst merkt davon nichts; der Explorer
//! aber läuft außerhalb des Pakets und sähe unter dem gewohnten Pfad einen
//! leeren Ordner. Der Camera-Raw-Ordner ist nicht betroffen: Er liegt tiefer
//! und wurde von Adobe angelegt (siehe `packaging/msix/AppxManifest.xml`).
//!
//! Die Fassung für GitHub nutzt das nicht: Dort gibt es keine Umleitung.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Für den Explorer: der private Ort einer Datei oder eines Ordners der App,
/// wenn es ihn gibt – sonst der Pfad selbst.
pub fn visible_path(path: &Path) -> PathBuf {
    let roaming = std::env::var_os("APPDATA").map(PathBuf::from);
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let (Some(roaming), Some(local)) = (roaming, local) else {
        return path.to_path_buf();
    };
    own_family(&local)
        .and_then(|family| private_location(path, &roaming, &local, family))
        .filter(|candidate| candidate.exists())
        .unwrap_or_else(|| path.to_path_buf())
}

/// Die Paketfamilie dieses Programms, einmal ermittelt. `None`, wenn es
/// nicht als Paket läuft.
fn own_family(local: &Path) -> Option<&'static str> {
    static FAMILY: OnceLock<Option<String>> = OnceLock::new();
    FAMILY
        .get_or_init(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| package_family(&exe))
                // Lokal eingerichtetes Testpaket (Entwicklermodus): Der Name
                // steht nicht im Pfad des Programms.
                .or_else(|| probe_family(local))
        })
        .as_deref()
}

/// Fragt Windows selbst: eine leere Datei direkt unter `%LOCALAPPDATA%`
/// anlegen – im Paket landet sie im eigenen privaten Ordner –, nachsehen,
/// unter welchem Paket, und wieder löschen.
fn probe_family(local: &Path) -> Option<String> {
    let name = format!("preset-manager-{}.probe", std::process::id());
    let probe = local.join(&name);
    std::fs::write(&probe, b"").ok()?;
    let family = std::fs::read_dir(local.join("Packages"))
        .ok()
        .and_then(|entries| {
            entries.flatten().find(|entry| {
                entry
                    .path()
                    .join("LocalCache")
                    .join("Local")
                    .join(&name)
                    .exists()
            })
        })
        .and_then(|entry| entry.file_name().to_str().map(str::to_owned));
    let _ = std::fs::remove_file(&probe);
    family
}

/// Die Paketfamilie aus dem Ort des Programms:
/// `…\WindowsApps\Name_1.9.2.0_x64__Herausgeber\PresetManager.exe` →
/// `Name_Herausgeber`. `None`, wenn das Programm nicht aus einem
/// installierten Paket läuft.
pub fn package_family(exe: &Path) -> Option<String> {
    let folder = exe.parent()?;
    let in_windows_apps = folder
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name.eq_ignore_ascii_case("WindowsApps"));
    if !in_windows_apps {
        return None;
    }
    // Name_Version_Architektur_Ressource_Herausgeber (Ressource meist leer).
    let parts: Vec<&str> = folder.file_name()?.to_str()?.split('_').collect();
    match (parts.first(), parts.last()) {
        (Some(name), Some(publisher))
            if parts.len() >= 5 && !name.is_empty() && !publisher.is_empty() =>
        {
            Some(format!("{name}_{publisher}"))
        }
        _ => None,
    }
}

/// Der private Ort von `path` im Paket `family`, wenn `path` unter AppData
/// liegt und nicht im Camera-Raw-Ordner.
pub fn private_location(
    path: &Path,
    roaming: &Path,
    local: &Path,
    family: &str,
) -> Option<PathBuf> {
    let cache = local.join("Packages").join(family).join("LocalCache");
    // Zuerst Roaming: Es liegt bei manchen Profilen unter Local.
    if let Ok(rest) = path.strip_prefix(roaming) {
        if is_camera_raw(rest) {
            return None;
        }
        return Some(cache.join("Roaming").join(rest));
    }
    let rest = path.strip_prefix(local).ok()?;
    if rest.starts_with("Packages") {
        return None;
    }
    Some(cache.join("Local").join(rest))
}

fn is_camera_raw(rest: &Path) -> bool {
    let mut components = rest.components();
    let first = components
        .next()
        .map(|c| c.as_os_str().to_ascii_lowercase());
    let second = components
        .next()
        .map(|c| c.as_os_str().to_ascii_lowercase());
    first.as_deref() == Some("adobe".as_ref()) && second.as_deref() == Some("cameraraw".as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_from_installed_package() {
        let exe = Path::new(
            r"C:\Program Files\WindowsApps\S.Schwabe.PresetManager_1.9.2.0_x64__ph3172v6sb7yg\PresetManager.exe",
        );
        assert_eq!(
            package_family(exe).as_deref(),
            Some("S.Schwabe.PresetManager_ph3172v6sb7yg")
        );
    }

    #[test]
    fn no_family_outside_windows_apps() {
        let exe = Path::new(r"C:\Users\anna\.preset-manager-build\store-layout\PresetManager.exe");
        assert_eq!(package_family(exe), None);
        let exe = Path::new(r"C:\Program Files\WindowsApps\Kaputt\PresetManager.exe");
        assert_eq!(package_family(exe), None);
    }

    #[test]
    fn app_folders_map_to_the_package_cache() {
        let roaming = Path::new(r"C:\Users\anna\AppData\Roaming");
        let local = Path::new(r"C:\Users\anna\AppData\Local");
        let family = "S.Schwabe.PresetManager_ph3172v6sb7yg";
        let cache = r"C:\Users\anna\AppData\Local\Packages\S.Schwabe.PresetManager_ph3172v6sb7yg\LocalCache";
        assert_eq!(
            private_location(&local.join(r"PresetManager\logs"), roaming, local, family),
            Some(PathBuf::from(format!(r"{cache}\Local\PresetManager\logs")))
        );
        assert_eq!(
            private_location(&roaming.join("PresetManager"), roaming, local, family),
            Some(PathBuf::from(format!(r"{cache}\Roaming\PresetManager")))
        );
    }

    #[test]
    fn camera_raw_and_other_places_stay_real() {
        let roaming = Path::new(r"C:\Users\anna\AppData\Roaming");
        let local = Path::new(r"C:\Users\anna\AppData\Local");
        let family = "F_x";
        assert_eq!(
            private_location(
                &roaming.join(r"Adobe\CameraRaw\Settings\Gruppe"),
                roaming,
                local,
                family
            ),
            None
        );
        assert_eq!(
            private_location(&roaming.join(r"adobe\cameraraw"), roaming, local, family),
            None
        );
        assert_eq!(
            private_location(
                Path::new(r"D:\Kataloge\Lightroom Settings"),
                roaming,
                local,
                family
            ),
            None
        );
    }
}
