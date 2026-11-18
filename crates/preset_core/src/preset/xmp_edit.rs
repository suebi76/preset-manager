//! Kennfelder in einer Preset-Datei gezielt ändern.
//!
//! Ein Preset enthält hunderte Reglerwerte, die die App nicht kennt. Deshalb
//! wird die Datei nicht eingelesen und neu geschrieben, sondern **nur an den
//! Stellen geändert, an denen der alte Wert steht**. Alles andere – Reihenfolge,
//! Einrückung, Zeilenumbrüche, unbekannte Felder – bleibt Byte für Byte
//! erhalten (Entscheidung E-5 in `Planung/03-Architektur.md`).
//!
//! Der Ablauf ist immer gleich:
//!
//! 1. [`locate`] liest die Datei mit `quick-xml` und merkt sich, **wo** die
//!    Felder des obersten `rdf:Description` stehen (Byte-Bereiche).
//! 2. Daraus entsteht eine Liste von Ersetzungen ([`Edit`]).
//! 3. [`apply`] setzt den neuen Text zusammen.
//! 4. Das Ergebnis wird erneut gelesen und mit dem Erwarteten verglichen. Nur
//!    wenn alles stimmt, wird es zurückgegeben.
//!
//! Ein Feld kann auf drei Arten in der Datei stehen; alle werden unterstützt:
//!
//! ```text
//! crs:Name="Warm"                                            Attribut
//! <crs:Name><rdf:Alt><rdf:li xml:lang="x-default">Warm</rdf:li></rdf:Alt></crs:Name>
//! <crs:SortName><rdf:Alt><rdf:li xml:lang="x-default"/></rdf:Alt></crs:SortName>   leer
//! ```
//!
//! Fehlt ein Feld ganz, wird es am Ende des obersten `rdf:Description`
//! eingefügt.

use std::ops::Range;
use std::path::Path;

use quick_xml::escape::escape;
use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::CoreError;
use crate::preset::xmp::{
    decode_utf8, is_top_level_description, parse_metadata, ParseProblem, PresetMetadata,
};

/// Die gewünschten Änderungen. `None` heißt: Feld bleibt, wie es ist.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MetadataChanges {
    /// Neuer Anzeigename (`crs:Name`).
    pub name: Option<String>,
    /// Neue Gruppe (`crs:Group`). Leer = ohne Gruppe.
    pub group: Option<String>,
    /// Neuer Sortierschlüssel (`crs:SortName`). Leer = nach Name sortieren.
    pub sort_name: Option<String>,
}

/// Ändert Name, Gruppe und/oder Sortierschlüssel eines Presets.
///
/// Mehrsprachige Felder bekommen den neuen Wert in **allen** Sprachvarianten,
/// damit Camera Raw in jeder Sprache denselben Namen zeigt.
///
/// # Fehler
///
/// - [`CoreError::InvalidName`], wenn ein Wert Steuerzeichen enthält (in XML
///   nicht darstellbar)
/// - [`CoreError::NotAPreset`] / [`CoreError::MalformedXmp`], wenn die Datei
///   kein lesbares Preset ist oder das Ergebnis nicht dem Erwarteten
///   entspricht
pub fn rewrite_metadata(
    bytes: &[u8],
    changes: &MetadataChanges,
    path: &Path,
) -> Result<Vec<u8>, CoreError> {
    for value in [&changes.name, &changes.group, &changes.sort_name]
        .into_iter()
        .flatten()
    {
        check_value(value)?;
    }

    let to_error = |problem: ParseProblem| problem.into_core_error(path);
    let xml = decode_utf8(bytes).map_err(to_error)?;
    let before = parse_metadata(xml).map_err(to_error)?;
    let document = locate(xml).map_err(to_error)?;

    let mut edits = Vec::new();
    let mut expected = before.clone();
    let requested = [
        (Field::Name, &changes.name, &mut expected.name),
        (Field::Group, &changes.group, &mut expected.group),
        (Field::SortName, &changes.sort_name, &mut expected.sort_name),
    ];
    for (field, value, expected_value) in requested {
        if let Some(value) = value {
            edits.extend(document.edits_for(field, value, xml));
            expected_value.clone_from(value);
        }
    }

    finish(bytes, xml, edits, &expected, path)
}

