//! Befehlszeile auswerten.
//!
//! Übergangslösung bis zur Oberfläche (Meilenstein 4). Bewusst ohne fremde
//! Bibliothek, weil es nur zwei Befehle gibt.

use std::ffi::OsString;
use std::path::PathBuf;

use preset_core::import::ConflictPolicy;

/// Hilfetext für `--hilfe` und bei falschen Angaben.
pub const USAGE: &str = "\
Aufruf:
  preset_manager [--ordner <Camera-Raw-Ordner>] [anzeigen]
  preset_manager [--ordner <Camera-Raw-Ordner>] importieren [--ausfuehren]
                 [--ersetzen | --beide-behalten] [--auch-installierte]
                 <Ordner oder ZIP> ...
  preset_manager fundorte

anzeigen      Listet die installierten Presets nach Gruppen (Standard).
fundorte      Listet alle Camera-Raw-Ordner, die auf diesem Rechner gefunden
              werden: den Benutzerordner und Ordner neben Lightroom-Katalogen.
importieren   Zeigt, was installiert würde. Erst mit --ausfuehren wird kopiert.
              Gibt es eine Datei schon, wird sie übersprungen, außer mit
              --ersetzen (vorher sichern) oder --beide-behalten.
              Presets, die schon an anderer Stelle installiert sind (gleiche
              UUID), werden ausgelassen, außer mit --auch-installierte.
--ordner      Anderer Camera-Raw-Ordner statt %APPDATA%\\Adobe\\CameraRaw.";

/// Was der Nutzer tun möchte.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Installierte Presets auflisten.
    Show,
    /// Presets importieren.
    Import {
        /// Die Ordner und ZIP-Archive.
        sources: Vec<PathBuf>,
        /// Ohne diese Angabe wird nur der Plan gezeigt.
        execute: bool,
        /// Verhalten bei vorhandenen Dateien.
        policy: ConflictPolicy,
        /// Auch Presets installieren, die schon an anderer Stelle installiert
        /// sind (`--auch-installierte`).
        include_installed: bool,
    },
    /// Alle gefundenen Camera-Raw-Ordner auflisten.
    Locations,
    /// Hilfe anzeigen.
    Help,
}

/// Die ausgewerteten Angaben.
#[derive(Debug, PartialEq, Eq)]
pub struct Arguments {
    /// Abweichender Camera-Raw-Ordner (`--ordner`).
    pub camera_raw_dir: Option<PathBuf>,
    /// Der Befehl.
    pub command: Command,
}

/// Wertet die Angaben nach dem Programmnamen aus.
pub fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Arguments, String> {
    let mut camera_raw_dir = None;
    let mut command_name: Option<String> = None;
    let mut sources = Vec::new();
    let mut execute = false;
    let mut policy = ConflictPolicy::Skip;
    let mut policy_given = false;
    let mut include_installed = false;

    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--hilfe" | "-h" | "--help") => {
                return Ok(Arguments {
                    camera_raw_dir,
                    command: Command::Help,
                })
            }
            Some("--ordner") => {
                let folder = arguments.next().ok_or("Nach --ordner fehlt der Ordner.")?;
                camera_raw_dir = Some(PathBuf::from(folder));
            }
            Some("--ausfuehren") => execute = true,
            Some("--auch-installierte") => include_installed = true,
            Some(option @ ("--ersetzen" | "--beide-behalten")) => {
                if policy_given {
                    return Err("--ersetzen und --beide-behalten schließen sich aus.".to_owned());
                }
                policy_given = true;
                policy = if option == "--ersetzen" {
                    ConflictPolicy::Replace
                } else {
                    ConflictPolicy::KeepBoth
                };
            }
            Some(option) if option.starts_with("--") => {
                return Err(format!("Unbekannte Angabe: {option}"));
            }
            Some(name @ ("anzeigen" | "importieren" | "fundorte")) if command_name.is_none() => {
                command_name = Some(name.to_owned());
            }
            _ if command_name.as_deref() == Some("importieren") => {
                sources.push(PathBuf::from(argument));
            }
            _ => {
                return Err(format!(
                    "Unerwartete Angabe: {}",
                    argument.to_string_lossy()
                ))
            }
        }
    }

    let command = match command_name.as_deref() {
        None | Some("anzeigen" | "fundorte") => {
            if execute || policy_given || include_installed {
                return Err("--ausfuehren, --ersetzen, --beide-behalten und \
                     --auch-installierte gibt es nur bei „importieren“."
                    .to_owned());
            }
            if command_name.as_deref() == Some("fundorte") {
                Command::Locations
            } else {
                Command::Show
            }
        }
        _ => {
            if sources.is_empty() {
                return Err("Bitte mindestens einen Ordner oder ein ZIP angeben.".to_owned());
            }
            Command::Import {
                sources,
                execute,
                policy,
                include_installed,
            }
        }
    };
    Ok(Arguments {
        camera_raw_dir,
        command,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(arguments: &[&str]) -> Result<Arguments, String> {
        parse(arguments.iter().map(OsString::from))
    }

    #[test]
    fn ohne_angaben_wird_angezeigt() {
        assert_eq!(parse_str(&[]).unwrap().command, Command::Show);
    }

    #[test]
    fn importieren_mit_allen_angaben() {
        let arguments = parse_str(&[
            "--ordner",
            "D:/CR",
            "importieren",
            "a.zip",
            "--beide-behalten",
            "Ordner",
            "--ausfuehren",
            "--auch-installierte",
        ])
        .unwrap();

        assert_eq!(arguments.camera_raw_dir, Some(PathBuf::from("D:/CR")));
        assert_eq!(
            arguments.command,
            Command::Import {
                sources: vec![PathBuf::from("a.zip"), PathBuf::from("Ordner")],
                execute: true,
                policy: ConflictPolicy::KeepBoth,
                include_installed: true,
            }
        );
    }

    #[test]
    fn lehnt_widerspruechliche_und_unbekannte_angaben_ab() {
        assert!(parse_str(&["importieren", "a", "--ersetzen", "--beide-behalten"]).is_err());
        assert!(parse_str(&["importieren"]).is_err());
        assert!(parse_str(&["--ausfuehren"]).is_err());
        assert!(parse_str(&["--loeschen"]).is_err());
        assert!(parse_str(&["anzeigen", "x"]).is_err());
        assert!(parse_str(&["anzeigen", "--auch-installierte"]).is_err());
        assert!(parse_str(&["fundorte", "--ausfuehren"]).is_err());
    }

    #[test]
    fn fundorte() {
        assert_eq!(
            parse_str(&["fundorte"]).unwrap().command,
            Command::Locations
        );
    }
}
