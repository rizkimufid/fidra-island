//! Dominant color extraction from album artwork images.
//!
//! Decodes and samples downscaled thumbnail pixels to find the most vibrant accent color
//! (prioritizing high saturation and moderate luminance, avoiding dull grays,
//! pure whites, or near-black backgrounds).

use image::ImageReader;
use std::io::Cursor;

/// RGB color tuple `(r, g, b)`.
pub type RgbColor = (u8, u8, u8);

/// Default fallback ambient tint (cool obsidian periwinkle `#38BDF8`).
pub const DEFAULT_AMBIENT_COLOR: RgbColor = (56, 189, 248);

/// Extracts the dominant vibrant RGB color from raw JPEG/PNG image bytes.
///
/// Fast execution (< 2ms): decodes directly, downsamples to 24x24, and scores
/// pixels based on saturation and balanced lightness.
pub fn extract_dominant_color(bytes: &[u8]) -> Option<RgbColor> {
    let cursor = Cursor::new(bytes);
    let reader = ImageReader::new(cursor).with_guessed_format().ok()?;
    let img = reader.decode().ok()?;
    let small = img.thumbnail(24, 24).to_rgb8();

    let mut best_score = 0.0f32;
    let mut best_color = None;

    for pixel in small.pixels() {
        let r = pixel[0] as f32 / 255.0;
        let g = pixel[1] as f32 / 255.0;
        let b = pixel[2] as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let lightness = (max + min) / 2.0;

        // Skip near-black (< 0.15) and near-white (> 0.88)
        if lightness < 0.15 || lightness > 0.88 {
            continue;
        }

        let saturation = if delta == 0.0 {
            0.0
        } else {
            delta / (1.0 - (2.0 * lightness - 1.0).abs())
        };

        // Skip dull grays/desaturated tones (< 0.20 saturation)
        if saturation < 0.20 {
            continue;
        }

        // Score: prioritize high saturation with balanced lightness (~0.5)
        let score = saturation * (1.0 - (lightness - 0.5).abs() * 1.5);
        if score > best_score {
            best_score = score;
            best_color = Some((pixel[0], pixel[1], pixel[2]));
        }
    }

    best_color
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_bytes_return_none() {
        assert_eq!(extract_dominant_color(&[0, 1, 2, 3]), None);
    }
}