/// Ersetzt die Kennung (`crs:UUID`) des Presets durch `new_uuid`.
///
/// Gebraucht beim Import mit „beide behalten“: Die Kopie soll eine eigene
/// Kennung haben (Versuch P-4 in `Planung/02-Adobe-Grundlagen.md`). Hat die
/// Datei keine Kennung, bleibt sie unverändert. Erwartet werden 32
/// Hexadezimalziffern wie bei Adobe.
///
/// # Fehler
///
/// Wie [`rewrite_metadata`].
pub fn replace_uuid(bytes: &[u8], new_uuid: &str, path: &Path) -> Result<Vec<u8>, CoreError> {
    let to_error = |problem: ParseProblem| problem.into_core_error(path);
    let xml = decode_utf8(bytes).map_err(to_error)?;
    let before = parse_metadata(xml).map_err(to_error)?;
    if before.uuid.is_none() {
        return Ok(bytes.to_vec());
    }
    let document = locate(xml).map_err(to_error)?;

    let edits = document.edits_for(Field::Uuid, new_uuid, xml);
    let expected = PresetMetadata {
        uuid: Some(new_uuid.to_owned()),
        ..before
    };
    finish(bytes, xml, edits, &expected, path)
}

/// Werte mit Steuerzeichen (Zeilenumbruch, Tabulator, Nullzeichen …) lehnt
/// die App ab: XML kann sie nicht oder nur verfälscht speichern, und in einer
/// Preset-Liste haben sie nichts zu suchen.
fn check_value(value: &str) -> Result<(), CoreError> {
    if value.chars().any(char::is_control) {
        return Err(CoreError::InvalidName {
            name: value.to_owned(),
            reason: "enthält Steuerzeichen".to_owned(),
        });
    }
    Ok(())
}

/// Wendet die Ersetzungen an und prüft das Ergebnis.
fn finish(
    bytes: &[u8],
    xml: &str,
    edits: Vec<Edit>,
    expected: &PresetMetadata,
    path: &Path,
) -> Result<Vec<u8>, CoreError> {
    let malformed = |reason: &str| CoreError::MalformedXmp {
        path: path.to_path_buf(),
        reason: reason.to_owned(),
    };
    let new_xml = apply(xml, edits).ok_or_else(|| malformed("Änderungen überlappen sich"))?;

    let after = parse_metadata(&new_xml).map_err(|problem| problem.into_core_error(path))?;
    if &after != expected {
        return Err(malformed(
            "geänderte Datei liefert nicht die erwarteten Werte",
        ));
    }

    let (byte_order_mark, _) = bytes.split_at(bytes.len() - xml.len());
    Ok([byte_order_mark, new_xml.as_bytes()].concat())
}

// ---------------------------------------------------------------------------
// Wo steht was?
// ---------------------------------------------------------------------------

/// Die Felder, die geändert werden können.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Group,
    SortName,
    Uuid,
}

impl Field {
    const ALL: [Self; 4] = [Self::Name, Self::Group, Self::SortName, Self::Uuid];

    fn qualified_name(self) -> &'static str {
        match self {
            Self::Name => "crs:Name",
            Self::Group => "crs:Group",
            Self::SortName => "crs:SortName",
            Self::Uuid => "crs:UUID",
        }
    }

    /// Name, Gruppe und Sortierschlüssel sind mehrsprachig (`rdf:Alt`), die
    /// Kennung ist einfacher Text.
    fn is_language_alternative(self) -> bool {
        self != Self::Uuid
    }

    fn from_qualified_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|field| field.qualified_name() == name)
    }
}

