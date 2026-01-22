//! Image engine & CV pipeline used by WFBuddy.
//!
//! This crate contains the computer vision and OCR logic. The higher-level UI
//! (iced) and application state live in the `wfbuddy` crate.

mod image;
pub use image::*;

mod ocr;
pub mod screen;
pub mod util;

mod theme;
pub use theme::*;

/// Computer vision engine.
///
/// Owns an OCR engine and a sampled UI theme.
pub struct Ie {
    ocr: crate::ocr::Ocr,
    theme: Theme,
}

impl Ie {
    /// Create a new engine instance.
    pub fn new(
        detection: impl AsRef<std::path::Path>,
        recognition: impl AsRef<std::path::Path>,
        charsset: impl AsRef<std::path::Path>,
        theme: Theme,
    ) -> Self {
        let ocr = crate::ocr::Ocr::new(detection, recognition, charsset);
        Self { ocr, theme }
    }

    /// Replace the current UI theme (useful when re-sampling from the options menu).
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// Detect relic rewards and timer from a screen capture.
    pub fn relicreward_get_rewards(&self, img: &OwnedImage) -> screen::relicreward::Rewards {
        // The detection logic is resolution-independent, so we avoid resizing here.
        screen::relicreward::get_rewards(img.as_image(), self.theme, &self.ocr)
    }

    /// Detect which reward slot is currently selected.
    pub fn relicreward_get_selected(&self, img: &OwnedImage) -> Option<usize> {
        screen::relicreward::get_selected(img.as_image(), self.theme)
    }

    /// Try to OCR the party header text (returns `None` if not found).
    pub fn util_party_header_text(&self, img: &OwnedImage) -> Option<String> {
        util::party_header_text(img.as_image(), self.theme, &self.ocr)
    }
}
