use crate::{Image, Theme};

pub const DIGIT_REGEX: std::sync::LazyLock<regex::Regex> =
	std::sync::LazyLock::new(|| regex::Regex::new(r"(?<digits>\d+)").unwrap());

const BASE_HEIGHT: f32 = 1080.0;

#[inline]
fn scale_factor(image: Image, ui_scale: f32) -> f32 {
	(image.height() as f32 / BASE_HEIGHT) * ui_scale
}

#[inline]
fn px(base: u32, s: f32) -> u32 {
	if base == 0 {
		0
	} else {
		((base as f32) * s).round().max(1.0) as u32
	}
}

pub fn party_header_text_start_scaled(image: Image, ui_scale: f32) -> (u32, u32) {
	// Reference layout is for a 1080p capture at UI scale 100%.
	let s = scale_factor(image, ui_scale);

	let avatar_start = px(96, s);
	let avatar_size = px(44, s);
	let avatar_spacing = px(4, s);

	let avatar_bar_y = px(86, s);
	let avatar_bar_w = px(40, s);
	let avatar_bar_h = px(2, s);

	let text_y = px(49, s);
	let one = px(1, s);

	let primary_color = image
		.sub_image(avatar_start + one, avatar_bar_y, avatar_bar_w, avatar_bar_h)
		.average_color();

	// Not actually playercount; if less than 4 it'll count the +, but that works for our purpose.
	let mut player_count = 1;
	for i in 1..4 {
		let x = avatar_start + (avatar_size + avatar_spacing) * i + one;
		let bar_color = image
			.sub_image(x, avatar_bar_y, avatar_bar_w, avatar_bar_h)
			.average_color();
		let deviation = primary_color.deviation(bar_color);
		if deviation > 5.0 {
			break;
		}

		player_count = i + 1;
	}

	(
		avatar_start + (avatar_size + avatar_spacing) * player_count,
		text_y,
	)
}

pub fn party_header_text_start(image: Image) -> (u32, u32) {
	party_header_text_start_scaled(image, 1.0)
}

pub fn party_header_text_scaled(
	image: Image,
	theme: Theme,
	ocr: &crate::ocr::Ocr,
	ui_scale: f32,
) -> String {
	let s = scale_factor(image, ui_scale);

	let text_h = px(36, s);
	let text_w = px(1000, s);
	let pad = px(4, s);

	let (x, y) = party_header_text_start_scaled(image, ui_scale);

	image
		.sub_image(x.saturating_sub(pad), y.saturating_sub(pad), text_w + pad * 2, text_h + pad * 2)
		.get_text(theme, ocr)
}

pub fn party_header_text(image: Image, theme: Theme, ocr: &crate::ocr::Ocr) -> String {
	party_header_text_scaled(image, theme, ocr, 1.0)
}
