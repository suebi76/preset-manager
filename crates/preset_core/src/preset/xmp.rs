//! Kennfelder aus einer Preset-Datei (`.xmp`) lesen.
//!
//! Eine Preset-Datei enthält hunderte Reglerwerte. Für die App zählen nur
//! wenige Felder: `crs:PresetType`, `crs:UUID`, `crs:Name`, `crs:Group` und
//! `crs:SortName`. Diese liest das Modul, alles andere wird übergangen.
//!
//! # Aufbau einer Datei (vereinfacht)
//!
//! ```text
//! <x:xmpmeta>
//!  <rdf:RDF>
//!   <rdf:Description crs:PresetType="Normal" crs:UUID="…">   ← oberste Ebene
//!    <crs:Name><rdf:Alt><rdf:li xml:lang="x-default">Mein Preset</rdf:li></rdf:Alt></crs:Name>
//!    <crs:Group><rdf:Alt><rdf:li xml:lang="x-default">Meine Gruppe</rdf:li></rdf:Alt></crs:Group>
//!    <crs:Look>
//!     <rdf:Description crs:Name="Profilname" crs:UUID="…">   ← eingebettetes Profil
//!    </crs:Look>
//!   </rdf:Description>
//!  </rdf:RDF>
//! </x:xmpmeta>
//! ```
//!
//! Zwei Dinge sind dabei wichtig:
//!
//! 1. **Nur die oberste Ebene zählt.** Viele Presets betten ein Profil ein
//!    (`crs:Look`), das wiederum eigene `crs:Name`-, `crs:UUID`- und
//!    `crs:Group`-Felder hat. Diese gehören zum Profil, nicht zum Preset.
//! 2. **Ein Feld kann auf zwei Arten notiert sein:** als Attribut
//!    (`crs:UUID="…"`) oder als Element (`<crs:Name>…</crs:Name>`). Name,
//!    Gruppe und Sortierschlüssel sind mehrsprachig (`rdf:Alt`); verwendet wird
//!    der Eintrag `x-default`, sonst der erste.
//!
//! Das Modul geht davon aus, dass Adobe die üblichen Präfixe `crs:` und `rdf:`
//! verwendet. Das ist in allen bekannten Dateien so.

use std::path::Path;

use quick_xml::escape::resolve_xml_entity;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use crate::error::CoreError;
use crate::preset::model::PresetType;

/// Die Kennfelder eines Presets, wie sie in der Datei stehen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetMetadata {
    /// Ob die Datei ein Preset oder ein Profil ist (`crs:PresetType`).
    pub preset_type: PresetType,
    /// Angezeigter Name (`crs:Name`). Kann leer sein.
    pub name: String,
    /// Angezeigte Gruppe (`crs:Group`). Leer, wenn keine Gruppe gesetzt ist.
    pub group: String,
    /// Eindeutige Kennung (`crs:UUID`), falls vorhanden.
    pub uuid: Option<String>,
    /// Sortierschlüssel (`crs:SortName`). Meist leer.
    pub sort_name: String,
}

