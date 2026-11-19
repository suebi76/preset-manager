//! Tests für die Deutung der Einstellungen.

use crate::preset::settings::{parse_settings, PresetSettings};

use super::*;

fn settings(attributes: &str, body: &str) -> PresetSettings {
    parse_settings(&format!(
        r#"<x:xmpmeta><rdf:RDF><rdf:Description crs:PresetType="Normal" {attributes}>{body}</rdf:Description></rdf:RDF></x:xmpmeta>"#
    ))
    .unwrap()
}

fn curve(points: &[(u8, u8)]) -> String {
    let items: String = points
        .iter()
        .map(|(x, y)| format!("<rdf:li>{x}, {y}</rdf:li>"))
        .collect();
    format!("<crs:ToneCurvePV2012><rdf:Seq>{items}</rdf:Seq></crs:ToneCurvePV2012>")
}

#[test]
fn zeigt_nur_geaenderte_regler() {
    let settings = settings(
        r#"crs:Exposure2012="+0.50" crs:Contrast2012="0" crs:Sharpness="40" crs:SharpenRadius="+1.0" crs:LuminanceSmoothing="20""#,
        "",
    );

    let basic: Vec<_> = settings
        .sliders(Panel::Basic)
        .iter()
        .map(|s| (s.spec.key, s.value))
        .collect();
    let detail: Vec<_> = settings
        .sliders(Panel::Detail)
        .iter()
        .map(|s| s.spec.key)
        .collect();

    assert_eq!(basic, vec![("Exposure2012", 0.5)]);
    assert_eq!(detail, vec!["LuminanceSmoothing"]);
}

#[test]
fn abhaengige_regler_nur_mit_ihrem_hauptregler() {
    let without = settings(r#"crs:GrainAmount="0" crs:GrainSize="40""#, "");
    let with = settings(r#"crs:GrainAmount="20" crs:GrainSize="40""#, "");

    assert!(without.sliders(Panel::Effects).is_empty());
    assert_eq!(with.sliders(Panel::Effects).len(), 2);
}

#[test]
fn gerade_kurven_zaehlen_nicht() {
    let linear = settings("", &curve(&[(0, 0), (255, 255)]));
    let bent = settings("", &curve(&[(0, 20), (128, 128), (255, 255)]));

    assert!(linear.point_curves().is_empty());
    assert_eq!(bent.point_curves().len(), 1);
}

#[test]
fn kurve_geht_durch_ihre_punkte() {
    let points = [(0.0, 20.0), (64.0, 50.0), (192.0, 210.0), (255.0, 240.0)];

    for (x, y) in points {
        assert!((curve_value(&points, x) - y).abs() < 0.01, "{x}");
    }
    assert!(curve_value(&points, 128.0) > 100.0 && curve_value(&points, 128.0) < 160.0);
    assert_eq!(curve_value(&[(0.0, 0.0), (255.0, 255.0)], 100.0), 100.0);
}

#[test]
fn beschreibt_einen_matten_warmen_look() {
    let settings = settings(
        r#"crs:Exposure2012="+0.30" crs:IncrementalTemperature="+15" crs:Saturation="-20"
           crs:SaturationAdjustmentGreen="-60" crs:GrainAmount="25" crs:PostCropVignetteAmount="-20"
           crs:SplitToningShadowHue="200" crs:SplitToningShadowSaturation="20""#,
        &curve(&[(0, 30), (128, 128), (255, 235)]),
    );

    let facts = settings.facts();

    for expected in [
        Fact::Brighter,
        Fact::FadedBlacks,
        Fact::MutedWhites,
        Fact::Warmer,
        Fact::LessColorful,
        Fact::Color {
            color: HslColor::Green,
            change: ColorChange::Desaturated,
        },
        Fact::Toned {
            zone: ToneZone::Shadows,
            hue: 200.0,
        },
        Fact::DarkVignette,
        Fact::Grain,
    ] {
        assert!(facts.contains(&expected), "{expected:?} fehlt in {facts:?}");
    }
    assert!(!facts.contains(&Fact::MoreContrast));
}

#[test]
fn schwarzweiss_ohne_farbaussagen() {
    let settings = settings(
        r#"crs:ConvertToGrayscale="True" crs:Saturation="+40" crs:GrayMixerRed="-20" crs:SaturationAdjustmentRed="+50""#,
        "",
    );

    assert_eq!(settings.facts(), vec![Fact::BlackAndWhite]);
    assert!(settings.mixer().is_empty());
    assert_eq!(settings.gray_mixer(), vec![(HslColor::Red, -20.0)]);
}

#[test]
fn weissabgleich() {
    assert_eq!(settings("", "").white_balance(), WhiteBalance::AsShot);
    assert_eq!(
        settings(r#"crs:WhiteBalance="Auto""#, "").white_balance(),
        WhiteBalance::Named("Auto".to_owned())
    );
    let custom = settings(
        r#"crs:WhiteBalance="Custom" crs:Temperature="7500" crs:Tint="+10""#,
        "",
    );
    assert_eq!(
        custom.white_balance(),
        WhiteBalance::Custom {
            temperature: Some(7500.0),
            tint: Some(10.0)
        }
    );
    assert!(custom.facts().contains(&Fact::Warmer));
}

#[test]
fn neutrales_preset_hat_keine_aussagen() {
    let settings = settings(
        r#"crs:Exposure2012="0.00" crs:Contrast2012="0" crs:ColorGradeBlending="50""#,
        &curve(&[(0, 0), (255, 255)]),
    );

    assert!(settings.facts().is_empty());
    assert_eq!(settings.color_grading(), None);
}
