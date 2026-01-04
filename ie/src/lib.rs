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
	pub fn new(theme: Theme, ocr_detection: impl AsRef<std::path::Path>, ocr_recognition: impl AsRef<std::path::Path>, ocr_charsset: impl AsRef<std::path::Path>) -> Self {
		Self {
			theme,
			ocr: ocr::Ocr::new(ocr_detection, ocr_recognition, ocr_charsset),
		}
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

	pub fn ocr_available(&self) -> bool {
		self.ocr.is_available()
	}

	pub fn ocr_init_error(&self) -> Option<&str> {
		self.ocr.init_error()
	}
}