/// Wo ein Feld in der Datei steht.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Location {
    /// Als Attribut: Bereich des Werts ohne Anführungszeichen.
    Attribute(Range<usize>),
    /// Als leeres Element `<crs:Name/>`: Bereich des ganzen Tags.
    EmptyElement(Range<usize>),
    /// Als Element mit Inhalt.
    Element {
        /// Bereich zwischen Start- und End-Tag.
        content: Range<usize>,
        /// Die Sprachvarianten darin.
        items: Vec<ListItem>,
    },
}

/// Eine Sprachvariante (`rdf:li`).
#[derive(Debug, Clone, PartialEq, Eq)]
enum ListItem {
    /// `<rdf:li …>Text</rdf:li>`: Bereich des Texts.
    Text(Range<usize>),
    /// `<rdf:li …/>`: Bereich des ganzen Tags.
    EmptyTag(Range<usize>),
}

/// Lage des obersten `rdf:Description` und seiner Felder.
#[derive(Debug, Default)]
struct Layout {
    /// Bereich des Start-Tags.
    start_tag: Range<usize>,
    /// `true` bei `<rdf:Description …/>` ohne Inhalt.
    is_empty: bool,
    /// Anfang des End-Tags `</rdf:Description>`.
    end_tag_start: usize,
    fields: Vec<(Field, Location)>,
}

/// Ein Element, das gerade gelesen wird.
struct OpenField {
    field: Field,
    depth: usize,
    content_start: usize,
    items: Vec<ListItem>,
    open_item_start: Option<usize>,
}

/// Alle obersten `rdf:Description` einer Datei.
///
/// Adobe schreibt genau eines. Andere Programme verteilen die Felder
/// gelegentlich auf mehrere; dann wird ein Feld überall geändert, wo es
/// vorkommt, damit kein alter Wert in der Datei zurückbleibt.
#[derive(Debug)]
struct Document {
    descriptions: Vec<Layout>,
}

impl Document {
    /// Die Ersetzungen, die `field` auf `value` setzen.
    fn edits_for(&self, field: Field, value: &str, xml: &str) -> Vec<Edit> {
        let escaped = escape(value).into_owned();
        let mut edits = Vec::new();
        let mut found = false;
        for description in &self.descriptions {
            if let Some(existing) = description.existing_field_edits(field, &escaped, xml) {
                edits.extend(existing);
                found = true;
            }
        }
        if !found && !value.is_empty() {
            if let Some(first) = self.descriptions.first() {
                edits.push(first.insertion(full_element(field, &escaped)));
            }
        }
        edits
    }
}

