//! Farbtabellen (3D-LUT) aus Profilen lesen und anwenden.
//!
//! Kreative Profile von Adobe (und viele gekaufte) tragen ihren Look als
//! Farbtabelle in der Datei: `crs:RGBTable="<Kennung>"` verweist auf
//! `crs:Table_<Kennung>="…"`. Der Text ist so kodiert wie in Adobes DNG-SDK
//! (`dng_big_table`), ermittelt am Entwicklungsrechner (Versuch in
//! `Planung/06-Roadmap.md`, Version 1.6):
//!
//! 1. Base85 mit eigenem Alphabet ([`ALPHABET`]): je 5 Zeichen ergeben
//!    4 Bytes, niedrigste Stelle zuerst, Bytes in Little Endian.
//! 2. Die ersten 4 Bytes nennen die entpackte Größe, der Rest ist zlib.
//! 3. Entpackt (Little Endian): Typ (1 = RGB-Tabelle), Version (1),
//!    Dimensionen (3), Teilungen n, dann n³ Punkte mit je drei `u16` –
//!    Rot außen, Blau innen –, gespeichert als Abstand zur neutralen Tabelle.
//!    Danach Primärfarben, Gamma, Farbumfang und der Bereich der Stärke
//!    (zwei `f64`).
//!
//! Daneben gibt es `crs:LookTable` (Typ 0, Version 2): eine Tabelle über
//! Farbton × Sättigung × Helligkeit wie die „HueSatMap“ der DNG-Spezifikation.
//! Jeder Punkt enthält drei `f32` – Farbtonverschiebung in Grad,
//! Sättigungs- und Helligkeitsfaktor –, Helligkeit außen, Sättigung innen;
//! danach die Kodierung der Helligkeitsachse (0 linear, 1 sRGB) und der
//! Bereich der Stärke. Gerechnet wird in linearem ProPhoto RGB.
//!
//! Was die App nicht versteht (andere Dimensionen, unbekannte Farbräume),
//! wird nicht angewendet; die Vorschau sagt das dann.

use std::io::Read;

