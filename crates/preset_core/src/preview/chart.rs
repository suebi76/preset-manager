//! Die Testtafel, wenn kein eigenes Foto gewählt ist.

use super::color_math::hsv_to_rgb;
use super::Image;

/// Eine Testtafel, wenn kein eigenes Foto gewählt ist: Grauverlauf,
/// Farbverlauf und typische Motivfarben (Haut, Himmel, Laub, Sand).
pub fn test_chart(width: usize, height: usize) -> Image {
    let mut pixels = Vec::with_capacity(width * height);
    let swatches: [[f32; 3]; 6] = [
        [0.85, 0.64, 0.52], // helle Haut
        [0.55, 0.36, 0.26], // dunkle Haut
        [0.38, 0.58, 0.85], // Himmel
        [0.28, 0.45, 0.18], // Laub
        [0.88, 0.80, 0.62], // Sand
        [0.70, 0.12, 0.15], // Rot
    ];
    for y in 0..height {
        let v = y as f32 / height.max(1) as f32;
        for x in 0..width {
            let u = x as f32 / width.max(1).saturating_sub(1).max(1) as f32;
            let pixel = if v < 0.3 {
                [u; 3]
            } else if v < 0.6 {
                let light = 1.0 - (v - 0.3) / 0.3 * 0.6;
                hsv_to_rgb(u * 360.0, 0.75, light)
            } else {
                let index = ((u * swatches.len() as f32) as usize).min(swatches.len() - 1);
                let shade = 0.55 + 0.45 * (1.0 - (v - 0.6) / 0.4);
                swatches[index].map(|c| c * shade)
            };
            pixels.push(pixel);
        }
    }
    Image {
        width,
        height,
        pixels,
    }
}
