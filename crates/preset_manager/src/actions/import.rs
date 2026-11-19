//! Import starten: Ordner oder ZIP wählen, durchsuchen, Plan ausführen.

use std::path::PathBuf;

use crate::app::{App, Place, ToastKind};
use crate::texts::{self, tr};
use crate::worker::Command;

impl App {
    /// Durchsucht Ordner/ZIPs und öffnet danach die Importvorschau.
    pub fn start_import(&mut self, paths: Vec<PathBuf>, target: Option<Place>, now: f64) {
        if self.scanning || self.import.is_some() || self.managing {
            self.toast(texts::wait_for_action().to_owned(), ToastKind::Info, now);
            return;
        }
        let target = match target {
            Some(Place::Group { section, name }) if !name.is_empty() => Some((section, name)),
            _ => None,
        };
        self.scanning = true;
        self.send(Command::Scan { paths, target });
    }

    /// Führt den Plan des offenen Importdialogs aus.
    pub fn execute_import(&mut self) {
        let Some(dialog) = &mut self.import else {
            return;
        };
        let Some(plan) = dialog.plan.take() else {
            return;
        };
        self.landing = plan
            .items()
            .iter()
            .find(|item| item.selected)
            .map(|item| Place::Group {
                section: item.preset.metadata.preset_type.section(),
                name: plan.group_after_import(item).to_owned(),
            });
        let policy = dialog.policy;
        // Importiert wird immer in die aktiven Presets.
        self.place_active = true;
        self.send(Command::Execute { plan, policy });
    }

    /// Ordnerauswahl (blockiert, solange der Windows-Dialog offen ist).
    pub fn pick_folder(&mut self, now: f64) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title(texts::import_folder())
            .pick_folder()
        {
            self.start_import(vec![folder], None, now);
        }
    }

    /// ZIP-Auswahl, mehrere Dateien möglich (F-04).
    pub fn pick_zips(&mut self, now: f64) {
        if let Some(files) = rfd::FileDialog::new()
            .set_title(texts::import_zip())
            .add_filter(tr("ZIP-Archive", "ZIP archives"), &["zip"])
            .pick_files()
        {
            self.start_import(files, None, now);
        }
    }
}