/// Liest die Kennfelder aus einer Preset-Datei.
///
/// # Fehler
///
/// - [`CoreError::Io`], wenn die Datei nicht gelesen werden kann.
/// - [`CoreError::NotAPreset`], wenn die Datei gültiges XMP ist, aber kein
///   `crs:PresetType` hat – etwa das Sidecar eines Fotos.
/// - [`CoreError::MalformedXmp`], wenn die Datei kein gültiges XML ist.
pub fn read_metadata(path: &Path) -> Result<PresetMetadata, CoreError> {
    let bytes = std::fs::read(path).map_err(|source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    metadata_from_bytes(&bytes, path)
}

/// Wie [`read_metadata`], aber für einen Dateiinhalt, der schon im Speicher
/// liegt. `path` dient nur für Fehlermeldungen.
///
/// Der Import prüft damit jede Datei ein zweites Mal, unmittelbar bevor er
/// sie schreibt.
pub fn metadata_from_bytes(bytes: &[u8], path: &Path) -> Result<PresetMetadata, CoreError> {
    let xml = decode_utf8(bytes).map_err(|problem| problem.into_core_error(path))?;
    parse_metadata(xml).map_err(|problem| problem.into_core_error(path))
}

/// Warum eine Datei nicht als Preset gelesen werden konnte.
///
/// Intern, weil hier noch kein Dateipfad bekannt ist. [`read_metadata`]
/// macht daraus einen [`CoreError`].
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ParseProblem {
    NotAPreset,
    Malformed(String),
}

impl ParseProblem {
    pub(super) fn into_core_error(self, path: &Path) -> CoreError {
        match self {
            Self::NotAPreset => CoreError::NotAPreset {
                path: path.to_path_buf(),
            },
            Self::Malformed(reason) => CoreError::MalformedXmp {
                path: path.to_path_buf(),
                reason,
            },
        }
    }
}

/// Prüft, dass die Datei UTF-8 ist, und entfernt eine eventuelle
/// Byte-Order-Mark am Anfang.
pub(super) fn decode_utf8(bytes: &[u8]) -> Result<&str, ParseProblem> {
    let without_bom = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    std::str::from_utf8(without_bom)
        .map_err(|error| ParseProblem::Malformed(format!("keine gültige UTF-8-Datei ({error})")))
}

/// Liest die Kennfelder aus dem Inhalt einer XMP-Datei.
pub(super) fn parse_metadata(xml: &str) -> Result<PresetMetadata, ParseProblem> {
    let mut reader = Reader::from_str(xml);
    let mut parser = Parser::default();

    loop {
        let event = reader
            .read_event()
            .map_err(|error| ParseProblem::Malformed(error.to_string()))?;

        match event {
            Event::Start(element) => parser.on_start(&element)?,
            Event::Empty(element) => parser.on_empty(&element)?,
            Event::End(_) => parser.on_end(),
            Event::Text(text) => parser.on_text(&text.xml10_content()),
            Event::CData(data) => parser.on_text(&data.xml10_content()),
            Event::GeneralRef(reference) => parser.on_text(&resolve_reference(&reference)?),
            Event::Eof => break,
            Event::Comment(_) | Event::Decl(_) | Event::PI(_) | Event::DocType(_) => {}
        }
    }

    if !parser.open_elements.is_empty() {
        return Err(ParseProblem::Malformed(
            "Datei endet mitten in einem Element".to_owned(),
        ));
    }

    parser.collected.into_metadata()
}

/// Löst `&amp;`, `&lt;`, `&#228;` usw. in das gemeinte Zeichen auf.
pub(super) fn resolve_reference(reference: &BytesRef) -> Result<String, ParseProblem> {
    let character = reference
        .resolve_char_ref()
        .map_err(|error| ParseProblem::Malformed(error.to_string()))?;

    if let Some(character) = character {
        return Ok(character.to_string());
    }

    resolve_xml_entity(reference)
        .map(str::to_owned)
        .ok_or_else(|| ParseProblem::Malformed(format!("unbekannte Entität &{};", &**reference)))
}

/// `true`, wenn das innerste offene Element das oberste `rdf:Description` ist.
pub(super) fn is_top_level_description(open_elements: &[String]) -> bool {
    match open_elements {
        [.., grandparent, parent] => grandparent == "rdf:RDF" && parent == "rdf:Description",
        _ => false,
    }
}

/// Die Felder, die die App interessieren.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    PresetType,
    Uuid,
    Name,
    Group,
    SortName,
}

impl Field {
    fn from_qualified_name(name: &str) -> Option<Self> {
        match name {
            "crs:PresetType" => Some(Self::PresetType),
            "crs:UUID" => Some(Self::Uuid),
            "crs:Name" => Some(Self::Name),
            "crs:Group" => Some(Self::Group),
            "crs:SortName" => Some(Self::SortName),
            _ => None,
        }
    }
}

/// Ein Feld, das als Element geöffnet wurde und noch nicht geschlossen ist.
#[derive(Debug)]
struct OpenField {
    field: Field,
    /// Anzahl offener Elemente außerhalb des Feldes. Wird die Zahl beim
    /// Schließen wieder erreicht, ist das Feld zu Ende.
    depth: usize,
    /// Bisher gelesener Text.
    text: String,
    /// `Some(ist_standardsprache)`, solange ein `rdf:li` offen ist.
    open_list_item: Option<bool>,
}

