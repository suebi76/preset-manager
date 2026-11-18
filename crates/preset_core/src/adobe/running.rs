//! Läuft gerade Photoshop oder Lightroom Classic?
//!
//! Camera Raw liest Presets beim Start und hält sie im Speicher (Versuch P-3).
//! Ändert die App etwas, während ein Adobe-Programm läuft, erscheint die
//! Änderung erst nach dessen Neustart. Die Oberfläche weist darauf hin (F-41);
//! blockiert wird nichts.

use std::path::Path;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Ein Adobe-Programm, das Camera-Raw-Presets liest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AdobeApp {
    /// Photoshop (jede Version).
    Photoshop,
    /// Photoshop (Beta).
    PhotoshopBeta,
    /// Lightroom Classic.
    LightroomClassic,
}

impl AdobeApp {
    /// Der Name, wie ihn der Nutzer kennt.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Photoshop => "Photoshop",
            Self::PhotoshopBeta => "Photoshop (Beta)",
            Self::LightroomClassic => "Lightroom Classic",
        }
    }
}

/// Die gerade laufenden Adobe-Programme, sortiert und ohne Doppelte.
///
/// Lässt sich die Prozessliste nicht lesen, ist das Ergebnis leer – der
/// Hinweis ist eine Hilfe, kein Muss.
pub fn running_adobe_apps() -> Vec<AdobeApp> {
    let Ok(mut system) = System::new() else {
        return Vec::new();
    };
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
    );
    let mut apps: Vec<AdobeApp> = system
        .processes()
        .values()
        .filter_map(|process| classify(&process.name().to_string_lossy(), process.exe()))
        .collect();
    apps.sort();
    apps.dedup();
    apps
}

/// Ordnet einen Prozess einem Adobe-Programm zu.
///
/// Photoshop und Photoshop (Beta) heißen beide `Photoshop.exe`; die Beta
/// erkennt man am Programmordner.
fn classify(process_name: &str, exe: Option<&Path>) -> Option<AdobeApp> {
    match process_name.to_ascii_lowercase().as_str() {
        "photoshop.exe" | "photoshop" => {
            let is_beta = exe.is_some_and(|path| {
                path.to_string_lossy()
                    .to_ascii_lowercase()
                    .contains("(beta)")
            });
            Some(if is_beta {
                AdobeApp::PhotoshopBeta
            } else {
                AdobeApp::Photoshop
            })
        }
        "lightroom.exe" | "lightroom" => Some(AdobeApp::LightroomClassic),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erkennt_adobe_programme() {
        let ps = Path::new(r"C:\Program Files\Adobe\Adobe Photoshop 2027\Photoshop.exe");
        let beta = Path::new(r"C:\Program Files\Adobe\Adobe Photoshop (Beta)\Photoshop.exe");
        let lr = Path::new(r"C:\Program Files\Adobe\Adobe Lightroom Classic\Lightroom.exe");

        assert_eq!(
            classify("Photoshop.exe", Some(ps)),
            Some(AdobeApp::Photoshop)
        );
        assert_eq!(
            classify("Photoshop.exe", Some(beta)),
            Some(AdobeApp::PhotoshopBeta)
        );
        assert_eq!(classify("Photoshop.exe", None), Some(AdobeApp::Photoshop));
        assert_eq!(
            classify("Lightroom.exe", Some(lr)),
            Some(AdobeApp::LightroomClassic)
        );
        assert_eq!(classify("explorer.exe", None), None);
    }

    #[test]
    fn prozessliste_laesst_sich_lesen() {
        // Läuft auf jedem Rechner; prüft nur, dass nichts abstürzt.
        let _ = running_adobe_apps();
    }
}
