//! Alle Camera-Raw-Werte einer Preset-Datei lesen.
//!
//! [`super::xmp`] liest nur die wenigen Kennfelder, die zum Verwalten nötig
//! sind. Dieses Modul liest, was das Preset **einstellt**: die Reglerwerte am
//! obersten `rdf:Description` (`crs:Exposure2012="+0.50"` …), die
//! Gradationskurven (`<crs:ToneCurvePV2012>` mit Punkten), das eingebettete
//! Profil (`<crs:Look>`) und die lokalen Anpassungen (Masken).
//!
//! Gedeutet werden die Werte in [`super::adjustments`].
//!
//! ```text
//! <rdf:Description crs:PresetType="Normal" crs:Exposure2012="+0.50" …>
//!  <crs:ToneCurvePV2012><rdf:Seq><rdf:li>0, 18</rdf:li> …</rdf:Seq></crs:ToneCurvePV2012>
//!  <crs:Look><rdf:Description crs:Name="Profil" crs:Amount="0.8"/></crs:Look>
//!  <crs:MaskGroupBasedCorrections><rdf:Seq><rdf:li>
//!   <rdf:Description crs:What="Correction" crs:CorrectionName="Himmel" crs:LocalExposure2012="-0.3">
//!    <crs:CorrectionMasks><rdf:Seq><rdf:li crs:What="Mask/Gradient" …/></rdf:Seq></crs:CorrectionMasks>
//!   </rdf:Description>
//!  </rdf:li></rdf:Seq></crs:MaskGroupBasedCorrections>
//! </rdf:Description>
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use super::xmp::{
    attribute_value, decode_utf8, is_top_level_description, resolve_reference, ParseProblem,
};
use crate::error::CoreError;

/// Ein Punkt einer Gradationskurve: Eingang und Ausgang, je 0–255.
pub type CurvePoint = (f32, f32);

/// Was eine Preset-Datei einstellt – roh, wie es in der Datei steht.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PresetSettings {
    /// Alle `crs:`-Attribute am obersten `rdf:Description`, ohne Präfix:
    /// `"Exposure2012" → "+0.50"`.
    values: BTreeMap<String, String>,
    /// Die Gradationskurven (`ToneCurvePV2012`, `…Red`, `…Green`, `…Blue`).
    curves: BTreeMap<String, Vec<CurvePoint>>,
    /// Das eingebettete Profil (`crs:Look`), falls vorhanden.
    pub look: Option<Look>,
    /// Lokale Anpassungen (Masken), in der Reihenfolge der Datei.
    pub local_adjustments: Vec<LocalAdjustment>,
    /// Die Datei enthält eine Farbtabelle (`crs:RGBTable`) – typisch für
    /// Profile. Ihr Look lässt sich nicht als Regler darstellen.
    pub uses_color_table: bool,
    /// Die Farbtabellen selbst, wie sie in der Datei stehen (`crs:Table_…`,
    /// kodiert) – erst `crs:LookTable`, dann `crs:RGBTable`, siehe
    /// [`super::color_table`]. Leer bei Presets, die nur auf ein Profil
    /// verweisen.
    pub color_tables: Vec<String>,
}

/// Ein Profil, auf dem das Preset aufbaut.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Look {
    /// Der Name des Profils.
    pub name: String,
    /// Die Stärke, 0–2 (1 = 100 %), falls angegeben.
    pub amount: Option<f32>,
    /// Die Kennung des Profils – darüber findet sich die Profildatei, wenn
    /// das Preset die Farbtabelle nicht selbst enthält.
    pub uuid: Option<String>,
}

/// Eine lokale Anpassung: eine oder mehrere Masken mit Reglerwerten.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocalAdjustment {
    /// Der Name, z. B. „Maske 1“. Kann leer sein.
    pub name: String,
    /// `false`, wenn die Anpassung ausgeschaltet ist.
    pub active: bool,
    /// Welche Masken die Anpassung bilden.
    pub masks: Vec<MaskKind>,
    /// Die geänderten lokalen Regler ohne Präfix `Local`, z. B.
    /// `("Exposure2012", -0.3)`. Werte, die nichts bewirken, fehlen.
    pub changes: Vec<(String, f32)>,
}

