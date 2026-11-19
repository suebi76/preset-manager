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
//!
//! | Datei | Inhalt |
//! |---|---|
//! | `lookup` | nachschlagen |
//! | `space` | Farbräume, Gamma |
//! | `binary` | Base85, Zahlen lesen |

mod binary;
mod lookup;
mod space;
#[cfg(test)]
pub(crate) mod tests;

pub use space::{linear_to_srgb, srgb_to_linear};

use binary::base85;
use binary::Reader;
use lookup::hsv6;
use lookup::lookup_hue_sat;
use lookup::lookup_rgb;
use lookup::rgb6;
use space::multiply;
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
