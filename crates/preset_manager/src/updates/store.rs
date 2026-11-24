//! Die Fassung für den Microsoft Store: Sie sucht nie selbst nach neuen
//! Versionen und lädt nichts aus dem Netz – das übernimmt der Store.
//! Dieselben Aufrufe wie [`super::github`], damit die Oberfläche nicht
//! unterscheiden muss.

use eframe::egui;

use crate::release_notes::VersionNotes;
use crate::settings::{Settings, UpdateMode};
use crate::texts::tr_format;

use super::{Status, UpdateEvent};

pub struct Updater {
    pub status: Status,
    /// Version, deren Hinweis weggeklickt wurde (bleibt hier leer).
    pub dismissed: Option<String>,
    /// Die laufende Version.
    pub current_version: String,
}

impl Updater {
    pub fn new(_settings: &Settings) -> Self {
        let current_version = env!("CARGO_PKG_VERSION").to_owned();
        tracing::info!("Updates: Version {current_version}, über den Microsoft Store");
        Self {
            status: Status::NotInstalled,
            dismissed: None,
            current_version,
        }
    }

    pub fn is_installed(&self) -> bool {
        false
    }

    pub fn busy(&self) -> bool {
        false
    }

    pub fn found_version(&self) -> Option<&str> {
        None
    }

    /// Die Store-Fassung kennt keine anstehende Version.
    pub fn notes(&self) -> Vec<VersionNotes> {
        Vec::new()
    }

    pub fn status_text(&self) -> String {
        let current = &self.current_version;
        tr_format!(
            "Version {current} – neue Versionen installiert der Microsoft Store.",
            "Version {current} – new versions are installed by the Microsoft Store."
        )
    }

    pub fn settings_changed(&mut self, _settings: &Settings, _now: f64) {}

    pub fn poll(
        &mut self,
        _ctx: &egui::Context,
        _now: f64,
        _mode: UpdateMode,
    ) -> Option<UpdateEvent> {
        None
    }

    pub fn check(&mut self, _ctx: &egui::Context) {}

    pub fn install_now(&mut self, _ctx: &egui::Context) -> Option<UpdateEvent> {
        None
    }

    pub fn apply_and_restart(&mut self) -> Result<(), String> {
        Ok(())
    }

    pub fn on_exit(&mut self, _mode: UpdateMode) {}
}
