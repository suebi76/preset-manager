//! Tests für die Vorschau-Rechnung.

use crate::preset::adjustments::HslColor;
use crate::preset::color_table::tests::encode_table;
use crate::preset::settings::parse_settings_for_tests;

use super::color_math::hue_weight;
use super::*;

pub(super) fn settings(attributes: &str) -> PresetSettings {
    parse_settings_for_tests(&format!(
        r#"<x:xmpmeta><rdf:RDF><rdf:Description crs:PresetType="Normal" {attributes}/></rdf:RDF></x:xmpmeta>"#
    ))
}

pub(super) fn mean(image: &Image) -> [f32; 3] {
    let n = image.pixels.len() as f32;
    image.pixels.iter().fold([0.0; 3], |sum, p| {
        [sum[0] + p[0] / n, sum[1] + p[1] / n, sum[2] + p[2] / n]
    })
}

#[test]
pub(super) fn ohne_einstellungen_bleibt_das_bild() {
    let chart = test_chart(60, 40);
    let result = develop(&chart, &settings(""), None);

    for (a, b) in chart.pixels.iter().zip(&result.pixels) {
        for channel in 0..3 {
            assert!((a[channel] - b[channel]).abs() < 0.002, "{a:?} → {b:?}");
        }
    }
}

#[test]
pub(super) fn belichtung_und_temperatur_wirken_in_die_richtige_richtung() {
    let chart = test_chart(60, 40);
    let base = mean(&chart);

    let brighter = mean(&develop(
        &chart,
        &settings(r#"crs:Exposure2012="+1.0""#),
        None,
    ));
    let warmer = mean(&develop(
        &chart,
        &settings(r#"crs:IncrementalTemperature="+40""#),
        None,
    ));

    assert!(brighter.iter().zip(&base).all(|(a, b)| a > b));
    assert!(warmer[0] - warmer[2] > base[0] - base[2] + 0.05);
}

#[test]
pub(super) fn schwarzweiss_hat_keine_farbe() {
    let chart = test_chart(40, 30);
    let result = develop(&chart, &settings(r#"crs:ConvertToGrayscale="True""#), None);

    assert!(result
        .pixels
        .iter()
        .all(|p| (p[0] - p[1]).abs() < 1e-5 && (p[1] - p[2]).abs() < 1e-5));
}

#[test]
pub(super) fn farbtabelle_des_profils_wird_angewendet() {
    let table = [ColorTable::decode(&encode_table(9, |[r, g, b]| [b, g, r])).unwrap()];
    let red = Image {
        width: 1,
        height: 1,
        pixels: vec![[1.0, 0.0, 0.0]],
    };

    let result = develop(
        &red,
        &settings(""),
        Some(LookTable {
            tables: &table,
            amount: 1.0,
        }),
    );

    assert!(
        result.pixels[0][2] > 0.9 && result.pixels[0][0] < 0.1,
        "{:?}",
        result.pixels
    );
}

#[test]
pub(super) fn nennt_was_fehlt() {
    let s = settings(r#"crs:WhiteBalance="Custom" crs:Temperature="6500" crs:RedHue="+20""#);

    let missing = missing(&s, None);

    assert!(missing.contains(&Missing::FixedWhiteBalance));
    assert!(missing.contains(&Missing::Calibration));
    assert!(!missing.contains(&Missing::Masks));
}

#[test]
pub(super) fn farbgewichte_decken_den_farbkreis_ab() {
    for degrees in (0..360).step_by(5) {
        let total: f32 = all_degrees()
            .into_iter()
            .map(|center| hue_weight(degrees as f32, center))
            .sum();
        assert!(total > 0.0, "{degrees}° gehört zu keiner Farbe");
    }
}

/// Alle Farben des Farbmischers in Grad.
pub(super) fn all_degrees() -> Vec<f32> {
    HslColor::ALL
        .iter()
        .map(|color| color.hue_degrees())
        .collect()
}