/// Das Base85-Alphabet von Adobe.
pub const ALPHABET: &[u8; 85] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?`'|()[]{}@%$#";

/// Höchstgröße einer entpackten Tabelle – Schutz vor kaputten Dateien.
const MAX_TABLE_BYTES: usize = 64 * 1024 * 1024;

/// Eine dreidimensionale Farbtabelle.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorTable {
    kind: Kind,
    /// Größte erlaubte Stärke (meist 2).
    max_amount: f32,
}

#[derive(Debug, Clone, PartialEq)]
enum Kind {
    /// `crs:RGBTable`: RGB → RGB.
    Rgb {
        /// Punkte je Achse.
        divisions: usize,
        /// Die Werte, 0–1, Rot außen, Blau innen.
        samples: Vec<[f32; 3]>,
        /// In welchem Farbraum die Tabelle rechnet.
        space: Space,
    },
    /// `crs:LookTable`: Farbton, Sättigung, Helligkeit.
    HueSat {
        hues: usize,
        saturations: usize,
        values: usize,
        /// Je Punkt: Farbtonverschiebung (Grad), Sättigungs-, Helligkeitsfaktor.
        deltas: Vec<[f32; 3]>,
        /// Die Helligkeitsachse ist sRGB-kodiert.
        srgb_values: bool,
        space: Space,
    },
}

/// Farbraum der Tabelle: Primärfarben und Gamma.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Space {
    /// Linear-sRGB → Tabellen-RGB (linear).
    to_table: [[f32; 3]; 3],
    /// Tabellen-RGB (linear) → Linear-sRGB.
    from_table: [[f32; 3]; 3],
    gamma: Gamma,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Gamma {
    Linear,
    Srgb,
    /// ProPhoto: 1,8.
    Pro,
    /// 2,2.
    Two2,
}

impl ColorTable {
    /// Dekodiert den Text von `crs:Table_…`. `None`, wenn er sich nicht lesen
    /// lässt oder keine 3D-Tabelle in einem bekannten Farbraum ist.
    pub fn decode(text: &str) -> Option<Self> {
        let packed = base85(text)?;
        let (size, data) = packed.split_first_chunk::<4>()?;
        let size = u32::from_le_bytes(*size) as usize;
        if size > MAX_TABLE_BYTES {
            return None;
        }
        let mut raw = Vec::with_capacity(size);
        flate2::read::ZlibDecoder::new(data)
            .take(MAX_TABLE_BYTES as u64)
            .read_to_end(&mut raw)
            .ok()?;
        Self::from_bytes(&raw)
    }

    fn from_bytes(raw: &[u8]) -> Option<Self> {
        let mut reader = Reader { data: raw, at: 0 };
        match reader.u32()? {
            0 => Self::hue_sat_from(&mut reader),
            1 => Self::rgb_from(&mut reader),
            _ => None,
        }
    }

    fn hue_sat_from(reader: &mut Reader<'_>) -> Option<Self> {
        let _version = reader.u32()?;
        let (hues, saturations, values) = (
            reader.u32()? as usize,
            reader.u32()? as usize,
            reader.u32()? as usize,
        );
        let valid = |n: usize| (1..=256).contains(&n);
        if !valid(hues) || !valid(saturations) || !valid(values) || saturations < 2 {
            return None;
        }
        let points = hues * saturations * values;
        // Erst prüfen, ob die Daten reichen – eine Tabelle, die nur
        // behauptet, groß zu sein, soll keinen Speicher reservieren.
        if reader.remaining() < points * 12 {
            return None;
        }
        let mut deltas = Vec::with_capacity(points);
        for _ in 0..points {
            deltas.push([reader.f32()?, reader.f32()?, reader.f32()?]);
        }
        let srgb_values = reader.u32()? == 1;
        let _min_amount = reader.f64()?;
        let max_amount = reader.f64()? as f32;
        Some(Self {
            kind: Kind::HueSat {
                hues,
                saturations,
                values,
                deltas,
                srgb_values,
                space: Space::new(2, 0)?,
            },
            max_amount: if max_amount.is_finite() && max_amount > 0.0 {
                max_amount
            } else {
                1.0
            },
        })
    }

    fn rgb_from(reader: &mut Reader<'_>) -> Option<Self> {
        let (_version, dimensions) = (reader.u32()?, reader.u32()?);
        let divisions = reader.u32()? as usize;
        if dimensions != 3 || !(2..=128).contains(&divisions) {
            return None;
        }
        let neutral = |index: usize| -> u32 {
            ((index as f64 * 65535.0 / (divisions - 1) as f64).round()) as u32
        };
        if reader.remaining() < divisions.pow(3) * 6 {
            return None;
        }
        let mut samples = Vec::with_capacity(divisions.pow(3));
        for red in 0..divisions {
            for green in 0..divisions {
                for blue in 0..divisions {
                    let mut point = [0.0; 3];
                    for (channel, index) in [red, green, blue].into_iter().enumerate() {
                        let delta = u32::from(reader.u16()?);
                        let value = (delta + neutral(index)) & 0xFFFF;
                        point[channel] = value as f32 / 65535.0;
                    }
                    samples.push(point);
                }
            }
        }
        let primaries = reader.u32()?;
        let gamma = reader.u32()?;
        let _gamut = reader.u32()?;
        let _min_amount = reader.f64()?;
        let max_amount = reader.f64()? as f32;
        let space = Space::new(primaries, gamma)?;
        Some(Self {
            kind: Kind::Rgb {
                divisions,
                samples,
                space,
            },
            max_amount: if max_amount.is_finite() && max_amount > 0.0 {
                max_amount
            } else {
                2.0
            },
        })
    }

    /// Wendet die Tabelle auf eine Farbe in **linearem sRGB** an. `amount`
    /// ist die Stärke des Profils (1 = wie gedacht, 0 = gar nicht).
    pub fn apply(&self, linear_srgb: [f32; 3], amount: f32) -> [f32; 3] {
        let amount = amount.clamp(0.0, self.max_amount);
        match &self.kind {
            Kind::Rgb {
                divisions,
                samples,
                space,
            } => {
                let table_linear = multiply(&space.to_table, linear_srgb);
                let encoded = table_linear.map(|value| space.gamma.encode(value.clamp(0.0, 1.0)));
                let looked_up = lookup_rgb(*divisions, samples, encoded);
                let blended: [f32; 3] = std::array::from_fn(|channel| {
                    (encoded[channel] + amount * (looked_up[channel] - encoded[channel]))
                        .clamp(0.0, 1.0)
                });
                let linear = blended.map(|value| space.gamma.decode(value));
                multiply(&space.from_table, linear)
            }
            Kind::HueSat {
                hues,
                saturations,
                values,
                deltas,
                srgb_values,
                space,
            } => {
                let pro = multiply(&space.to_table, linear_srgb).map(|v| v.max(0.0));
                let (hue, sat, value) = hsv6(pro);
                let axis_value = if *srgb_values {
                    linear_to_srgb(value.min(1.0))
                } else {
                    value.min(1.0)
                };
                let [shift, sat_scale, val_scale] =
                    lookup_hue_sat([*hues, *saturations, *values], deltas, hue, sat, axis_value);
                let hue = hue + shift * amount / 60.0;
                let sat = (sat * (1.0 + (sat_scale - 1.0) * amount)).clamp(0.0, 1.0);
                let value = value * (1.0 + (val_scale - 1.0) * amount);
                multiply(&space.from_table, rgb6(hue, sat, value.max(0.0)))
            }
        }
    }
}

/// Dreifach lineare Interpolation in einer RGB-Tabelle.
fn lookup_rgb(divisions: usize, samples: &[[f32; 3]], rgb: [f32; 3]) -> [f32; 3] {
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
fn lookup_hue_sat(
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
fn hsv6([r, g, b]: [f32; 3]) -> (f32, f32, f32) {
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

fn rgb6(hue: f32, sat: f32, value: f32) -> [f32; 3] {
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

impl Space {
    fn new(primaries: u32, gamma: u32) -> Option<Self> {
        // Linear-sRGB → XYZ (D65).
        const SRGB: [[f32; 3]; 3] = [
            [0.412_456_4, 0.357_576_1, 0.180_437_5],
            [0.212_672_9, 0.715_152_2, 0.072_175],
            [0.019_333_9, 0.119_192, 0.950_304_1],
        ];
        // Adobe RGB → XYZ (D65).
        const ADOBE: [[f32; 3]; 3] = [
            [0.576_730_9, 0.185_554, 0.188_185_2],
            [0.297_376_9, 0.627_349_1, 0.075_274_1],
            [0.027_034_3, 0.070_687_2, 0.991_108_5],
        ];
        // ProPhoto → XYZ (D50).
        const PROPHOTO: [[f32; 3]; 3] = [
            [0.797_674_9, 0.135_191_7, 0.031_353_4],
            [0.288_040_2, 0.711_874_1, 0.000_085_7],
            [0.0, 0.0, 0.825_21],
        ];
        // Bradford D50 → D65.
        const D50_TO_D65: [[f32; 3]; 3] = [
            [0.955_576_6, -0.023_039_3, 0.063_163_6],
            [-0.028_289_5, 1.009_941_6, 0.021_007_7],
            [0.012_298_2, -0.020_483, 1.329_909_8],
        ];
        let table_to_xyz65 = match primaries {
            0 => SRGB,
            1 => ADOBE,
            2 => product(&D50_TO_D65, &PROPHOTO),
            _ => return None,
        };
        let from_table = product(&invert(&SRGB)?, &table_to_xyz65);
        let to_table = invert(&from_table)?;
        let gamma = match gamma {
            0 => Gamma::Linear,
            1 => Gamma::Srgb,
            2 => Gamma::Pro,
            3 => Gamma::Two2,
            _ => return None,
        };
        Some(Self {
            to_table,
            from_table,
            gamma,
        })
    }
}

impl Gamma {
    fn encode(self, value: f32) -> f32 {
        match self {
            Self::Linear => value,
            Self::Srgb => linear_to_srgb(value),
            Self::Pro => value.powf(1.0 / 1.8),
            Self::Two2 => value.powf(1.0 / 2.2),
        }
    }

    fn decode(self, value: f32) -> f32 {
        match self {
            Self::Linear => value,
            Self::Srgb => srgb_to_linear(value),
            Self::Pro => value.powf(1.8),
            Self::Two2 => value.powf(2.2),
        }
    }
}

/// sRGB-kodiert (0–1) → linear.
pub fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear → sRGB-kodiert (0–1).
pub fn linear_to_srgb(value: f32) -> f32 {
    let value = value.max(0.0);
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn multiply(matrix: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| (0..3).map(|k| matrix[row][k] * v[k]).sum())
}

fn product(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    std::array::from_fn(|row| {
        std::array::from_fn(|col| (0..3).map(|k| a[row][k] * b[k][col]).sum())
    })
}

fn invert(m: &[[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-9 {
        return None;
    }
    Some([
        [
            (e * i - f * h) / det,
            (c * h - b * i) / det,
            (b * f - c * e) / det,
        ],
        [
            (f * g - d * i) / det,
            (a * i - c * g) / det,
            (c * d - a * f) / det,
        ],
        [
            (d * h - e * g) / det,
            (b * g - a * h) / det,
            (a * e - b * d) / det,
        ],
    ])
}

/// Base85 nach Adobe: je 5 Zeichen → 4 Bytes (Little Endian). Ein kürzerer
/// Rest ergibt entsprechend weniger Bytes.
fn base85(text: &str) -> Option<Vec<u8>> {
    let mut lookup = [u8::MAX; 256];
    for (index, &character) in ALPHABET.iter().enumerate() {
        lookup[usize::from(character)] = index as u8;
    }
    let digits: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .map(|byte| lookup[usize::from(byte)])
        .collect();
    if digits.contains(&u8::MAX) {
        return None;
    }
    let mut bytes = Vec::with_capacity(digits.len() / 5 * 4 + 4);
    for chunk in digits.chunks(5) {
        let value = chunk
            .iter()
            .rev()
            .fold(0u64, |value, &digit| value * 85 + u64::from(digit));
        let word = (value as u32).to_le_bytes();
        let take = if chunk.len() == 5 {
            4
        } else {
            chunk.len().saturating_sub(1)
        };
        bytes.extend_from_slice(&word[..take]);
    }
    Some(bytes)
}

/// Liest Zahlen aus der entpackten Tabelle.
struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    /// Wie viele Bytes noch ungelesen sind.
    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.at)
    }

    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let bytes = self.data.get(self.at..self.at + N)?;
        self.at += N;
        bytes.try_into().ok()
    }
    fn u16(&mut self) -> Option<u16> {
        self.take().map(u16::from_le_bytes)
    }
    fn u32(&mut self) -> Option<u32> {
        self.take().map(u32::from_le_bytes)
    }
    fn f32(&mut self) -> Option<f32> {
        self.take().map(f32::from_le_bytes)
    }
    fn f64(&mut self) -> Option<f64> {
        self.take().map(f64::from_le_bytes)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn behauptete_riesentabelle_ohne_daten_wird_abgelehnt() {
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
        let neutral =
            |index: usize| ((index as f64 * 65535.0 / (divisions - 1) as f64).round()) as u32;
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
    fn neutrale_tabelle_aendert_nichts() {
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
    fn tabelle_wirkt_mit_staerke() {
        // Tauscht Rot und Blau.
        let table = ColorTable::decode(&encode_table(17, |[r, g, b]| [b, g, r])).unwrap();
        let red = [1.0, 0.0, 0.0];

        let full = table.apply(red, 1.0);
        let none = table.apply(red, 0.0);

        assert!(full[2] > 0.9 && full[0] < 0.1, "{full:?}");
        assert!((none[0] - 1.0).abs() < 0.01 && none[2] < 0.01, "{none:?}");
    }

    /// Baut eine Farbton-Sättigung-Tabelle wie Adobe, `delta` je Punkt.
    fn encode_hue_sat(delta: [f32; 3]) -> String {
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

    fn pack(raw: &[u8]) -> String {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
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
    fn farbton_saettigungs_tabelle_wirkt() {
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
    fn kaputte_texte_ergeben_nichts() {
        assert!(ColorTable::decode("").is_none());
        assert!(ColorTable::decode("äöü").is_none());
        assert!(ColorTable::decode("00000000000").is_none());
    }

    #[test]
    fn farbraeume_sind_umkehrbar() {
        let space = Space::new(2, 2).unwrap();
        let color = [0.3, 0.5, 0.1];
        let back = multiply(&space.from_table, multiply(&space.to_table, color));
        for channel in 0..3 {
            assert!((back[channel] - color[channel]).abs() < 1e-4);
        }
    }
}
