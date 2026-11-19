//! Wo steht was? Findet in der Datei die Stellen von Name, Gruppe,
//! Sortierschlüssel und UUID – als Attribut, Element oder Sprachliste.

use std::ops::Range;

use quick_xml::escape::escape;
use quick_xml::events::Event;
use quick_xml::Reader;

use crate::preset::xmp::{is_top_level_description, ParseProblem};

use super::edits::{attribute_value_span, position, Edit};

/// Die Felder, die geändert werden können.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Field {
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
pub(super) struct Document {
    descriptions: Vec<Layout>,
}

impl Document {
    /// Die Ersetzungen, die `field` auf `value` setzen.
    pub(super) fn edits_for(&self, field: Field, value: &str, xml: &str) -> Vec<Edit> {
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
pub(super) fn locate(xml: &str) -> Result<Document, ParseProblem> {
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
