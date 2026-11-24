//! Was in einer Version neu ist – aus `CHANGELOG.md`.
//!
//! Jeder Abschnitt dort heißt `## [1.11.2] – Datum` und hat die Teile
//! `### English` und `### Deutsch`. Angezeigt wird der Teil in der Sprache
//! der Oberfläche.
//!
//! - Fassung von GitHub: Velopack bringt den Abschnitt der neuen Version mit
//!   ([`crate::updates`]); er erscheint **vor** dem Installieren.
//! - Store-Fassung: Der Store installiert selbst. Deshalb steckt der
//!   CHANGELOG im Programm, und nach einem Update zeigt die App einmal, was
//!   neu ist ([`since`]).

use std::cmp::Ordering;

use crate::texts::{self, Language};

/// Der CHANGELOG, wie er beim Bauen im Projekt lag.
const CHANGELOG: &str = include_str!("../../../CHANGELOG.md");

/// Eine Version mit ihren Notizen in der Sprache der Oberfläche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionNotes {
    pub version: String,
    pub text: String,
}

/// Der Teil eines Abschnitts in einer Sprache – ohne die Überschrift
/// `### …`. Fehlt die Unterteilung, der ganze Abschnitt.
pub fn in_language(section: &str, language: Language) -> String {
    let wanted = match language {
        Language::German => "### Deutsch",
        Language::English => "### English",
    };
    let Some(start) = section.find(wanted) else {
        return section.trim().to_owned();
    };
    let rest = &section[start + wanted.len()..];
    let end = rest.find("\n### ").unwrap_or(rest.len());
    rest[..end].trim().to_owned()
}

/// Alle Versionen aus dem eingebauten CHANGELOG nach `after` bis
/// einschließlich `until`, die neueste zuerst.
pub fn since(after: &str, until: &str) -> Vec<VersionNotes> {
    sections(CHANGELOG)
        .into_iter()
        .filter(|(version, _)| {
            compare(version, after) == Ordering::Greater
                && compare(version, until) != Ordering::Greater
        })
        .map(|(version, body)| VersionNotes {
            text: in_language(body, texts::language()),
            version,
        })
        .collect()
}

/// Nur die Notizen einer Version aus dem eingebauten CHANGELOG.
pub fn of(version: &str) -> Vec<VersionNotes> {
    sections(CHANGELOG)
        .into_iter()
        .filter(|(found, _)| found == version)
        .map(|(version, body)| VersionNotes {
            text: in_language(body, texts::language()),
            version,
        })
        .collect()
}

/// Die Abschnitte eines CHANGELOG: Version und Inhalt.
fn sections(changelog: &str) -> Vec<(String, &str)> {
    let mut found = Vec::new();
    let mut rest = changelog;
    while let Some(start) = rest.find("\n## [") {
        let after = &rest[start + 5..];
        let Some(close) = after.find(']') else { break };
        let version = after[..close].to_owned();
        let body_start = after.find('\n').map_or(after.len(), |line| line + 1);
        let body = &after[body_start..];
        let body_end = body.find("\n## [").unwrap_or(body.len());
        found.push((version, &body[..body_end]));
        rest = &after[body_start + body_end..];
    }
    found
}

/// Vergleicht Versionsnummern Zahl für Zahl; ein Zusatz wie `-beta.1`
/// zählt weniger als die Version ohne.
pub fn compare(a: &str, b: &str) -> Ordering {
    let split = |version: &str| {
        let (numbers, suffix) = version.split_once('-').unwrap_or((version, ""));
        let numbers: Vec<u64> = numbers
            .split('.')
            .map(|part| part.trim().parse().unwrap_or(0))
            .collect();
        (numbers, suffix.is_empty())
    };
    let (a_numbers, a_final) = split(a);
    let (b_numbers, b_final) = split(b);
    a_numbers
        .cmp(&b_numbers)
        .then_with(|| a_final.cmp(&b_final))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Änderungen\n\nText.\n\n## [1.11.2] – 2026-11-24\n\n### English\n\n- **Security:** fixed.\n\n### Deutsch\n\n- **Sicherheit:** behoben.\n\n## [1.11.1] – 2026-11-24\n\n### English\n\n- Clean uninstall.\n\n### Deutsch\n\n- Sauber deinstallieren.\n\n## [1.9.0] – 2026-11-19\n\nOhne Unterteilung.\n";

    #[test]
    fn findet_alle_abschnitte() {
        let found = sections(SAMPLE);
        let versions: Vec<&str> = found.iter().map(|(v, _)| v.as_str()).collect();
        assert_eq!(versions, ["1.11.2", "1.11.1", "1.9.0"]);
    }

    #[test]
    fn nimmt_die_sprache() {
        let (_, body) = sections(SAMPLE).remove(0);
        assert_eq!(
            in_language(body, Language::German),
            "- **Sicherheit:** behoben."
        );
        assert_eq!(
            in_language(body, Language::English),
            "- **Security:** fixed."
        );
    }

    #[test]
    fn ohne_unterteilung_der_ganze_abschnitt() {
        let (_, body) = sections(SAMPLE).remove(2);
        assert_eq!(in_language(body, Language::German), "Ohne Unterteilung.");
    }

    #[test]
    fn vergleicht_versionen() {
        assert_eq!(compare("1.11.0", "1.9.2"), Ordering::Greater);
        assert_eq!(compare("1.11.2", "1.11.2"), Ordering::Equal);
        assert_eq!(compare("1.12.0-beta.1", "1.12.0"), Ordering::Less);
    }

    #[test]
    fn eingebauter_changelog_hat_die_aktuelle_version() {
        let found = sections(CHANGELOG);
        assert!(found
            .iter()
            .any(|(version, _)| version == env!("CARGO_PKG_VERSION")));
    }
}