impl LocalAdjustment {
    /// `true`, wenn die Anpassung eingeschaltet ist und etwas verändert.
    pub fn has_effect(&self) -> bool {
        self.active && !self.changes.is_empty()
    }
}

/// Die Art einer Maske.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MaskKind {
    /// Linearer Verlauf.
    Linear,
    /// Radialverlauf.
    Radial,
    /// Pinsel.
    Brush,
    /// Von Camera Raw erkannt (Motiv, Himmel, Person …).
    Ai,
    /// Farb-, Luminanz- oder Tiefenbereich.
    Range,
    /// Eine Art, die die App nicht kennt.
    Other,
}

impl MaskKind {
    fn from_what(what: &str) -> Option<Self> {
        let kind = what.strip_prefix("Mask/")?;
        Some(match kind {
            "Gradient" => Self::Linear,
            "CircularGradient" => Self::Radial,
            "Paint" | "Brush" => Self::Brush,
            "Image" => Self::Ai,
            "RangeMask" | "Range" => Self::Range,
            _ => Self::Other,
        })
    }
}

/// Lokale Regler, die nichts bewirken, wenn sie auf diesem Wert stehen –
/// alle anderen wirken bei 0 nicht.
const LOCAL_NEUTRAL: &[(&str, f32)] = &[("CurveRefineSaturation", 100.0)];
/// Lokale Werte, die allein nichts verändern (der Farbton wirkt erst mit
/// Sättigung).
const LOCAL_IGNORED: &[&str] = &["ToningHue"];

impl PresetSettings {
    /// Der Rohwert eines Reglers, z. B. `text("WhiteBalance")` → `"As Shot"`.
    pub fn text(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Ein Zahlenwert, z. B. `number("Exposure2012")` → `0.5`.
    pub fn number(&self, key: &str) -> Option<f32> {
        self.text(key)?.trim().parse().ok()
    }

    /// Ein Schalter (`"True"`/`"False"`, Groß-/Kleinschreibung egal).
    pub fn flag(&self, key: &str) -> Option<bool> {
        match self.text(key)?.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        }
    }

    /// Die Punkte einer Gradationskurve, z. B. `curve("ToneCurvePV2012")`.
    pub fn curve(&self, key: &str) -> Option<&[CurvePoint]> {
        self.curves.get(key).map(Vec::as_slice)
    }

    /// `true`, wenn die Datei gar keine Werte enthält.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.curves.is_empty() && self.look.is_none()
    }
}

