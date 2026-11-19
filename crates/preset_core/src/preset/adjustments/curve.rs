//! Gradationskurven: Kanäle, welche Kurven ein Preset ändert, und der
//! Kurvenverlauf als natürlicher Spline wie in Camera Raw.

use crate::preset::settings::{CurvePoint, PresetSettings};

/// Ein Kanal der Gradationskurve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveChannel {
    /// Alle Kanäle (Helligkeit).
    Master,
    /// Rot.
    Red,
    /// Grün.
    Green,
    /// Blau.
    Blue,
}

impl CurveChannel {
    /// Alle Kanäle.
    pub const ALL: [CurveChannel; 4] = [
        CurveChannel::Master,
        CurveChannel::Red,
        CurveChannel::Green,
        CurveChannel::Blue,
    ];

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Master => "ToneCurvePV2012",
            Self::Red => "ToneCurvePV2012Red",
            Self::Green => "ToneCurvePV2012Green",
            Self::Blue => "ToneCurvePV2012Blue",
        }
    }
}

impl PresetSettings {
    /// Die Punktkurven, die nicht gerade sind.
    pub fn point_curves(&self) -> Vec<(CurveChannel, &[CurvePoint])> {
        CurveChannel::ALL
            .into_iter()
            .filter_map(|channel| {
                let points = self.curve(channel.key())?;
                (!is_linear(points)).then_some((channel, points))
            })
            .collect()
    }
}

/// `true`, wenn die Kurve die Gerade von (0, 0) nach (255, 255) ist.
pub(super) fn is_linear(points: &[CurvePoint]) -> bool {
    points.iter().all(|(x, y)| (x - y).abs() < 0.5)
}

/// Der Wert einer Punktkurve an der Stelle `x` (0–255), wie Camera Raw sie
/// zeichnet: eine glatte Kurve (natürlicher kubischer Spline) durch die
/// Punkte, auf 0–255 begrenzt. Vor dem ersten und nach dem letzten Punkt
/// bleibt sie waagerecht.
pub fn curve_value(points: &[CurvePoint], x: f32) -> f32 {
    let mut sorted: Vec<CurvePoint> = points.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    sorted.dedup_by(|a, b| (a.0 - b.0).abs() < f32::EPSILON);
    let (Some(first), Some(last)) = (sorted.first().copied(), sorted.last().copied()) else {
        return x;
    };
    if sorted.len() == 1 || x <= first.0 {
        return first.1.clamp(0.0, 255.0);
    }
    if x >= last.0 {
        return last.1.clamp(0.0, 255.0);
    }

    let second_derivatives = spline_second_derivatives(&sorted);
    let index = sorted
        .windows(2)
        .position(|pair| x <= pair[1].0)
        .unwrap_or(sorted.len() - 2);
    let (x0, y0) = sorted[index];
    let (x1, y1) = sorted[index + 1];
    let (m0, m1) = (second_derivatives[index], second_derivatives[index + 1]);
    let h = x1 - x0;
    let a = (x1 - x) / h;
    let b = (x - x0) / h;
    let y = a * y0 + b * y1 + ((a * a * a - a) * m0 + (b * b * b - b) * m1) * h * h / 6.0;
    y.clamp(0.0, 255.0)
}

/// Zweite Ableitungen eines natürlichen kubischen Splines (Thomas-Verfahren).
fn spline_second_derivatives(points: &[CurvePoint]) -> Vec<f32> {
    let n = points.len();
    let mut result = vec![0.0; n];
    if n < 3 {
        return result;
    }
    let mut upper = vec![0.0; n];
    let mut rhs = vec![0.0; n];
    for i in 1..n - 1 {
        let (x_prev, y_prev) = points[i - 1];
        let (x, y) = points[i];
        let (x_next, y_next) = points[i + 1];
        let sigma = (x - x_prev) / (x_next - x_prev);
        let p = sigma * upper[i - 1] + 2.0;
        upper[i] = (sigma - 1.0) / p;
        let slope = (y_next - y) / (x_next - x) - (y - y_prev) / (x - x_prev);
        rhs[i] = (6.0 * slope / (x_next - x_prev) - sigma * rhs[i - 1]) / p;
    }
    for i in (1..n - 1).rev() {
        result[i] = upper[i] * result[i + 1] + rhs[i];
    }
    result
}
