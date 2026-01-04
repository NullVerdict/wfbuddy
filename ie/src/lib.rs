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
	
	pub fn util_party_header_text_scaled(&self, image: Image, ui_scale: f32) -> String {
		util::party_header_text_scaled(image, self.theme, &self.ocr, ui_scale)
	}
	
	pub fn util_party_header_text(&self, image: Image) -> String {
		self.util_party_header_text_scaled(image, 1.0)
	}
	
	pub fn relicreward_get_rewards(&self, image: Image, ui_scale: f32) -> screen::relicreward::Rewards {
		screen::relicreward::get_rewards(image, self.theme, &self.ocr, ui_scale)
	}

	pub fn relicreward_get_rewards_default(&self, image: Image) -> screen::relicreward::Rewards {
		self.relicreward_get_rewards(image, 1.0)
	}
	
	pub fn relicreward_get_selected(&self, image: Image, ui_scale: f32) -> u32 {
		screen::relicreward::get_selected(image, self.theme, ui_scale)
	}

	pub fn relicreward_get_selected_default(&self, image: Image) -> u32 {
		self.relicreward_get_selected(image, 1.0)
	}
}