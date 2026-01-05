mod image;
pub use image::*;
mod ocr;
mod theme;
pub use theme::*;
mod util;

pub mod screen;

pub struct Ie {
	theme: Theme,
	ocr: ocr::Ocr,
}

impl Ie {
	pub fn try_new(
		theme: Theme,
		ocr_detection: impl AsRef<std::path::Path>,
		ocr_recognition: impl AsRef<std::path::Path>,
		ocr_charsset: impl AsRef<std::path::Path>,
	) -> anyhow::Result<Self> {
		Ok(Self {
			theme,
			ocr: ocr::Ocr::try_new(ocr_detection, ocr_recognition, ocr_charsset)?,
		})
	}

	/// Backwards-compatible constructor.
	///
	/// Prefer [`Ie::try_new`] so the caller can handle missing model files gracefully.
	pub fn new(theme: Theme, ocr_detection: impl AsRef<std::path::Path>, ocr_recognition: impl AsRef<std::path::Path>, ocr_charsset: impl AsRef<std::path::Path>) -> Self {
		Self::try_new(theme, ocr_detection, ocr_recognition, ocr_charsset)
			.expect("IE initialization failed (OCR models missing?)")
	}
	
	pub fn util_party_header_text(&self, image: Image) -> String {
		util::party_header_text(image, self.theme, &self.ocr)
	}
	
	pub fn relicreward_get_rewards(&self, image: Image) -> screen::relicreward::Rewards {
		screen::relicreward::get_rewards(image, self.theme, &self.ocr)
	}
	
	pub fn relicreward_get_selected(&self, image: Image) -> u32 {
		screen::relicreward::get_selected(image, self.theme)
	}

	/// Cheap screen check for the Void Fissure rewards screen (no OCR).
	pub fn relicreward_is_screen(&self, image: Image) -> bool {
		screen::relicreward::is_screen(image, self.theme)
	}
}