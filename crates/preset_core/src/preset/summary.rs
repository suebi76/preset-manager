//! Die Kurzfassung eines Presets für Suche und Hinweise: was es bewirkt
//! und auf welches Profil es angewiesen ist.
//!
//! Die Oberfläche braucht das für alle Einträge auf einmal (Suche nach
//! Wirkung, F-49; fehlende Profile, F-50) – deshalb klein und ohne die
//! Rohwerte, die [`PresetSettings`] mitschleppt.

use super::adjustments::Fact;
use super::settings::PresetSettings;

/// Ein Profil, auf das ein Preset verweist.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProfileRef {
    /// Der Name, wie Camera Raw ihn anzeigt.
    pub name: String,
    /// Die Kennung (`crs:UUID`), falls angegeben. Darüber findet Camera Raw
    /// das Profil; der Name dient nur zur Anzeige.
    pub uuid: Option<String>,
}

/// Was ein Preset bewirkt und was es braucht.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PresetSummary {
    /// Die Aussagen über die Wirkung, wie [`PresetSettings::facts`].
    pub facts: Vec<Fact>,
    /// Das Profil, das installiert sein muss: Das Preset verweist darauf,
    /// enthält dessen Farbtabelle aber nicht selbst. `None`, wenn das Preset
    /// kein Profil nutzt oder es mitbringt.
    pub needs_profile: Option<ProfileRef>,
}

impl PresetSettings {
    /// Die Kurzfassung für Suche und Hinweise.
    pub fn summary(&self) -> PresetSummary {
        let needs_profile = self
            .look
            .as_ref()
            .filter(|_| self.color_tables.is_empty())
            .filter(|look| !look.name.is_empty() || look.uuid.is_some())
            .map(|look| ProfileRef {
                name: look.name.clone(),
                uuid: look.uuid.clone(),
            });
        PresetSummary {
            facts: self.facts(),
            needs_profile,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::preset::settings::parse_settings_for_tests;

    use super::*;

    fn summary(xml: &str) -> PresetSummary {
        parse_settings_for_tests(xml).summary()
    }

    #[test]
    fn verweis_ohne_farbtabelle_braucht_das_profil() {
        let s = summary(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:PresetType="Normal" crs:Name="P" crs:GrainAmount="30">
             <crs:Look><rdf:Description crs:Name="Film warm" crs:UUID="ABC" crs:Amount="1"/></crs:Look>
            </rdf:Description></rdf:RDF></x:xmpmeta>"#,
        );
        assert_eq!(
            s.needs_profile,
            Some(ProfileRef {
                name: "Film warm".to_owned(),
                uuid: Some("ABC".to_owned())
            })
        );
        assert!(s.facts.contains(&Fact::Grain));
    }

    #[test]
    fn ohne_verweis_braucht_es_nichts() {
        let s = summary(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:PresetType="Normal" crs:Name="P" crs:Exposure2012="+1.00"/>
            </rdf:RDF></x:xmpmeta>"#,
        );
        assert_eq!(s.needs_profile, None);
        assert!(s.facts.contains(&Fact::Brighter));
    }
}
