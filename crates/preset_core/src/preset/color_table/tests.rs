//! Tests für Dekodieren und Anwenden der Farbtabellen.

use std::io::Write;

use super::*;

#[test]
pub(super) fn behauptete_riesentabelle_ohne_daten_wird_abgelehnt() {
    // Farbton-Tabelle mit 256 × 256 × 256 Punkten – aber ohne Daten.
    let mut raw = Vec::new();
    for value in [0_u32, 2, 256, 256, 256] {
        raw.extend_from_slice(&value.to_le_bytes());
    }
    assert!(ColorTable::from_bytes(&raw).is_none());

    // RGB-Tabelle mit 128 Stützstellen je Achse – ebenfalls leer.
    let mut raw = Vec::new();
    for value in [1_u32, 1, 3, 128] {
        raw.extend_from_slice(&value.to_le_bytes());
    }
    assert!(ColorTable::from_bytes(&raw).is_none());
}

/// Baut den Text einer Tabelle wie Adobe: `map` bestimmt jeden Punkt.
pub(crate) fn encode_table(divisions: usize, map: impl Fn([f32; 3]) -> [f32; 3]) -> String {
    let mut raw = Vec::new();
    for value in [1u32, 1, 3, divisions as u32] {
        raw.extend_from_slice(&value.to_le_bytes());
    }
    let neutral = |index: usize| ((index as f64 * 65535.0 / (divisions - 1) as f64).round()) as u32;
    for red in 0..divisions {
        for green in 0..divisions {
            for blue in 0..divisions {
                let position = [red, green, blue].map(|i| i as f32 / (divisions - 1) as f32);
                let target = map(position);
                for (channel, index) in [red, green, blue].into_iter().enumerate() {
                    let value = (target[channel].clamp(0.0, 1.0) * 65535.0).round() as u32;
                    let delta = value.wrapping_sub(neutral(index)) & 0xFFFF;
                    raw.extend_from_slice(&(delta as u16).to_le_bytes());
                }
            }
        }
    }
    // sRGB-Primärfarben, sRGB-Gamma, Beschneiden, Stärke 0–2.
    for value in [0u32, 1, 0] {
        raw.extend_from_slice(&value.to_le_bytes());
    }
    raw.extend_from_slice(&0.0f64.to_le_bytes());
    raw.extend_from_slice(&2.0f64.to_le_bytes());

    pack(&raw)
}

#[test]
pub(super) fn neutrale_tabelle_aendert_nichts() {
    let table = ColorTable::decode(&encode_table(9, |rgb| rgb)).unwrap();

    for color in [
        [0.2, 0.5, 0.8],
        [0.9, 0.1, 0.4],
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
    ] {
        let linear = color.map(srgb_to_linear);
        let result = table.apply(linear, 1.0);
        for channel in 0..3 {
            assert!(
                (result[channel] - linear[channel]).abs() < 0.01,
                "{color:?} → {result:?}"
            );
        }
    }
}

#[test]
pub(super) fn tabelle_wirkt_mit_staerke() {
    // Tauscht Rot und Blau.
    let table = ColorTable::decode(&encode_table(17, |[r, g, b]| [b, g, r])).unwrap();
    let red = [1.0, 0.0, 0.0];

    let full = table.apply(red, 1.0);
    let none = table.apply(red, 0.0);

    assert!(full[2] > 0.9 && full[0] < 0.1, "{full:?}");
    assert!((none[0] - 1.0).abs() < 0.01 && none[2] < 0.01, "{none:?}");
}

/// Baut eine Farbton-Sättigung-Tabelle wie Adobe, `delta` je Punkt.
pub(super) fn encode_hue_sat(delta: [f32; 3]) -> String {
    let (hues, sats, values) = (6usize, 4usize, 2usize);
    let mut raw = Vec::new();
    for value in [0u32, 2, hues as u32, sats as u32, values as u32] {
        raw.extend_from_slice(&value.to_le_bytes());
    }
    for _ in 0..hues * sats * values {
        for part in delta {
            raw.extend_from_slice(&part.to_le_bytes());
        }
    }
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw.extend_from_slice(&0.0f64.to_le_bytes());
    raw.extend_from_slice(&1.0f64.to_le_bytes());
    pack(&raw)
}

pub(super) fn pack(raw: &[u8]) -> String {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(raw).unwrap();
    let mut packed = (raw.len() as u32).to_le_bytes().to_vec();
    packed.extend(encoder.finish().unwrap());
    let mut text = String::new();
    for chunk in packed.chunks(4) {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        let mut value = u32::from_le_bytes(word);
        let count = if chunk.len() == 4 { 5 } else { chunk.len() + 1 };
        for _ in 0..count {
            text.push(char::from(ALPHABET[(value % 85) as usize]));
            value /= 85;
        }
    }
    text
}

#[test]
pub(super) fn farbton_saettigungs_tabelle_wirkt() {
    let neutral = ColorTable::decode(&encode_hue_sat([0.0, 1.0, 1.0])).unwrap();
    let grey_out = ColorTable::decode(&encode_hue_sat([0.0, 0.0, 1.0])).unwrap();
    let color = [0.6, 0.2, 0.1];

    let unchanged = neutral.apply(color, 1.0);
    let gray = grey_out.apply(color, 1.0);

    for channel in 0..3 {
        assert!(
            (unchanged[channel] - color[channel]).abs() < 0.01,
            "{unchanged:?}"
        );
    }
    assert!(
        (gray[0] - gray[1]).abs() < 0.02 && (gray[1] - gray[2]).abs() < 0.02,
        "{gray:?}"
    );
}

#[test]
pub(super) fn kaputte_texte_ergeben_nichts() {
    assert!(ColorTable::decode("").is_none());
    assert!(ColorTable::decode("äöü").is_none());
    assert!(ColorTable::decode("00000000000").is_none());
}

#[test]
pub(super) fn farbraeume_sind_umkehrbar() {
    let space = Space::new(2, 2).unwrap();
    let color = [0.3, 0.5, 0.1];
    let back = multiply(&space.from_table, multiply(&space.to_table, color));
    for channel in 0..3 {
        assert!((back[channel] - color[channel]).abs() < 1e-4);
    }
}
