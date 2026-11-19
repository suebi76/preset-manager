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
//!
//! | Datei | Inhalt |
//! |---|---|
//! | `locate` | Stellen der Felder finden |
//! | `edits` | Ersetzungen anwenden |

mod edits;
mod locate;

use locate::locate;
#[cfg(test)]
mod tests;

use std::path::Path;

use crate::error::{CoreError, NameProblem};
use crate::preset::xmp::{decode_utf8, parse_metadata, ParseProblem, PresetMetadata};
use edits::apply;
use edits::Edit;
use locate::Field;

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
            problem: NameProblem::ControlCharacters,
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