/// Liest alle Einstellungen aus einer Preset-Datei.
///
/// # Fehler
///
/// - [`CoreError::Io`], wenn die Datei nicht gelesen werden kann.
/// - [`CoreError::MalformedXmp`], wenn die Datei kein gültiges XML ist.
pub fn read_settings(path: &Path) -> Result<PresetSettings, CoreError> {
    let bytes = std::fs::read(path).map_err(|source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let xml = decode_utf8(&bytes).map_err(|problem| problem.into_core_error(path))?;
    parse_settings(xml).map_err(|problem| problem.into_core_error(path))
}

/// Liest die Einstellungen aus dem Inhalt einer XMP-Datei.
pub(super) fn parse_settings(xml: &str) -> Result<PresetSettings, ParseProblem> {
    let mut reader = Reader::from_str(xml);
    let mut parser = Parser::default();
    loop {
        let event = reader
            .read_event()
            .map_err(|error| ParseProblem::Malformed(error.to_string()))?;
        match event {
            Event::Start(element) => {
                let name = parser.on_element(&element)?;
                parser.open.push(name);
            }
            Event::Empty(element) => {
                parser.on_element(&element)?;
            }
            Event::End(_) => parser.on_end(),
            Event::Text(text) => parser.on_text(&text.xml10_content()),
            Event::GeneralRef(reference) => parser.on_text(&resolve_reference(&reference)?),
            Event::Eof => break,
            _ => {}
        }
    }
    if !parser.open.is_empty() {
        return Err(ParseProblem::Malformed(
            "Datei endet mitten in einem Element".to_owned(),
        ));
    }
    let mut settings = parser.settings;
    settings.color_tables = [parser.look_table, parser.rgb_table]
        .into_iter()
        .flatten()
        .filter_map(|key| parser.tables.get(&key).cloned())
        .collect();
    Ok(settings)
}

/// Für Tests anderer Module: liest Einstellungen aus einem XMP-Text.
#[cfg(test)]
pub(crate) fn parse_settings_for_tests(xml: &str) -> PresetSettings {
    parse_settings(xml).unwrap_or_default()
}

/// Eine Kurve, deren Punkte gerade gelesen werden.
#[derive(Debug)]
struct OpenCurve {
    key: String,
    /// Anzahl offener Elemente außerhalb der Kurve.
    depth: usize,
    points: Vec<CurvePoint>,
    /// Text des offenen `rdf:li`.
    item: Option<String>,
}

#[derive(Debug, Default)]
struct Parser {
    open: Vec<String>,
    settings: PresetSettings,
    curve: Option<OpenCurve>,
    /// Welche Tabellen `crs:LookTable` und `crs:RGBTable` meinen.
    look_table: Option<String>,
    rgb_table: Option<String>,
    /// Alle Tabellen der Datei (`crs:Table_<Kennung>`).
    tables: BTreeMap<String, String>,
}

impl Parser {
    fn on_element(&mut self, element: &BytesStart) -> Result<String, ParseProblem> {
        let name = element.name().as_ref().to_owned();
        let attributes = crs_attributes(element)?;

        for (key, value) in &attributes {
            if key == "RGBTable" {
                self.settings.uses_color_table = true;
                self.rgb_table.get_or_insert_with(|| value.clone());
            } else if key == "LookTable" {
                self.settings.uses_color_table = true;
                self.look_table.get_or_insert_with(|| value.clone());
            } else if let Some(id) = key.strip_prefix("Table_") {
                self.settings.uses_color_table = true;
                self.tables.insert(id.to_owned(), value.clone());
            }
        }
        // Die Tabellen sind groß und keine Reglerwerte.
        let attributes: Vec<(String, String)> = attributes
            .into_iter()
            .filter(|(key, _)| !key.starts_with("Table_"))
            .collect();

        if let Some(curve) = self.curve.as_mut() {
            if name == "rdf:li" {
                curve.item = Some(String::new());
            }
            return Ok(name);
        }

        let in_look = self.open.iter().any(|open| open == "crs:Look");
        if name == "rdf:Description" && self.open.last().is_some_and(|last| last == "rdf:RDF") {
            self.settings.values.extend(attributes);
        } else if name == "rdf:Description" && self.open.last().is_some_and(|l| l == "crs:Look") {
            let look = self.settings.look.get_or_insert_with(Look::default);
            for (key, value) in attributes {
                match key.as_str() {
                    "Name" => look.name = value,
                    "Amount" => look.amount = value.trim().parse().ok(),
                    "UUID" => look.uuid = Some(value).filter(|uuid| !uuid.is_empty()),
                    _ => {}
                }
            }
        } else if in_look {
            // Felder des Profils gehören nicht zum Preset.
        } else if attributes
            .iter()
            .any(|(key, value)| key == "What" && value == "Correction")
        {
            self.settings
                .local_adjustments
                .push(local_adjustment(&attributes));
        } else if let Some(kind) = attributes
            .iter()
            .find(|(key, _)| key == "What")
            .and_then(|(_, what)| MaskKind::from_what(what))
        {
            if let Some(adjustment) = self.settings.local_adjustments.last_mut() {
                if !adjustment.masks.contains(&kind) {
                    adjustment.masks.push(kind);
                }
            }
        } else if is_top_level_description(&self.open) && name.starts_with("crs:ToneCurve") {
            self.curve = Some(OpenCurve {
                key: name.trim_start_matches("crs:").to_owned(),
                depth: self.open.len(),
                points: Vec::new(),
                item: None,
            });
        } else if is_top_level_description(&self.open) && name == "crs:Look" {
            self.settings.look.get_or_insert_with(Look::default);
        }
        Ok(name)
    }

    fn on_text(&mut self, text: &str) {
        if let Some(item) = self.curve.as_mut().and_then(|curve| curve.item.as_mut()) {
            item.push_str(text);
        }
    }

    fn on_end(&mut self) {
        let Some(name) = self.open.pop() else {
            return;
        };
        let Some(curve) = self.curve.as_mut() else {
            return;
        };
        if name == "rdf:li" {
            if let Some(point) = curve.item.take().as_deref().and_then(parse_point) {
                curve.points.push(point);
            }
        } else if self.open.len() == curve.depth {
            if let Some(curve) = self.curve.take() {
                if !curve.points.is_empty() {
                    self.settings.curves.insert(curve.key, curve.points);
                }
            }
        }
    }
}

/// Alle `crs:`-Attribute eines Elements, ohne Präfix.
fn crs_attributes(element: &BytesStart) -> Result<Vec<(String, String)>, ParseProblem> {
    let mut found = Vec::new();
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| ParseProblem::Malformed(error.to_string()))?;
        let key = attribute.key.as_ref().to_owned();
        if let Some(key) = key.strip_prefix("crs:") {
            found.push((key.to_owned(), attribute_value(&attribute)?));
        }
    }
    Ok(found)
}

