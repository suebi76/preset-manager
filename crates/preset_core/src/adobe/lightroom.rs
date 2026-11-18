//! Was die Einstellungsdatei von Lightroom Classic über Kataloge verrät.
//!
//! Lightroom Classic merkt sich in
//! `%APPDATA%\Adobe\Lightroom\Preferences\Lightroom Classic CC 7 Preferences.agprefs`
//! (Lua-Text) die zuletzt geöffneten Kataloge und, ob „Presets mit diesem
//! Katalog speichern“ eingeschaltet ist. Dann liegen die Presets nicht im
//! Benutzerordner, sondern in einem Ordner neben dem Katalog – siehe
//! [`super::locations`].
//!
//! Die Datei wird nur gelesen, nie geschrieben. Was sich nicht verstehen
//! lässt, wird übergangen: Ein unbekanntes Format führt zu weniger Fundorten,
//! nie zu einem Fehler.

use std::path::PathBuf;

/// Das Wenige, das die App aus der Einstellungsdatei braucht.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LightroomPreferences {
    /// Alle Kataloge (`.lrcat`), die in der Datei vorkommen – zuletzt
    /// geöffnete, der beim Start geladene usw. Ohne Doppelte, in der
    /// Reihenfolge des ersten Auftretens.
    pub catalogs: Vec<PathBuf>,
    /// „Presets mit diesem Katalog speichern“ ist eingeschaltet.
    pub presets_with_catalog: bool,
}

/// Der Schlüssel für „Presets mit diesem Katalog speichern“.
const PRESETS_WITH_CATALOG: &str = "AgTemplateBrowser_storePresetsWithCatalog = true";

/// Liest Kataloge und Schalter aus dem Text der Einstellungsdatei.
///
/// Pfade stehen dort teils einfach (`"E:\\Kataloge\\Bilder.lrcat"`), teils in
/// verschachtelten Zeichenketten mehrfach maskiert
/// (`\"C:\\\\Users\\\\…\\\\Katalog.lrcat\"`). Beides ergibt denselben Pfad.
pub fn parse_preferences(text: &str) -> LightroomPreferences {
    let mut catalogs: Vec<PathBuf> = Vec::new();
    for catalog in catalog_paths(text) {
        let known = catalogs
            .iter()
            .any(|seen| super::locations::same_path(seen, &catalog));
        if !known {
            catalogs.push(catalog);
        }
    }
    LightroomPreferences {
        catalogs,
        presets_with_catalog: text.contains(PRESETS_WITH_CATALOG),
    }
}

/// Alle Pfade, die auf `.lrcat` enden und in Anführungszeichen stehen.
fn catalog_paths(text: &str) -> Vec<PathBuf> {
    const EXTENSION: &str = ".lrcat";
    // Gleiche Länge in Bytes wie `text`, daher passen die Positionen.
    let lower = text.to_ascii_lowercase();
    let mut paths = Vec::new();
    let mut from = 0;
    while let Some(found) = lower[from..].find(EXTENSION) {
        let start = from + found;
        let end = start + EXTENSION.len();
        from = end;
        // `.lrcat-data`, `.lrcat.lock` usw. sind keine Kataloge.
        if !matches!(text[end..].chars().next(), Some('"' | '\\')) {
            continue;
        }
        let Some(quote) = text[..start].rfind('"') else {
            continue;
        };
        if let Some(path) = unescape(&text[quote + 1..end]) {
            paths.push(path);
        }
    }
    paths
}

/// Macht aus jeder Folge von `\` einen einzelnen – außer am Anfang eines
/// Netzwerkpfads (`\\server\…`). Gibt `None` zurück, wenn das Ergebnis nicht
/// wie ein absoluter Windows-Pfad aussieht.
fn unescape(raw: &str) -> Option<PathBuf> {
    let leading = raw.chars().take_while(|&c| c == '\\').count();
    let mut path = String::with_capacity(raw.len());
    if leading > 0 {
        path.push_str(r"\\");
    }
    let mut previous_backslash = false;
    for c in raw.chars().skip(leading) {
        if c == '\\' {
            if !previous_backslash {
                path.push('\\');
            }
            previous_backslash = true;
        } else {
            path.push(c);
            previous_backslash = false;
        }
    }

    let is_drive = path.as_bytes().get(1) == Some(&b':')
        && path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic);
    let is_network = leading > 0 && path.len() > 2;
    (is_drive || is_network).then(|| PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gekürzter Auszug aus einer echten Einstellungsdatei (Pfade geändert).
    const SAMPLE: &str = r#"prefs = {
	Adobe_checkCatalogIntegrityForPaths = "catalogsToCheck = {\
	[\"C:\\\\Users\\\\Anna\\\\Bilder\\\\Lightroom\\\\Lightroom Catalog.lrcat\"] = {\
		catalogType = \"lr\",\
	},\
}\
",
	AgTemplateBrowser_storePresetsWithCatalog = true,
	AgTemplateBrowser_storePresetsWithCatalog_oldValue = false,
	SelectLibraryDialog_newCatalogDir = "E:\\Kataloge",
	libraryToLoad20 = "E:\\Kataloge\\Reisen\\Reisen.lrcat",
	recentLibraries20 = "recentLibraries = {\
	\"C:\\\\Users\\\\Anna\\\\Bilder\\\\Lightroom\\\\Lightroom Catalog.lrcat\",\
	\"E:\\\\Kataloge\\\\Reisen\\\\Reisen.lrcat\",\
	\"\\\\\\\\nas\\\\fotos\\\\Archiv.LRCAT\",\
}\
",
	someOther = "E:\\Kataloge\\Reisen\\Reisen.lrcat-data",
}
"#;

    #[test]
    fn findet_kataloge_ohne_doppelte() {
        let prefs = parse_preferences(SAMPLE);

        assert_eq!(
            prefs.catalogs,
            vec![
                PathBuf::from(r"C:\Users\Anna\Bilder\Lightroom\Lightroom Catalog.lrcat"),
                PathBuf::from(r"E:\Kataloge\Reisen\Reisen.lrcat"),
                PathBuf::from(r"\\nas\fotos\Archiv.LRCAT"),
            ]
        );
    }

    #[test]
    fn erkennt_den_schalter() {
        assert!(parse_preferences(SAMPLE).presets_with_catalog);

        let off = SAMPLE.replace(
            "storePresetsWithCatalog = true",
            "storePresetsWithCatalog = false",
        );
        assert!(!parse_preferences(&off).presets_with_catalog);
    }

    #[test]
    fn unbekannter_text_ergibt_nichts() {
        assert_eq!(parse_preferences(""), LightroomPreferences::default());
        assert_eq!(
            parse_preferences(r#"x = "nur.lrcat" y = ".lrcat""#).catalogs,
            Vec::<PathBuf>::new()
        );
    }
}
