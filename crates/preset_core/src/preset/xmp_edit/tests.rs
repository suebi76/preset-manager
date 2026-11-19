//! Tests für das Ändern von Name, Gruppe, Reihenfolge und UUID.

use crate::preset::xmp::metadata_from_bytes;

use super::*;

pub(super) fn preset_xml(attributes: &str, body: &str) -> String {
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

pub(super) fn alt(field: &str, value: &str) -> String {
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
pub(super) fn adobe_like() -> String {
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

pub(super) fn metadata(bytes: &[u8]) -> PresetMetadata {
    metadata_from_bytes(bytes, Path::new("t.xmp")).unwrap()
}

pub(super) fn rewrite(xml: &str, changes: &MetadataChanges) -> String {
    let bytes = rewrite_metadata(xml.as_bytes(), changes, Path::new("t.xmp")).unwrap();
    String::from_utf8(bytes).unwrap()
}

pub(super) fn name(value: &str) -> MetadataChanges {
    MetadataChanges {
        name: Some(value.to_owned()),
        ..MetadataChanges::default()
    }
}

#[test]
pub(super) fn aendert_namen_und_sonst_nichts() {
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
pub(super) fn sonderzeichen_ueberleben_schreiben_und_lesen() {
    let value = "C&A <1> \"Zitat\" 'x' Größe „Kühl“ 📷";

    let result = rewrite(&adobe_like(), &name(value));

    assert_eq!(metadata(result.as_bytes()).name, value);
    assert!(result.contains("C&amp;A &lt;1&gt;"));
}

#[test]
pub(super) fn fuellt_leeren_sortierschluessel() {
    let changes = MetadataChanges {
        sort_name: Some("0003".to_owned()),
        ..MetadataChanges::default()
    };

    let result = rewrite(&adobe_like(), &changes);

    assert!(result.contains(r#"<rdf:li xml:lang="x-default">0003</rdf:li>"#));
    assert_eq!(metadata(result.as_bytes()).sort_name, "0003");
}

#[test]
pub(super) fn aendert_alle_sprachvarianten() {
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
pub(super) fn aendert_attributform() {
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
pub(super) fn fuegt_fehlendes_feld_ein() {
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
pub(super) fn fuegt_feld_auch_in_leeres_description_ein() {
    let xml = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:PresetType="Normal"/></rdf:RDF></x:xmpmeta>"#;

    let result = rewrite(xml, &name("Neu"));

    assert_eq!(metadata(result.as_bytes()).name, "Neu");
}

#[test]
pub(super) fn aendert_feld_auch_in_zweitem_obersten_description() {
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
pub(super) fn leere_gruppe_entfernt_gruppe_aus_der_anzeige() {
    let changes = MetadataChanges {
        group: Some(String::new()),
        ..MetadataChanges::default()
    };

    let result = rewrite(&adobe_like(), &changes);

    assert_eq!(metadata(result.as_bytes()).group, "");
}

#[test]
pub(super) fn eingebettetes_profil_bleibt_unberuehrt() {
    let changes = MetadataChanges {
        name: Some("Neu".to_owned()),
        group: Some("G".to_owned()),
        sort_name: Some("1".to_owned()),
    };

    let result = rewrite(&adobe_like(), &changes);

    assert!(result.contains(r#"<rdf:Description crs:Name="Profil" crs:UUID="PROFIL"/>"#));
}

#[test]
pub(super) fn lehnt_steuerzeichen_ab() {
    let result = rewrite_metadata(
        adobe_like().as_bytes(),
        &name("Zeile\nZwei"),
        Path::new("t.xmp"),
    );

    assert!(matches!(result, Err(CoreError::InvalidName { .. })));
}

#[test]
pub(super) fn sidecar_wird_nicht_geaendert() {
    let xml = preset_xml(r#"crs:Exposure2012="+0.50""#, "");

    let result = rewrite_metadata(xml.as_bytes(), &name("x"), Path::new("t.xmp"));

    assert!(matches!(result, Err(CoreError::NotAPreset { .. })));
}

// --- Kennung ---------------------------------------------------------

pub(super) const NEW_UUID: &str = "0123456789ABCDEF0123456789ABCDEF";

pub(super) fn replaced(xml: &str) -> String {
    let bytes = replace_uuid(xml.as_bytes(), NEW_UUID, Path::new("t.xmp")).unwrap();
    String::from_utf8(bytes).unwrap()
}

#[test]
pub(super) fn ersetzt_uuid_als_attribut_und_laesst_den_rest_unveraendert() {
    let xml = adobe_like();

    let result = replaced(&xml);

    assert_eq!(result, xml.replace("\"AAAA\"", &format!("\"{NEW_UUID}\"")));
    assert_eq!(metadata(result.as_bytes()).uuid.as_deref(), Some(NEW_UUID));
}

#[test]
pub(super) fn ersetzt_uuid_als_element() {
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
pub(super) fn findet_uuid_auch_mit_einfachen_anfuehrungszeichen_und_taeuschung() {
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
pub(super) fn datei_ohne_uuid_bleibt_unveraendert() {
    let xml = preset_xml(r#"crs:PresetType="Normal""#, "");

    assert_eq!(replaced(&xml), xml);
}

#[test]
pub(super) fn behaelt_byte_order_mark() {
    let with_bom = [b"\xEF\xBB\xBF".as_slice(), adobe_like().as_bytes()].concat();

    let result = replace_uuid(&with_bom, NEW_UUID, Path::new("t.xmp")).unwrap();
    let renamed = rewrite_metadata(&with_bom, &name("Neu"), Path::new("t.xmp")).unwrap();

    assert!(result.starts_with(b"\xEF\xBB\xBF"));
    assert!(renamed.starts_with(b"\xEF\xBB\xBF"));
    assert_eq!(metadata(&renamed).name, "Neu");
}

#[test]
pub(super) fn ueberlappende_ersetzungen_werden_erkannt() {
    let edits = vec![
        Edit::new(0..5, "a".to_owned()),
        Edit::new(3..8, "b".to_owned()),
    ];

    assert_eq!(apply("0123456789", edits), None);
}
