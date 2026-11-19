//! Explorer öffnen, Camera-Raw-Ordner prüfen und kleine Helfer rund um
//! Dateipfade.

use std::path::{Path, PathBuf};

use preset_core::adobe::AdobeLocations;
use preset_core::preset::{Preset, PresetOrigin};
use preset_core::CoreError;

use crate::texts::{self, tr_format};

/// Öffnet den Explorer und markiert die Datei.
pub fn show_in_explorer(path: &Path) -> Result<(), String> {
    let mut command = std::process::Command::new(explorer());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Der Explorer erwartet `/select,"Pfad"` als ein einziges Argument
        // mit Anführungszeichen nur um den Pfad.
        command.raw_arg(format!("/select,\"{}\"", path.display()));
    }
    #[cfg(not(windows))]
    {
        command.arg(path.parent().unwrap_or(path));
    }
    command.spawn().map(|_| ()).map_err(|error| {
        tr_format!(
            "Explorer konnte nicht geöffnet werden ({error}).",
            "Explorer could not be opened ({error})."
        )
    })
}

/// Der Explorer mit vollem Pfad aus dem Windows-Ordner – nicht über die
/// Suche nach einem Programm dieses Namens, die auch andere Ordner einbezieht.
fn explorer() -> PathBuf {
    std::env::var_os("WINDIR")
        .map(|windows| PathBuf::from(windows).join("explorer.exe"))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("explorer"))
}

/// Öffnet einen Ordner im Explorer; fehlt er, wird er angelegt.
pub fn open_folder(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|error| {
        tr_format!(
            "{} konnte nicht angelegt werden ({error}).",
            "{} could not be created ({error}).",
            path.display()
        )
    })?;
    std::process::Command::new(explorer())
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            tr_format!(
                "Explorer konnte nicht geöffnet werden ({error}).",
                "Explorer could not be opened ({error})."
            )
        })
}

/// Prüft einen gewählten Camera-Raw-Ordner. Wurde der Unterordner `Settings`
/// gewählt, gilt der Ordner darüber.
pub(super) fn check_camera_raw_dir(dir: &Path) -> Result<PathBuf, String> {
    let first = match AdobeLocations::resolve(Some(dir)) {
        Ok(_) => return Ok(dir.to_path_buf()),
        Err(error) => error,
    };
    let is_settings = dir
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("Settings"));
    if let (true, Some(parent)) = (is_settings, dir.parent()) {
        match AdobeLocations::resolve(Some(parent)) {
            Ok(_) => return Ok(parent.to_path_buf()),
            // Die genauere Meldung zuerst.
            Err(error @ CoreError::AdobeOwnFolder { .. }) => return Err(texts::describe(&error)),
            Err(_) => {}
        }
    }
    Err(texts::describe(&first))
}

/// Ob ein Preset über Adobes Importdialog kam.
pub fn is_adobe_origin(preset: &Preset) -> bool {
    preset.origin == PresetOrigin::ImportedSettings
}

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