/// Verarbeitet die XML-Ereignisse nacheinander und merkt sich, wo im
/// Dokument es gerade steht.
#[derive(Debug, Default)]
struct Parser {
    /// Namen aller gerade offenen Elemente, von außen nach innen.
    open_elements: Vec<String>,
    /// Das Feld, in dem der Leser gerade steht.
    current_field: Option<OpenField>,
    /// Die bisher gefundenen Werte.
    collected: CollectedValues,
}

impl Parser {
    fn on_start(&mut self, element: &BytesStart) -> Result<(), ParseProblem> {
        let name = self.inspect_element(element, false)?;
        self.open_elements.push(name);
        Ok(())
    }

    fn on_empty(&mut self, element: &BytesStart) -> Result<(), ParseProblem> {
        self.inspect_element(element, true).map(|_name| ())
    }

    fn on_text(&mut self, text: &str) {
        if let Some(open) = self.current_field.as_mut() {
            open.text.push_str(text);
        }
    }

    fn on_end(&mut self) {
        let Some(name) = self.open_elements.pop() else {
            return;
        };
        let Some(open) = self.current_field.as_mut() else {
            return;
        };

        if name == "rdf:li" {
            if let Some(is_default_language) = open.open_list_item.take() {
                let value = std::mem::take(&mut open.text);
                self.collected.store(open.field, value, is_default_language);
            }
        } else if self.open_elements.len() == open.depth {
            // Das Feld selbst wurde geschlossen. Steht Text direkt darin
            // (z. B. `<crs:UUID>…</crs:UUID>`), ist das der Wert.
            let value = open.text.trim().to_owned();
            let field = open.field;
            self.current_field = None;
            if !value.is_empty() {
                self.collected.store(field, value, true);
            }
        }
    }

    /// Entscheidet, ob ein gerade begonnenes Element für uns von Belang ist,
    /// und gibt seinen Namen zurück.
    fn inspect_element(
        &mut self,
        element: &BytesStart,
        is_empty: bool,
    ) -> Result<String, ParseProblem> {
        let name = element.name().as_ref().to_owned();
        self.inspect_named_element(element, &name, is_empty)?;
        Ok(name)
    }

    fn inspect_named_element(
        &mut self,
        element: &BytesStart,
        name: &str,
        is_empty: bool,
    ) -> Result<(), ParseProblem> {
        if let Some(open) = self.current_field.as_mut() {
            if name == "rdf:li" {
                let is_default_language = is_default_language(element)?;
                if is_empty {
                    self.collected
                        .store(open.field, String::new(), is_default_language);
                } else {
                    open.text.clear();
                    open.open_list_item = Some(is_default_language);
                }
            }
            return Ok(());
        }

        if name == "rdf:Description" && self.parent_is("rdf:RDF") {
            return self.read_description_attributes(element);
        }

        if self.is_inside_top_level_description() && !is_empty {
            if let Some(field) = Field::from_qualified_name(name) {
                self.current_field = Some(OpenField {
                    field,
                    depth: self.open_elements.len(),
                    text: String::new(),
                    open_list_item: None,
                });
            }
        }
        Ok(())
    }

    /// Liest Felder, die als Attribut am obersten `rdf:Description` stehen.
    fn read_description_attributes(&mut self, element: &BytesStart) -> Result<(), ParseProblem> {
        for attribute in element.attributes() {
            let attribute =
                attribute.map_err(|error| ParseProblem::Malformed(error.to_string()))?;
            if let Some(field) = Field::from_qualified_name(attribute.key.as_ref()) {
                self.collected
                    .store(field, attribute_value(&attribute)?, true);
            }
        }
        Ok(())
    }

    fn parent_is(&self, name: &str) -> bool {
        self.open_elements.last().is_some_and(|last| last == name)
    }

    /// `true`, wenn das zuletzt geöffnete Element das oberste
    /// `rdf:Description` ist – also direkt in `rdf:RDF` liegt.
    fn is_inside_top_level_description(&self) -> bool {
        is_top_level_description(&self.open_elements)
    }
}

/// `true`, wenn ein `rdf:li` die Standardsprache (`xml:lang="x-default"`) hat.
fn is_default_language(element: &BytesStart) -> Result<bool, ParseProblem> {
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| ParseProblem::Malformed(error.to_string()))?;
        if attribute.key.as_ref() == "xml:lang" {
            return Ok(attribute_value(&attribute)? == "x-default");
        }
    }
    Ok(false)
}

