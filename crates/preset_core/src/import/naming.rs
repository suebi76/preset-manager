//! Datei- und Ordnernamen für installierte Presets.
//!
//! Namen aus einem ZIP-Archiv können alles enthalten, was das Archivformat
//! erlaubt – auch Zeichen, die Windows in Dateinamen verbietet. Bevor ein
//! solcher Name im Adobe-Ordner verwendet wird, macht dieses Modul daraus
//! einen gültigen Windows-Namen.

/// Zeichen, die Windows in Datei- und Ordnernamen nicht erlaubt.
const FORBIDDEN_CHARACTERS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Namen, die unter Windows Geräte bezeichnen – auch mit Endung (`CON.xmp`).
const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Höchstlänge eines Namens in Zeichen. Windows erlaubt 255, aber viele
/// Programme – womöglich auch Photoshop – kommen nur mit Pfaden bis 260
/// Zeichen zurecht. Der Adobe-Ordner braucht davon rund 55; Paketordner und
/// Dateiname bekommen je 100.
const MAX_NAME_CHARS: usize = 100;

/// Macht aus `name` einen gültigen Windows-Datei- oder Ordnernamen.
///
/// - verbotene Zeichen und Steuerzeichen werden zu `_`
/// - Leerzeichen und Punkte am Ende werden entfernt (Windows schneidet sie
///   sonst stillschweigend ab)
/// - Gerätenamen wie `CON` bekommen ein `_` vorangestellt
/// - zu lange Namen werden gekürzt, die Endung bleibt erhalten
/// - bleibt nichts übrig, wird `fallback` verwendet
pub fn safe_component(name: &str, fallback: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|character| {
            if character.is_control() || FORBIDDEN_CHARACTERS.contains(&character) {
                '_'
            } else {
                character
            }
        })
        .collect();

    let mut name = replaced
        .trim_start()
        .trim_end_matches([' ', '.'])
        .to_owned();
    if name.is_empty() {
        name = fallback.to_owned();
    }

    // Der Vorsatz kommt vor dem Kürzen, damit die Höchstlänge auch mit ihm
    // eingehalten wird.
    let base = name.split('.').next().unwrap_or_default();
    if RESERVED_NAMES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(base.trim_end()))
    {
        name.insert(0, '_');
    }
    shorten(&name).trim_end_matches([' ', '.']).to_owned()
}

/// Kürzt einen Namen auf [`MAX_NAME_CHARS`] Zeichen und erhält dabei eine
/// kurze Endung wie `.xmp`.
fn shorten(name: &str) -> String {
    if name.chars().count() <= MAX_NAME_CHARS {
        return name.to_owned();
    }
    let (stem, extension) = split_extension(name);
    let keep = MAX_NAME_CHARS.saturating_sub(extension.chars().count());
    let mut result: String = stem.chars().take(keep).collect();
    result.push_str(extension);
    result
}

/// Teilt `Warm.xmp` in `("Warm", ".xmp")`. Als Endung gilt nur ein kurzer
/// Teil nach dem letzten Punkt; `Version 1.5 final` hat keine Endung.
fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(dot) if dot > 0 && name.len() - dot <= 12 => name.split_at(dot),
        _ => (name, ""),
    }
}

/// Liefert `Warm (2).xmp` für `Warm.xmp` und `number` = 2.
pub fn numbered(file_name: &str, number: u32) -> String {
    let (stem, extension) = split_extension(file_name);
    format!("{stem} ({number}){extension}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laesst_gueltige_namen_unveraendert() {
        assert_eq!(
            safe_component("Sommerlicht - Bondi.xmp", "x"),
            "Sommerlicht - Bondi.xmp"
        );
        assert_eq!(
            safe_component("Größe & „Kühl“ 📷", "x"),
            "Größe & „Kühl“ 📷"
        );
    }

    #[test]
    fn ersetzt_verbotene_zeichen() {
        assert_eq!(safe_component("a<b>c:d\"e|f?g*h", "x"), "a_b_c_d_e_f_g_h");
        assert_eq!(safe_component("mit\ttab", "x"), "mit_tab");
        assert_eq!(safe_component("a/b\\c", "x"), "a_b_c");
    }

    #[test]
    fn entfernt_punkte_und_leerzeichen_am_ende() {
        assert_eq!(safe_component("  Paket . . ", "x"), "Paket");
        assert_eq!(safe_component("...", "Ersatz"), "Ersatz");
        assert_eq!(safe_component("", "Ersatz"), "Ersatz");
    }

    #[test]
    fn entschaerft_geraetenamen() {
        assert_eq!(safe_component("CON", "x"), "_CON");
        assert_eq!(safe_component("con.xmp", "x"), "_con.xmp");
        assert_eq!(safe_component("Lpt1 .xmp", "x"), "_Lpt1 .xmp");
        assert_eq!(safe_component("CONSOLE.xmp", "x"), "CONSOLE.xmp");
        let long_reserved = format!("CON.{}.xmp", "a".repeat(200));
        assert_eq!(
            safe_component(&long_reserved, "x").chars().count(),
            MAX_NAME_CHARS
        );
    }

    #[test]
    fn kuerzt_lange_namen_und_behaelt_endung() {
        let long = format!("{}.xmp", "ä".repeat(300));

        let result = safe_component(&long, "x");

        assert_eq!(result.chars().count(), MAX_NAME_CHARS);
        assert!(result.ends_with(".xmp"));
    }

    #[test]
    fn nummeriert_vor_der_endung() {
        assert_eq!(numbered("Warm.xmp", 2), "Warm (2).xmp");
        assert_eq!(numbered("Paket", 3), "Paket (3)");
        assert_eq!(
            numbered("Version 1.5 final.xmp", 2),
            "Version 1.5 final (2).xmp"
        );
    }
}
