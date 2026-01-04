use crate::Image;
use std::sync::LazyLock;

// Regex is lazily compiled at runtime; this must be a `static` (not `const`) because
// `LazyLock` has interior mutability.
pub static DIGIT_REGEX: LazyLock<regex::Regex> =
	LazyLock::new(|| regex::Regex::new(r"(?<digits>\d+)").unwrap());

pub fn party_header_text_start(image: Image) -> (u32, u32) {
	const AVATAR_START: u32 = 96;
	const AVATAR_SIZE: u32 = 44;
	const AVATAR_SPACING: u32 = 4;
	const AVATAR_BAR_Y: u32 = 86;
	const AVATAR_BAR_W: u32 = 40;
	const AVATAR_BAR_H: u32 = 2;
	const TEXT_Y: u32 = 49;
	
	let primary_color = image.sub_image(AVATAR_START + 1, AVATAR_BAR_Y, AVATAR_BAR_W, AVATAR_BAR_H).average_color();
	// println!("primary color is {primary_color:?}");
	
	// not actually playercount, if less than 4 it'll count the +, but that works for our purpose
	let mut player_count = 1;
	for i in 1..4 {
		let bar_color = image.sub_image(AVATAR_START + (AVATAR_SIZE + AVATAR_SPACING) * i + 1, AVATAR_BAR_Y, AVATAR_BAR_W, AVATAR_BAR_H).average_color();
		let deviation = primary_color.deviation(bar_color);
		// println!("avatar bar color of {i} has a deviation of {deviation} and color of {bar_color:?}");
		if deviation > 5.0 {
			break;
		}
		
		player_count = i + 1;
	}
	
	println!("avatarcount is {player_count}");
	
	(
		AVATAR_START + (AVATAR_SIZE + AVATAR_SPACING) * player_count,
		TEXT_Y,
	)
}

pub fn party_header_text(image: Image, theme: crate::Theme, ocr: &crate::ocr::Ocr) -> String {
	const TEXT_HEIGHT: u32 = 36;
	const TEXT_WIDTH: u32 = 1000;
	
	let (x, y) = party_header_text_start(image);
	image.sub_image(x - 4, y - 4, TEXT_WIDTH + 8, TEXT_HEIGHT + 8).get_text(theme, ocr)
	// let mut image = image.sub_image(x - 2, y - 2, TEXT_WIDTH + 4, TEXT_HEIGHT + 4).to_owned_image();
	// image.resize_h(24);
	// image.as_image().get_text()
}