fn local_adjustment(attributes: &[(String, String)]) -> LocalAdjustment {
    let mut adjustment = LocalAdjustment {
        active: true,
        ..LocalAdjustment::default()
    };
    for (key, value) in attributes {
        match key.as_str() {
            "CorrectionName" => adjustment.name = value.clone(),
            "CorrectionActive" => adjustment.active = !value.eq_ignore_ascii_case("false"),
            _ => {
                let Some(local) = key.strip_prefix("Local") else {
                    continue;
                };
                let Ok(number) = value.trim().parse::<f32>() else {
                    continue;
                };
                let neutral = LOCAL_NEUTRAL
                    .iter()
                    .find(|(name, _)| *name == local)
                    .map_or(0.0, |(_, neutral)| *neutral);
                if (number - neutral).abs() > f32::EPSILON && !LOCAL_IGNORED.contains(&local) {
                    adjustment.changes.push((local.to_owned(), number));
                }
            }
        }
    }
    adjustment
}

/// `"64, 70"` → `(64.0, 70.0)`.
fn parse_point(text: &str) -> Option<CurvePoint> {
    let (x, y) = text.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
   crs:PresetType="Normal" crs:Exposure2012="+0.50" crs:Contrast2012="-12"
   crs:WhiteBalance="As Shot" crs:ConvertToGrayscale="False">
   <crs:Name><rdf:Alt><rdf:li xml:lang="x-default">Warm &amp; matt</rdf:li></rdf:Alt></crs:Name>
   <crs:ToneCurvePV2012>
    <rdf:Seq><rdf:li>0, 18</rdf:li><rdf:li>128, 132</rdf:li><rdf:li>255, 240</rdf:li></rdf:Seq>
   </crs:ToneCurvePV2012>
   <crs:Look>
    <rdf:Description crs:Name="Mein Profil" crs:Amount="0.8" crs:Exposure2012="+3">
     <crs:Group><rdf:Alt><rdf:li xml:lang="x-default">Profile</rdf:li></rdf:Alt></crs:Group>
     <crs:Parameters><rdf:Description crs:RGBTable="ABC" crs:Table_ABC="..."/></crs:Parameters>
    </rdf:Description>
   </crs:Look>
   <crs:MaskGroupBasedCorrections><rdf:Seq>
    <rdf:li><rdf:Description crs:What="Correction" crs:CorrectionName="Himmel"
      crs:CorrectionActive="true" crs:LocalExposure2012="-0.3" crs:LocalContrast2012="0"
      crs:LocalCurveRefineSaturation="100" crs:LocalToningHue="40">
     <crs:CorrectionMasks><rdf:Seq>
      <rdf:li crs:What="Mask/Gradient" crs:MaskName="Verlauf"/>
      <rdf:li crs:What="Mask/Image" crs:MaskSubType="2"/>
     </rdf:Seq></crs:CorrectionMasks>
    </rdf:Description></rdf:li>
    <rdf:li><rdf:Description crs:What="Correction" crs:CorrectionName="Leer"
      crs:LocalExposure2012="0">
     <crs:CorrectionMasks><rdf:Seq><rdf:li crs:What="Mask/CircularGradient"/></rdf:Seq></crs:CorrectionMasks>
    </rdf:Description></rdf:li>
   </rdf:Seq></crs:MaskGroupBasedCorrections>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#;

    #[test]
    fn liest_reglerwerte() {
        let settings = parse_settings(SAMPLE).unwrap();

        assert_eq!(settings.number("Exposure2012"), Some(0.5));
        assert_eq!(settings.number("Contrast2012"), Some(-12.0));
        assert_eq!(settings.text("WhiteBalance"), Some("As Shot"));
        assert_eq!(settings.flag("ConvertToGrayscale"), Some(false));
        assert_eq!(settings.number("Fehlt"), None);
    }

    #[test]
    fn liest_kurven() {
        let settings = parse_settings(SAMPLE).unwrap();

        assert_eq!(
            settings.curve("ToneCurvePV2012"),
            Some(&[(0.0, 18.0), (128.0, 132.0), (255.0, 240.0)][..])
        );
        assert_eq!(settings.curve("ToneCurvePV2012Red"), None);
    }

    #[test]
    fn werte_des_profils_gehoeren_nicht_zum_preset() {
        let settings = parse_settings(SAMPLE).unwrap();

        assert_eq!(
            settings.look,
            Some(Look {
                name: "Mein Profil".to_owned(),
                amount: Some(0.8),
                uuid: None,
            })
        );
        assert_eq!(settings.number("Exposure2012"), Some(0.5));
        assert!(settings.uses_color_table);
        assert_eq!(settings.color_tables, vec!["...".to_owned()]);
    }

    #[test]
    fn liest_lokale_anpassungen() {
        let settings = parse_settings(SAMPLE).unwrap();

        assert_eq!(settings.local_adjustments.len(), 2);
        let sky = &settings.local_adjustments[0];
        assert_eq!(sky.name, "Himmel");
        assert_eq!(sky.masks, vec![MaskKind::Linear, MaskKind::Ai]);
        assert_eq!(sky.changes, vec![("Exposure2012".to_owned(), -0.3)]);
        assert!(sky.has_effect());
        assert!(!settings.local_adjustments[1].has_effect());
    }

    #[test]
    fn leere_datei_hat_keine_werte() {
        let xml = r#"<x:xmpmeta><rdf:RDF><rdf:Description crs:PresetType="Normal"/></rdf:RDF></x:xmpmeta>"#;
        let settings = parse_settings(xml).unwrap();

        assert_eq!(settings.text("PresetType"), Some("Normal"));
        assert!(settings.local_adjustments.is_empty());
        assert!(!settings.uses_color_table);
    }

    #[test]
    fn beschaedigte_datei_ist_ein_fehler() {
        assert!(parse_settings(&SAMPLE[..SAMPLE.len() / 2]).is_err());
    }
}