/// Der Wert eines Attributs mit aufgelösten Entitäten (`&amp;` → `&`).
pub(super) fn attribute_value(attribute: &Attribute) -> Result<String, ParseProblem> {
    attribute
        .normalized_value(XmlVersion::Implicit1_0)
        .map(|value| value.into_owned())
        .map_err(|error| ParseProblem::Malformed(error.to_string()))
}

/// Die gefundenen Werte, bevor daraus [`PresetMetadata`] wird.
#[derive(Debug, Default)]
struct CollectedValues {
    preset_type: Option<String>,
    uuid: Option<String>,
    name: LanguageAlternatives,
    group: LanguageAlternatives,
    sort_name: LanguageAlternatives,
}

impl CollectedValues {
    fn store(&mut self, field: Field, value: String, is_default_language: bool) {
        match field {
            Field::PresetType => self.preset_type = Some(value),
            Field::Uuid => self.uuid = Some(value),
            Field::Name => self.name.offer(value, is_default_language),
            Field::Group => self.group.offer(value, is_default_language),
            Field::SortName => self.sort_name.offer(value, is_default_language),
        }
    }

    fn into_metadata(self) -> Result<PresetMetadata, ParseProblem> {
        let preset_type = self.preset_type.ok_or(ParseProblem::NotAPreset)?;
        Ok(PresetMetadata {
            preset_type: PresetType::from_xmp_value(&preset_type),
            name: self.name.into_value(),
            group: self.group.into_value(),
            uuid: self.uuid.filter(|uuid| !uuid.is_empty()),
            sort_name: self.sort_name.into_value(),
        })
    }
}

/// Die Sprachvarianten eines mehrsprachigen Feldes. Die App braucht nur eine:
/// die Standardsprache, ersatzweise die erste nicht leere.
///
/// Leere Einträge zählen nicht. Sonst würde ein leeres `x-default` einen
/// gefüllten Eintrag in einer anderen Sprache verdecken.
#[derive(Debug, Default)]
struct LanguageAlternatives {
    default_language: Option<String>,
    first: Option<String>,
}

impl LanguageAlternatives {
    fn offer(&mut self, value: String, is_default_language: bool) {
        if value.is_empty() {
            return;
        }
        if is_default_language {
            self.default_language.get_or_insert(value);
        } else {
            self.first.get_or_insert(value);
        }
    }