/// Liest die Datei einmal durch und merkt sich die Lage der Felder.
fn locate(xml: &str) -> Result<Document, ParseProblem> {
    let mut reader = Reader::from_str(xml);
    let mut open_elements: Vec<String> = Vec::new();
    let mut descriptions: Vec<Layout> = Vec::new();
    let mut layout: Option<Layout> = None;
    let mut open_field: Option<OpenField> = None;

    loop {
        let start = position(&reader)?;
        let event = reader
            .read_event()
            .map_err(|error| ParseProblem::Malformed(error.to_string()))?;
        let end = position(&reader)?;

        match &event {
            Event::Start(element) | Event::Empty(element) => {
                let name: String = element.name().as_ref().to_owned();
                let is_empty = matches!(event, Event::Empty(_));

                if layout.is_none()
                    && name == "rdf:Description"
                    && open_elements
                        .last()
                        .is_some_and(|parent| parent == "rdf:RDF")
                {
                    let tag = xml.get(start..end).unwrap_or_default();
                    let mut found = Layout {
                        start_tag: start..end,
                        is_empty,
                        end_tag_start: end,
                        fields: Vec::new(),
                    };
                    for field in Field::ALL {
                        if let Some(value) = attribute_value_span(tag, field.qualified_name()) {
                            let range = start + value.start..start + value.end;
                            found.fields.push((field, Location::Attribute(range)));
                        }
                    }
                    if is_empty {
                        descriptions.push(found);
                    } else {
                        layout = Some(found);
                    }
                } else if let Some(open) = open_field.as_mut() {
                    if name == "rdf:li" {
                        if is_empty {
                            open.items.push(ListItem::EmptyTag(start..end));
                        } else {
                            open.open_item_start = Some(end);
                        }
                    }
                } else if let Some(found) = layout.as_mut() {
                    let field = Field::from_qualified_name(&name)
                        .filter(|field| !found.fields.iter().any(|(known, _)| known == field));
                    if let (Some(field), true) = (field, is_top_level_description(&open_elements)) {
                        if is_empty {
                            found
                                .fields
                                .push((field, Location::EmptyElement(start..end)));
                        } else {
                            open_field = Some(OpenField {
                                field,
                                depth: open_elements.len(),
                                content_start: end,
                                items: Vec::new(),
                                open_item_start: None,
                            });
                        }
                    }
                }
                if !is_empty {
                    open_elements.push(name);
                }
            }
            Event::End(_) => {
                let closed = open_elements.pop().unwrap_or_default();
                if let Some(open) = open_field.as_mut() {
                    if closed == "rdf:li" {
                        if let Some(text_start) = open.open_item_start.take() {
                            open.items.push(ListItem::Text(text_start..start));
                        }
                    }
                    if open_elements.len() == open.depth {
                        let location = Location::Element {
                            content: open.content_start..start,
                            items: std::mem::take(&mut open.items),
                        };
                        if let Some(found) = layout.as_mut() {
                            found.fields.push((open.field, location));
                        }
                        open_field = None;
                    }
                } else if let Some(mut found) = layout.take() {
                    if closed == "rdf:Description"
                        && open_elements.last().is_some_and(|p| p == "rdf:RDF")
                    {
                        found.end_tag_start = start;
                        descriptions.push(found);
                    } else {
                        layout = Some(found);
                    }
                }
            }
            Event::Eof => {
                if descriptions.is_empty() {
                    return Err(ParseProblem::Malformed(
                        "kein vollständiges rdf:Description gefunden".to_owned(),
                    ));
                }
                return Ok(Document { descriptions });
            }
            _ => {}
        }
    }
}

impl Layout {
    /// Die Ersetzungen für `field`, falls es in diesem `rdf:Description`
    /// vorkommt; sonst `None`.
    fn existing_field_edits(&self, field: Field, escaped: &str, xml: &str) -> Option<Vec<Edit>> {
        let location = self
            .fields
            .iter()
            .find(|(known, _)| *known == field)
            .map(|(_, location)| location)?;

        let edits = match location {
            Location::Attribute(range) => vec![Edit::new(range.clone(), escaped.to_owned())],
            Location::EmptyElement(range) => {
                vec![Edit::new(range.clone(), full_element(field, escaped))]
            }
            Location::Element { content, items } if items.is_empty() => {
                let inner = if field.is_language_alternative() {
                    alternative(escaped)
                } else {
                    escaped.to_owned()
                };
                vec![Edit::new(content.clone(), inner)]
            }
            Location::Element { items, .. } => items
                .iter()
                .map(|item| match item {
                    ListItem::Text(range) => Edit::new(range.clone(), escaped.to_owned()),
                    ListItem::EmptyTag(range) => {
                        let tag = xml.get(range.clone()).unwrap_or("<rdf:li>");
                        Edit::new(range.clone(), filled_list_item(tag, escaped))
                    }
                })
                .collect(),
        };
        Some(edits)
    }

    /// Fügt ein neues Element am Ende des obersten `rdf:Description` ein.
    fn insertion(&self, element: String) -> Edit {
        if self.is_empty {
            // `<rdf:Description …/>` wird zu `<rdf:Description …>NEU</rdf:Description>`.
            let close_start = self.start_tag.end - 2;
            Edit::new(
                close_start..self.start_tag.end,
                format!(">{element}</rdf:Description>"),
            )
        } else {
            Edit::new(
                self.end_tag_start..self.end_tag_start,
                format!(" {element}\n  "),
            )
        }
    }
}

