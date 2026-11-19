//! Ersetzungen anwenden: Textstellen austauschen, ohne den Rest der Datei
//! anzufassen.

use std::ops::Range;

use quick_xml::Reader;

use crate::preset::xmp::ParseProblem;

/// Eine Ersetzung: Bereich im alten Text und neuer Inhalt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Edit {
    range: Range<usize>,
    replacement: String,
}

impl Edit {
    pub(super) fn new(range: Range<usize>, replacement: String) -> Self {
        Self { range, replacement }
    }
}

/// Setzt den neuen Text zusammen. `None`, wenn sich Ersetzungen überlappen
/// oder ein Bereich ungültig ist – das wäre ein Programmierfehler.
pub(super) fn apply(xml: &str, mut edits: Vec<Edit>) -> Option<String> {
    edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
    let mut result = String::with_capacity(xml.len() + 256);
    let mut copied_until = 0;
    for edit in edits {
        if edit.range.start < copied_until {
            return None;
        }
        result.push_str(xml.get(copied_until..edit.range.start)?);
        result.push_str(&edit.replacement);
        copied_until = edit.range.end;
    }
    result.push_str(xml.get(copied_until..)?);
    Some(result)
}

/// Die aktuelle Leseposition als Index in den Text.
pub(super) fn position(reader: &Reader<&[u8]>) -> Result<usize, ParseProblem> {
    usize::try_from(reader.buffer_position())
        .map_err(|_| ParseProblem::Malformed("Datei zu groß".to_owned()))
}

/// Sucht in einem Start-Tag wie `<rdf:Description a="1" crs:UUID="X">` den
/// Wert des Attributs `key` und gibt seine Lage innerhalb von `tag` zurück.
///
/// Der Tag wurde vorher von `quick-xml` gelesen und ist daher wohlgeformt.
/// Die Funktion geht trotzdem vorsichtig vor und gibt bei allem Unerwarteten
/// `None` zurück. Anführungszeichen werden beachtet: Ein Wert, der wie ein
/// Attribut aussieht (`a='crs:UUID="X"'`), wird nicht verwechselt.
pub(super) fn attribute_value_span(tag: &str, key: &str) -> Option<Range<usize>> {
    let bytes = tag.as_bytes();
    let skip_while = |mut index: usize, condition: &dyn Fn(u8) -> bool| {
        while bytes.get(index).is_some_and(|&byte| condition(byte)) {
            index += 1;
        }
        index
    };
    let is_name_byte =
        |byte: u8| !byte.is_ascii_whitespace() && !matches!(byte, b'=' | b'>' | b'/');

    // `<` und den Elementnamen überspringen.
    let mut index = skip_while(1, &is_name_byte);
    loop {
        index = skip_while(index, &|byte| byte.is_ascii_whitespace());
        let name_start = index;
        index = skip_while(index, &is_name_byte);
        if index == name_start {
            return None; // Ende des Tags erreicht
        }
        let name = tag.get(name_start..index)?;

        index = skip_while(index, &|byte| byte.is_ascii_whitespace());
        if bytes.get(index) != Some(&b'=') {
            return None;
        }
        index = skip_while(index + 1, &|byte| byte.is_ascii_whitespace());
        let quote = *bytes.get(index)?;
        if quote != b'"' && quote != b'\'' {
            return None;
        }
        let value_start = index + 1;
        let value_end = value_start + tag.get(value_start..)?.find(char::from(quote))?;
        if name == key {
            return Some(value_start..value_end);
        }
        index = value_end + 1;
    }
}
