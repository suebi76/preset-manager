//! Farbräume und Gamma: Umrechnung zwischen Linear-sRGB und dem Farbraum
//! der Tabelle; kleine Matrixrechnung.

use super::{Gamma, Space};

impl Space {
    pub(super) fn new(primaries: u32, gamma: u32) -> Option<Self> {
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
    pub(super) fn encode(self, value: f32) -> f32 {
        match self {
            Self::Linear => value,
            Self::Srgb => linear_to_srgb(value),
            Self::Pro => value.powf(1.0 / 1.8),
            Self::Two2 => value.powf(1.0 / 2.2),
        }
    }

    pub(super) fn decode(self, value: f32) -> f32 {
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

pub(super) fn multiply(matrix: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| (0..3).map(|k| matrix[row][k] * v[k]).sum())
}

pub(super) fn product(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
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