    fn into_value(self) -> String {
        self.default_language.or(self.first).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Baut eine Preset-Datei um den übergebenen Inhalt des obersten
    /// `rdf:Description` herum.
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
            r#"<{field}><rdf:Alt><rdf:li xml:lang="x-default">{value}</rdf:li></rdf:Alt></{field}>"#
        )
    }

    #[test]
    fn liest_alle_felder() {
        let xml = preset_xml(
            r#"crs:PresetType="Normal" crs:UUID="ABC123""#,
            &[
                alt("crs:Name", "Warm"),
                alt("crs:Group", "Sommer"),
                alt("crs:SortName", "02"),
            ]
            .concat(),
        );

        let metadata = parse_metadata(&xml).unwrap();

        assert_eq!(metadata.preset_type, PresetType::Preset);
        assert_eq!(metadata.name, "Warm");
        assert_eq!(metadata.group, "Sommer");
        assert_eq!(metadata.sort_name, "02");
        assert_eq!(metadata.uuid.as_deref(), Some("ABC123"));
    }

    #[test]
    fn dekodiert_sonderzeichen() {
        let xml = preset_xml(
            r#"crs:PresetType="Normal""#,
            &[
                alt("crs:Name", "C&amp;A &lt;1&gt; &#228;&#x00F6;ü 📷"),
                alt("crs:Group", "Tom &quot;&amp;&quot; Jerry"),
            ]
            .concat(),
        );

        let metadata = parse_metadata(&xml).unwrap();

        assert_eq!(metadata.name, "C&A <1> äöü 📷");
        assert_eq!(metadata.group, "Tom \"&\" Jerry");
    }

    #[test]
    fn ignoriert_felder_eines_eingebetteten_profils() {
        let body = format!(
            r#"{}
<crs:Look>
 <rdf:Description crs:Name="Profil" crs:UUID="PROFIL-UUID">
  {}
 </rdf:Description>
</crs:Look>"#,
            alt("crs:Name", "Preset"),
            alt("crs:Group", "Profilgruppe"),
        );
        let xml = preset_xml(r#"crs:PresetType="Normal" crs:UUID="PRESET-UUID""#, &body);

        let metadata = parse_metadata(&xml).unwrap();

        assert_eq!(metadata.name, "Preset");
        assert_eq!(metadata.group, "");
        assert_eq!(metadata.uuid.as_deref(), Some("PRESET-UUID"));
    }

    #[test]
    fn liest_felder_auch_in_attributform_und_elementform() {
        let xml = preset_xml(
            r#"crs:PresetType="Look" crs:Name="Als Attribut""#,
            "<crs:UUID> ELEMENT-UUID </crs:UUID>",
        );

        let metadata = parse_metadata(&xml).unwrap();

        assert_eq!(metadata.preset_type, PresetType::Profile);
        assert_eq!(metadata.name, "Als Attribut");
        assert_eq!(metadata.uuid.as_deref(), Some("ELEMENT-UUID"));
    }

    #[test]
    fn leere_felder_ergeben_leere_werte() {
        let xml = preset_xml(
            r#"crs:PresetType="Normal" crs:UUID="""#,
            r#"<crs:Group><rdf:Alt><rdf:li xml:lang="x-default"/></rdf:Alt></crs:Group>"#,
        );

        let metadata = parse_metadata(&xml).unwrap();

        assert_eq!(metadata.name, "");
        assert_eq!(metadata.group, "");
        assert_eq!(metadata.uuid, None);
    }

    #[test]
    fn bevorzugt_standardsprache_sonst_erste_sprache() {
        let mit_standard = preset_xml(
            r#"crs:PresetType="Normal""#,
            r#"<crs:Name><rdf:Alt>
                 <rdf:li xml:lang="en-US">English</rdf:li>
                 <rdf:li xml:lang="x-default">Standard</rdf:li>
               </rdf:Alt></crs:Name>"#,
        );
        let ohne_standard = preset_xml(
            r#"crs:PresetType="Normal""#,
            r#"<crs:Name><rdf:Alt>
                 <rdf:li xml:lang="de-DE">Deutsch</rdf:li>
                 <rdf:li xml:lang="en-US">English</rdf:li>
               </rdf:Alt></crs:Name>"#,
        );

        assert_eq!(parse_metadata(&mit_standard).unwrap().name, "Standard");
        assert_eq!(parse_metadata(&ohne_standard).unwrap().name, "Deutsch");
    }

    #[test]
    fn leere_standardsprache_verdeckt_keine_andere_sprache() {
        let xml = preset_xml(
            r#"crs:PresetType="Normal""#,
            r#"<crs:Name><rdf:Alt>
                 <rdf:li xml:lang="x-default"/>
                 <rdf:li xml:lang="de-DE">Warm</rdf:li>
               </rdf:Alt></crs:Name>"#,
        );

        assert_eq!(parse_metadata(&xml).unwrap().name, "Warm");
    }

    #[test]
    fn foto_sidecar_ist_kein_preset() {
        let xml = preset_xml(r#"crs:Exposure2012="+0.50""#, "");

        assert_eq!(parse_metadata(&xml), Err(ParseProblem::NotAPreset));
    }

    #[test]
    fn abgeschnittene_datei_ist_beschaedigt() {
        let xml = preset_xml(r#"crs:PresetType="Normal""#, &alt("crs:Name", "Warm"));
        let truncated = &xml[..xml.len() / 2];

        assert!(matches!(
            parse_metadata(truncated),
            Err(ParseProblem::Malformed(_))
        ));
    }

    #[test]
    fn falsch_verschachtelte_datei_ist_beschaedigt() {
        let xml = "<x:xmpmeta><rdf:RDF></x:xmpmeta></rdf:RDF>";

        assert!(matches!(
            parse_metadata(xml),
            Err(ParseProblem::Malformed(_))
        ));
    }

    #[test]
    fn entfernt_byte_order_mark() {
        assert_eq!(decode_utf8(b"\xEF\xBB\xBF<a/>"), Ok("<a/>"));
    }

    #[test]
    fn lehnt_ungueltiges_utf8_ab() {
        assert!(matches!(
            decode_utf8(b"<a>\xFF</a>"),
            Err(ParseProblem::Malformed(_))
        ));
    }
}
