//! Kleine Rechenbausteine: Helligkeit, weiche Übergänge, Farbton-Gewichte,
//! HSV, Rauschen für die Körnung, Weichzeichnen für Klarheit und Struktur.

/// Helligkeit nach Rec. 709 auf sRGB-kodierten Werten (für Bildbearbeitung
/// genau genug).
pub(super) fn luma(rgb: [f32; 3]) -> f32 {
    0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
}

/// Setzt die Helligkeit auf `to`, ohne die Farbverhältnisse zu ändern.
pub(super) fn scale_light(rgb: [f32; 3], from: f32, to: f32) -> [f32; 3] {
    if from > 0.01 {
        rgb.map(|value| value * to / from)
    } else {
        rgb.map(|value| value + (to - from))
    }
}

pub(super) fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Punktkurve (0–1) mit linearer Interpolation – Camera Raw glättet stärker,
/// für die Vorschau genügt das.
pub(super) fn curve(points: &[(f32, f32)], x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let scaled: Vec<(f32, f32)> = points.iter().map(|(a, b)| (a * 255.0, b * 255.0)).collect();
    crate::preset::adjustments::curve_value(&scaled, x * 255.0) / 255.0
}

/// Wie stark ein Farbton zu einer Farbe des Mischers gehört (0–1).
pub(super) fn hue_weight(hue: f32, center: f32) -> f32 {
    let distance = ((hue - center + 540.0).rem_euclid(360.0) - 180.0).abs();
    let width = if (20.0..=70.0).contains(&center) {
        28.0
    } else {
        40.0
    };
    (1.0 - distance / width).max(0.0)
}

pub(super) fn hue_rgb(hue: f32) -> [f32; 3] {
    hsv_to_rgb(hue, 1.0, 1.0)
}

pub(super) fn rgb_to_hsv([r, g, b]: [f32; 3]) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta < 1e-6 {
        0.0
    } else if (max - r).abs() < f32::EPSILON {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() < f32::EPSILON {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let sat = if max <= 1e-6 { 0.0 } else { delta / max };
    (hue, sat, max)
}

pub(super) fn hsv_to_rgb(hue: f32, sat: f32, value: f32) -> [f32; 3] {
    let c = value * sat;
    let h = hue.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = value - c;
    [r + m, g + m, b + m]
}

/// Gleichmäßiges Rauschen −1 … +1, für jedes Pixel immer dasselbe.
pub(super) fn hash_noise(index: usize) -> f32 {
    let mut x = index as u32 ^ 0x9E37_79B9;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x as f32 / u32::MAX as f32) * 2.0 - 1.0
}

/// Kastenfilter in zwei Durchgängen (waagerecht, senkrecht).
pub(super) fn box_blur(values: &[f32], width: usize, height: usize, radius: usize) -> Vec<f32> {
    if radius == 0 || width == 0 || height == 0 {
        return values.to_vec();
    }
    let pass = |input: &[f32], horizontal: bool| -> Vec<f32> {
        let mut output = vec![0.0; input.len()];
        let (lines, length) = if horizontal {
            (height, width)
        } else {
            (width, height)
        };
        for line in 0..lines {
            let at = |i: usize| {
                if horizontal {
                    line * width + i
                } else {
                    i * width + line
                }
            };
            let mut sum = 0.0;
            let mut count = 0.0;
            for i in 0..=radius.min(length - 1) {
                sum += input[at(i)];
                count += 1.0;
            }
            for i in 0..length {
                output[at(i)] = sum / count;
                if i + radius + 1 < length {
                    sum += input[at(i + radius + 1)];
                    count += 1.0;
                }
                if i >= radius {
                    sum -= input[at(i - radius)];
                    count -= 1.0;
                }
            }
        }
        output
    };
    pass(&pass(values, true), false)
}