/// `<crs:Name><rdf:Alt><rdf:li xml:lang="x-default">Wert</rdf:li></rdf:Alt></crs:Name>`
fn full_element(field: Field, escaped_value: &str) -> String {
    let name = field.qualified_name();
    let inner = if field.is_language_alternative() {
        alternative(escaped_value)
    } else {
        escaped_value.to_owned()
    };
    format!("<{name}>{inner}</{name}>")
}

fn alternative(escaped_value: &str) -> String {
    format!(r#"<rdf:Alt><rdf:li xml:lang="x-default">{escaped_value}</rdf:li></rdf:Alt>"#)
}

/// Macht aus `<rdf:li xml:lang="x-default"/>` ein
/// `<rdf:li xml:lang="x-default">Wert</rdf:li>` und behält die Attribute.
fn filled_list_item(empty_tag: &str, escaped_value: &str) -> String {
    let opening = empty_tag
        .trim_end()
        .trim_end_matches('>')
        .trim_end_matches('/')
        .trim_end();
    format!("{opening}>{escaped_value}</rdf:li>")
}

// ---------------------------------------------------------------------------
// Ersetzungen anwenden
// ---------------------------------------------------------------------------

/// Eine Ersetzung: Bereich im alten Text und neuer Inhalt.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    range: Range<usize>,
    replacement: String,
}

impl Edit {
    fn new(range: Range<usize>, replacement: String) -> Self {
        Self { range, replacement }
    }
}

