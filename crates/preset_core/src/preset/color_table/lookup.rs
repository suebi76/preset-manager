//! In den Tabellen nachschlagen: trilinear in der RGB-Tabelle, im
//! Farbton-Sättigungs-Feld mit HSV.

/// Dreifach lineare Interpolation in einer RGB-Tabelle.
pub(super) fn lookup_rgb(divisions: usize, samples: &[[f32; 3]], rgb: [f32; 3]) -> [f32; 3] {
    let n = divisions;
    let scale = (n - 1) as f32;
    let position = rgb.map(|value| value.clamp(0.0, 1.0) * scale);
    let low = position.map(|value| (value.floor() as usize).min(n - 2));
    let fraction: [f32; 3] = std::array::from_fn(|axis| position[axis] - low[axis] as f32);
    let at = |r: usize, g: usize, b: usize| samples[(r * n + g) * n + b];
    let mut result = [0.0; 3];
    for corner in 0..8 {
        let step = [corner >> 2 & 1, corner >> 1 & 1, corner & 1];
        let weight: f32 = (0..3)
            .map(|axis| {
                if step[axis] == 1 {
                    fraction[axis]
                } else {
                    1.0 - fraction[axis]
                }
            })
            .product();
        let sample = at(low[0] + step[0], low[1] + step[1], low[2] + step[2]);
        for channel in 0..3 {
            result[channel] += weight * sample[channel];
        }
    }
    result
}

/// Interpolation in einer Farbton-Sättigung-Helligkeit-Tabelle. Der Farbton
/// läuft im Kreis; `hue` ist 0–6 (wie in der DNG-Spezifikation).
pub(super) fn lookup_hue_sat(
    [hues, saturations, values]: [usize; 3],
    deltas: &[[f32; 3]],
    hue: f32,
    sat: f32,
    value: f32,
) -> [f32; 3] {
    let h = hue.rem_euclid(6.0) / 6.0 * hues as f32;
    let h0 = (h.floor() as usize) % hues;
    let h1 = (h0 + 1) % hues;
    let hf = h - h.floor();
    let s = sat.clamp(0.0, 1.0) * (saturations - 1) as f32;
    let s0 = (s.floor() as usize).min(saturations - 2);
    let sf = s - s0 as f32;
    let (v0, v1, vf) = if values > 1 {
        let v = value.clamp(0.0, 1.0) * (values - 1) as f32;
        let v0 = (v.floor() as usize).min(values - 2);
        (v0, v0 + 1, v - v0 as f32)
    } else {
        (0, 0, 0.0)
    };
    let at = |v: usize, h: usize, s: usize| deltas[(v * hues + h) * saturations + s];
    let mut result = [0.0; 3];
    for (v, wv) in [(v0, 1.0 - vf), (v1, vf)] {
        for (hh, wh) in [(h0, 1.0 - hf), (h1, hf)] {
            for (ss, ws) in [(s0, 1.0 - sf), (s0 + 1, sf)] {
                let weight = wv * wh * ws;
                let sample = at(v, hh, ss);
                for channel in 0..3 {
                    result[channel] += weight * sample[channel];
                }
            }
        }
    }
    result
}

/// RGB → Farbton (0–6), Sättigung, Helligkeit (Maximum), wie in der DNG-SDK.
pub(super) fn hsv6([r, g, b]: [f32; 3]) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    if max <= 0.0 || delta <= 0.0 {
        return (0.0, 0.0, max);
    }
    let hue = if (r - max).abs() < f32::EPSILON {
        (g - b) / delta
    } else if (g - max).abs() < f32::EPSILON {
        2.0 + (b - r) / delta
    } else {
        4.0 + (r - g) / delta
    };
    (hue.rem_euclid(6.0), delta / max, max)
}

pub(super) fn rgb6(hue: f32, sat: f32, value: f32) -> [f32; 3] {
    let h = hue.rem_euclid(6.0);
    let sector = h.floor();
    let f = h - sector;
    let p = value * (1.0 - sat);
    let q = value * (1.0 - sat * f);
    let t = value * (1.0 - sat * (1.0 - f));
    match sector as u32 {
        0 => [value, t, p],
        1 => [q, value, p],
        2 => [p, value, t],
        3 => [p, q, value],
        4 => [t, p, value],
        _ => [value, p, q],
    }
}
