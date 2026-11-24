//! Kurzfassungen aller Einträge für die Suche nach Wirkung (F-49) und den
//! Hinweis „Profil fehlt“ (F-50).
//!
//! Dafür muss jede Datei ganz gelesen werden. Damit das nach jeder Aktion
//! schnell geht, merkt sich der Thread die Kurzfassung je Datei zusammen
//! mit Änderungszeit und Größe und liest nur Geänderte neu. Adobes eigene
//! Profile werden einmal je Programmstart gelesen.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use preset_core::library::{adobe_profile_dirs, profile_status, Library, ProfileIndex};
use preset_core::preset::{read_settings, PresetSummary, PresetType};

use super::PresetInfo;

/// Was der Thread zwischen zwei Einlesevorgängen behält.
#[derive(Default)]
pub(super) struct SummaryCache {
    files: HashMap<PathBuf, (Option<SystemTime>, u64, PresetSummary)>,
    /// Adobes mitgelieferte Profile – ändern sich nur mit einem Update.
    adobe: Option<ProfileIndex>,
}

impl SummaryCache {
    /// Kurzfassung und Profilstand für jeden Eintrag der Bibliothek.
    pub(super) fn update(&mut self, library: &Library) -> HashMap<PathBuf, PresetInfo> {
        let adobe = self
            .adobe
            .get_or_insert_with(|| ProfileIndex::scan(&adobe_profile_dirs()));
        // Profile des Nutzers: aktive zählen wie Adobes, deaktivierte nicht.
        let mut available = ProfileIndex::default();
        let mut inactive = ProfileIndex::default();
        let presets: Vec<_> = library
            .groups()
            .iter()
            .flat_map(|group| &group.presets)
            .collect();
        for preset in &presets {
            if preset.preset_type == PresetType::Profile {
                let index = if preset.active {
                    &mut available
                } else {
                    &mut inactive
                };
                index.add(&preset.name, preset.uuid.as_deref());
            }
        }

        let mut files = HashMap::with_capacity(presets.len());
        let mut infos = HashMap::with_capacity(presets.len());
        for preset in presets {
            let stamp = std::fs::metadata(&preset.path)
                .map(|meta| (meta.modified().ok(), meta.len()))
                .unwrap_or((None, 0));
            let summary = match self.files.remove(&preset.path) {
                Some((modified, len, summary)) if (modified, len) == stamp => summary,
                _ => read_settings(&preset.path)
                    .map(|settings| settings.summary())
                    .unwrap_or_default(),
            };
            let profile = summary.needs_profile.clone().map(|needed| {
                let status = if adobe.contains(&needed) {
                    preset_core::library::ProfileStatus::Installed
                } else {
                    profile_status(&needed, &available, &inactive)
                };
                (needed, status)
            });
            infos.insert(
                preset.path.clone(),
                PresetInfo {
                    summary: summary.clone(),
                    profile,
                },
            );
            files.insert(preset.path.clone(), (stamp.0, stamp.1, summary));
        }
        // Was es nicht mehr gibt, fällt heraus.
        self.files = files;
        infos
    }
}
