//! Als ZIP exportieren (F-37): Speichern-Dialog, Export, Einblendung.

use preset_core::preset::Preset;

use crate::app::{App, Place, ToastKind};
use crate::texts::{self, tr, tr_format};

use super::suggested_name;

impl App {
    /// Fragt nach dem Ziel und speichert die Einträge als ZIP. `name` ist der
    /// vorgeschlagene Dateiname ohne Endung.
    pub fn export_presets(&mut self, presets: Vec<Preset>, name: &str, now: f64) {
        if presets.is_empty() {
            return;
        }
        let suggested = format!(
            "{}.zip",
            preset_core::import::naming::safe_component(name, "Presets")
        );
        // Der Dialog hält das Fenster an; die Zeit dafür zählt nicht zur
        // Anzeigedauer der Einblendung.
        let opened = std::time::Instant::now();
        let Some(target) = rfd::FileDialog::new()
            .set_title(tr("Als ZIP exportieren", "Export as ZIP"))
            .set_file_name(suggested)
            .add_filter(tr("ZIP-Archiv", "ZIP archive"), &["zip"])
            .save_file()
        else {
            return;
        };
        let now = now + opened.elapsed().as_secs_f64();
        let target = if target.extension().is_none() {
            target.with_extension("zip")
        } else {
            target
        };
        let refs: Vec<&Preset> = presets.iter().collect();
        match preset_core::export::export_zip(&refs, &target) {
            Ok(report) => {
                let file = target
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut text = tr_format!(
                    "{} als „{file}“ gespeichert",
                    "{} saved as “{file}”",
                    texts::entries(report.done_count())
                );
                if report.failed_count() > 0 {
                    text.push_str(&tr_format!(
                        " – {} nicht lesbar, ausgelassen",
                        " – {} unreadable, left out",
                        report.failed_count()
                    ));
                }
                tracing::info!(
                    "Als ZIP exportiert: {} Einträge nach {}",
                    report.done_count(),
                    target.display()
                );
                self.toast(text, ToastKind::Success, now);
            }
            Err(error) => self.toast(texts::describe(&error), ToastKind::Error, now),
        }
    }

    /// Strg+E: die Auswahl exportieren – ohne Auswahl die angezeigte Gruppe.
    pub fn export_selection_or_group(&mut self, now: f64) {
        let selected = self.selected_presets();
        if !selected.is_empty() {
            let name = suggested_name(&selected);
            self.export_presets(selected, &name, now);
        } else if let Some(Place::Group { section, name }) = self.place.clone() {
            let presets: Vec<Preset> = self
                .group_presets(section, &name, self.place_active)
                .into_iter()
                .cloned()
                .collect();
            self.export_presets(presets, &name, now);
        }
    }
}