/// Setzt den neuen Text zusammen. `None`, wenn sich Ersetzungen überlappen
/// oder ein Bereich ungültig ist – das wäre ein Programmierfehler.
fn apply(xml: &str, mut edits: Vec<Edit>) -> Option<String> {
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
fn position(reader: &Reader<&[u8]>) -> Result<usize, ParseProblem> {
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
fn attribute_value_span(tag: &str, key: &str) -> Option<Range<usize>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::xmp::metadata_from_bytes;

    fn preset_xml(attributes: &str, body: &str) -> String {
        format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" {attributes}>
{body}
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#
        )
    }

    fn alt(field: &str, value: &str) -> String {
        format!(
            r#"   <{field}>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">{value}</rdf:li>
    </rdf:Alt>
   </{field}>"#
        )
    }

    /// Ein Preset, wie Adobe es schreibt: Felder als Elemente, leerer
    /// Sortierschlüssel, eingebettetes Profil mit eigenem Namen.
    fn adobe_like() -> String {
        preset_xml(
            "crs:PresetType=\"Normal\"\n    crs:UUID=\"AAAA\"\n    crs:Exposure2012=\"+0.50\"",
            &format!(
                "{}\n{}\n{}\n{}",
                alt("crs:Name", "Warm"),
                r#"   <crs:SortName><rdf:Alt><rdf:li xml:lang="x-default"/></rdf:Alt></crs:SortName>"#,
                alt("crs:Group", "Sommer"),
                r#"   <crs:Look><rdf:Description crs:Name="Profil" crs:UUID="PROFIL"/></crs:Look>"#,
            ),
        )
    }

    fn metadata(bytes: &[u8]) -> PresetMetadata {
        metadata_from_bytes(bytes, Path::new("t.xmp")).unwrap()
    }

    fn rewrite(xml: &str, changes: &MetadataChanges) -> String {
        let bytes = rewrite_metadata(xml.as_bytes(), changes, Path::new("t.xmp")).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    fn name(value: &str) -> MetadataChanges {
        MetadataChanges {
            name: Some(value.to_owned()),
            ..MetadataChanges::default()
        }
    }

    #[test]
    fn aendert_namen_und_sonst_nichts() {
        let xml = adobe_like();

        let result = rewrite(&xml, &name("Kalt"));

        assert_eq!(
            result,
            xml.replacen(">Warm<", ">Kalt<", 1),
            "nur der Name darf sich ändern"
        );
        assert_eq!(metadata(result.as_bytes()).name, "Kalt");
    }

    #[test]
    fn sonderzeichen_ueberleben_schreiben_und_lesen() {
        let value = "C&A <1> \"Zitat\" 'x' Größe „Kühl“ 📷";

        let result = rewrite(&adobe_like(), &name(value));

        assert_eq!(metadata(result.as_bytes()).name, value);
        assert!(result.contains("C&amp;A &lt;1&gt;"));
    }

    #[test]
    fn fuellt_leeren_sortierschluessel() {
        let changes = MetadataChanges {
            sort_name: Some("0003".to_owned()),
            ..MetadataChanges::default()
        };

        let result = rewrite(&adobe_like(), &changes);

        assert!(result.contains(r#"<rdf:li xml:lang="x-default">0003</rdf:li>"#));
        assert_eq!(metadata(result.as_bytes()).sort_name, "0003");
    }

    #[test]
    fn aendert_alle_sprachvarianten() {
        let xml = preset_xml(
            r#"crs:PresetType="Normal""#,
            r#"<crs:Name><rdf:Alt>
                 <rdf:li xml:lang="en-US">Warm</rdf:li>
                 <rdf:li xml:lang="x-default">Warm</rdf:li>
               </rdf:Alt></crs:Name>"#,
        );

        let result = rewrite(&xml, &name("Kalt"));

        assert!(!result.contains("Warm"));
        assert_eq!(result.matches(">Kalt<").count(), 2);
    }

    #[test]
    fn aendert_attributform() {
        let xml = preset_xml(r#"crs:PresetType="Normal" crs:Group='Alt &amp; neu'"#, "");
        let changes = MetadataChanges {
            group: Some("Neu & besser".to_owned()),
            ..MetadataChanges::default()
        };

        let result = rewrite(&xml, &changes);

        assert!(result.contains("crs:Group='Neu &amp; besser'"));
        assert_eq!(metadata(result.as_bytes()).group, "Neu & besser");
    }

    #[test]
    fn fuegt_fehlendes_feld_ein() {
        let xml = preset_xml(r#"crs:PresetType="Normal""#, &alt("crs:Name", "Warm"));
        let changes = MetadataChanges {
            group: Some("Neue Gruppe".to_owned()),
            sort_name: Some("0001".to_owned()),
            ..MetadataChanges::default()
        };

        let result = rewrite(&xml, &changes);

        let read = metadata(result.as_bytes());
        assert_eq!(read.group, "Neue Gruppe");
        assert_eq!(read.sort_name, "0001");
        assert_eq!(read.name, "Warm");
    }

    #[test]
    fn fuegt_feld_auch_in_leeres_description_ein() {
        let xml = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:PresetType="Normal"/></rdf:RDF></x:xmpmeta>"#;

        let result = rewrite(xml, &name("Neu"));

        assert_eq!(metadata(result.as_bytes()).name, "Neu");
    }

    #[test]
    fn aendert_feld_auch_in_zweitem_obersten_description() {
        // Manche Programme verteilen die Felder auf mehrere Blöcke.
        let xml = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:PresetType="Normal"/>
<rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/">
<crs:Name><rdf:Alt><rdf:li xml:lang="x-default">Warm</rdf:li></rdf:Alt></crs:Name>
</rdf:Description>
</rdf:RDF></x:xmpmeta>"#;

        let result = rewrite(xml, &name("Kalt"));

        assert!(!result.contains("Warm"), "kein alter Wert bleibt zurück");
        assert_eq!(
            result.matches(">Kalt<").count(),
            1,
            "nicht zusätzlich eingefügt"
        );
        assert_eq!(metadata(result.as_bytes()).name, "Kalt");
    }

    #[test]
    fn leere_gruppe_entfernt_gruppe_aus_der_anzeige() {
        let changes = MetadataChanges {
            group: Some(String::new()),
            ..MetadataChanges::default()
        };

        let result = rewrite(&adobe_like(), &changes);

        assert_eq!(metadata(result.as_bytes()).group, "");
    }

    #[test]
    fn eingebettetes_profil_bleibt_unberuehrt() {
        let changes = MetadataChanges {
            name: Some("Neu".to_owned()),
            group: Some("G".to_owned()),
            sort_name: Some("1".to_owned()),
        };

        let result = rewrite(&adobe_like(), &changes);

        assert!(result.contains(r#"<rdf:Description crs:Name="Profil" crs:UUID="PROFIL"/>"#));
    }

    #[test]
    fn lehnt_steuerzeichen_ab() {
        let result = rewrite_metadata(
            adobe_like().as_bytes(),
            &name("Zeile\nZwei"),
            Path::new("t.xmp"),
        );

        assert!(matches!(result, Err(CoreError::InvalidName { .. })));
    }

    #[test]
    fn sidecar_wird_nicht_geaendert() {
        let xml = preset_xml(r#"crs:Exposure2012="+0.50""#, "");

        let result = rewrite_metadata(xml.as_bytes(), &name("x"), Path::new("t.xmp"));

        assert!(matches!(result, Err(CoreError::NotAPreset { .. })));
    }

    // --- Kennung ---------------------------------------------------------

    const NEW_UUID: &str = "0123456789ABCDEF0123456789ABCDEF";

    fn replaced(xml: &str) -> String {
        let bytes = replace_uuid(xml.as_bytes(), NEW_UUID, Path::new("t.xmp")).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn ersetzt_uuid_als_attribut_und_laesst_den_rest_unveraendert() {
        let xml = adobe_like();

        let result = replaced(&xml);

        assert_eq!(result, xml.replace("\"AAAA\"", &format!("\"{NEW_UUID}\"")));
        assert_eq!(metadata(result.as_bytes()).uuid.as_deref(), Some(NEW_UUID));
    }

    #[test]
    fn ersetzt_uuid_als_element() {
        let xml = preset_xml(r#"crs:PresetType="Normal""#, "<crs:UUID> ALT </crs:UUID>");

        let result = replaced(&xml);

        assert_eq!(
            result,
            xml.replace(
                "<crs:UUID> ALT </crs:UUID>",
                &format!("<crs:UUID>{NEW_UUID}</crs:UUID>")
            )
        );
    }

    #[test]
    fn findet_uuid_auch_mit_einfachen_anfuehrungszeichen_und_taeuschung() {
        // Der Wert eines anderen Attributs enthält Text, der wie das gesuchte
        // Attribut aussieht. Er darf nicht verändert werden.
        let xml = preset_xml(
            r#"crs:Note='crs:UUID="FALLE"' crs:PresetType="Normal" crs:UUID='ALT'"#,
            "",
        );

        let result = replaced(&xml);

        assert!(result.contains(r#"crs:Note='crs:UUID="FALLE"'"#));
        assert!(result.contains(&format!("crs:UUID='{NEW_UUID}'")));
    }

    #[test]
    fn datei_ohne_uuid_bleibt_unveraendert() {
        let xml = preset_xml(r#"crs:PresetType="Normal""#, "");

        assert_eq!(replaced(&xml), xml);
    }

    #[test]
    fn behaelt_byte_order_mark() {
        let with_bom = [b"\xEF\xBB\xBF".as_slice(), adobe_like().as_bytes()].concat();

        let result = replace_uuid(&with_bom, NEW_UUID, Path::new("t.xmp")).unwrap();
        let renamed = rewrite_metadata(&with_bom, &name("Neu"), Path::new("t.xmp")).unwrap();

        assert!(result.starts_with(b"\xEF\xBB\xBF"));
        assert!(renamed.starts_with(b"\xEF\xBB\xBF"));
        assert_eq!(metadata(&renamed).name, "Neu");
    }

    #[test]
    fn ueberlappende_ersetzungen_werden_erkannt() {
        let edits = vec![
            Edit::new(0..5, "a".to_owned()),
            Edit::new(3..8, "b".to_owned()),
        ];

        assert_eq!(apply("0123456789", edits), None);
    }
}
