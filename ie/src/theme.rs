use crate::{Color, Image};

/// UI theme colors sampled from the game options screen.
///
/// The app uses these colors to robustly detect UI highlights (selection boxes,
/// progress bars, etc.) across different graphics settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
}

impl Theme {
    pub const WHITE: Self = Self {
        primary: Color::WHITE,
        secondary: Color::WHITE,
    };

    /// Sample theme colors from a Warframe options screen capture.
    ///
    /// The original implementation assumed a 1920×1080 capture. To support
    /// arbitrary resolutions and UI scaling, we compute sampling rectangles
    /// as *relative* coordinates.
    pub fn from_options(image: Image) -> Self {
        let w = image.width().max(1);
        let h = image.height().max(1);

        // Ratios were derived from the previous hard-coded 1920×1080 coordinates:
        // primary: (110,87) size (20×1)
        // secondary: (146,181) size (14×8)
        let bar_x = (w as f32 * 110.0 / 1920.0).round() as u32;
        let bar_y = (h as f32 * 87.0 / 1080.0).round() as u32;
        let bar_w = (w as f32 * 20.0 / 1920.0).round().max(1.0) as u32;
        let bar_h = (h as f32 * 1.0 / 1080.0).round().max(1.0) as u32;

        let mouse_x = (w as f32 * 146.0 / 1920.0).round() as u32;
        let mouse_y = (h as f32 * 181.0 / 1080.0).round() as u32;
        let mouse_w = (w as f32 * 14.0 / 1920.0).round().max(1.0) as u32;
        let mouse_h = (h as f32 * 8.0 / 1080.0).round().max(1.0) as u32;

        Self {
            primary: image.sub_image(bar_x, bar_y, bar_w, bar_h).average_color(),
            secondary: image
                .sub_image(mouse_x, mouse_y, mouse_w, mouse_h)
                .average_color(),
        }
    }
}
