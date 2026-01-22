//! Miscellaneous CV utilities.

use crate::{Image, Theme};

/// Try to read the party header player name.
///
/// This is used as a lightweight signal that we are in a mission/reward screen.
/// The original implementation used fixed 1080p coordinates; we now derive all
/// positions from relative ratios so it works across different resolutions.
///
/// Returns the first OCR string that looks non-empty.
pub fn party_header_text(image: Image, theme: Theme, ocr: &crate::ocr::Ocr) -> Option<String> {
    // Reference capture the ratios were derived from.
    const REF_W: f32 = 1920.0;
    const REF_H: f32 = 1080.0;

    // Avatar box and spacing (relative to 1080p reference).
    let w = image.width() as f32;
    let h = image.height() as f32;
    let sx = w / REF_W;
    let sy = h / REF_H;

    let avatar_x = (96.0 * sx).round() as u32;
    let avatar_y = (40.0 * sy).round() as u32;
    let avatar_w = (94.0 * sx).round().max(1.0) as u32;
    let avatar_h = (94.0 * sy).round().max(1.0) as u32;

    let spacing_x = (324.0 * sx).round() as u32;
    let spacing_y = (175.0 * sy).round() as u32;

    // Player name region relative to avatar origin.
    let name_x = (115.0 * sx).round() as u32;
    let name_y = (124.0 * sy).round() as u32;
    let name_w = (210.0 * sx).round().max(1.0) as u32;
    let name_h = (24.0 * sy).round().max(1.0) as u32;

    // We scan the 2×2 grid of party avatars (up to 4 players).
    for i in 0..4 {
        let gx = i % 2;
        let gy = i / 2;

        let x = avatar_x + gx * spacing_x;
        let y = avatar_y + gy * spacing_y;

        // A quick color check to see if the avatar UI element is present.
        let avatar_avg = image.sub_image(x, y, avatar_w, avatar_h).average_color();
        if avatar_avg.deviation(theme.primary) > 20.0 && avatar_avg.deviation(theme.secondary) > 20.0 {
            continue;
        }

        let name_img = image.sub_image(x + name_x, y + name_y, name_w, name_h);
        let text = name_img.get_text(theme, ocr);
        let text = text.trim().to_string();
        if !text.is_empty() {
            return Some(text);
        }
    }

    None
}
