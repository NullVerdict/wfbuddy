use std::sync::LazyLock;
use crate::{Image, Mask, OwnedImage, OwnedMask, Theme};


static OWNED_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?<amount>\d+)?\s*(?:Owned|Crafted)").unwrap());

pub struct Rewards {
	pub timer: u32,
	pub rewards: Vec<Reward>,
}

pub struct Reward {
	pub name: String,
	pub owned: u32,
}

/// Expects an image with a height of 1080
pub(crate) fn get_rewards(image: Image, _theme: Theme, ocr: &crate::ocr::Ocr) -> Rewards {
	const CRAFTED_AREA_SIZE: u32 = 32;
	const NAME_AREA_SIZE: u32 = 96;
	const TIMER_Y: u32 = 135;
	const TIMER_W: u32 = 64;
	const TIMER_H: u32 = 64;
	
	let rewards = get_reward_subimages(image)
		.into_iter()
		.map(|image| Reward {
			name: image.trimmed_bottom(NAME_AREA_SIZE).get_text(Theme::WHITE, ocr),
			owned: OWNED_REGEX
					.captures(&image.trimmed_top(CRAFTED_AREA_SIZE).get_text(Theme::WHITE, ocr))
					.map(|caps| {
						caps.name("amount")
							.and_then(|m| m.as_str().parse::<u32>().ok())
							.unwrap_or(1)
					})
					.unwrap_or(0),
		})
		.collect();
	
	let timer = crate::util::DIGIT_REGEX
		.captures(&image
			.trimmed_centerh(TIMER_W)
			.sub_image(0, TIMER_Y, TIMER_W, TIMER_H)
			.get_text(Theme::WHITE, ocr))
		.and_then(|caps| caps
			.name("digits")
			.and_then(|m| m.as_str().parse::<u32>().ok()))
		.unwrap_or(10);
	
	Rewards{timer, rewards}
}

/// Expects an image with a height of 1080
pub(crate) fn get_selected(image: Image, theme: Theme) -> u32 {
	let rewards = get_reward_subimages(image);
	
	// Pick the reward whose corner highlight is closest to the theme secondary color.
	// `Color::deviation` can be much larger than 1.0, so start with INFINITY.
	let mut picked = 0;
	let mut best = f32::INFINITY;
	for (i, image) in rewards.iter().enumerate() {
		let dev = image.sub_image(image.width() - 12, 0, 12, 12).average_color().deviation(theme.secondary);
		log::debug!("pick check dev {dev}");
		if dev < best {
			picked = i as u32;
			best = dev;
		}
	}
	
	picked
}

fn get_reward_subimages<'a>(image: Image<'a>) -> Vec<Image<'a>> {
	const REWARD_SIZE: u32 = 235;
	const REWARD_SPACING: u32 = 8;
	const REWARD_Y: u32 = 225;
	
	let count = reward_count(image);
	log::debug!("rewardcount is {count}");
	
	let area_width = count * REWARD_SIZE + (count - 1) * REWARD_SPACING;
	let image = image.trimmed_centerh(area_width);
	
	let mut images = Vec::with_capacity(count as usize);
	for i in 0..count {
		let offset = (REWARD_SIZE + REWARD_SPACING) * i;
		images.push(image.sub_image(offset, REWARD_Y, REWARD_SIZE, REWARD_SIZE));
	}
	
	images
}

static ICON_COMMON: LazyLock<(OwnedImage, OwnedMask)> = LazyLock::new(|| {crate::OwnedImage::from_png_mask(include_bytes!("../asset/icon_common.png"), 250).unwrap()});
static ICON_UNCOMMON: LazyLock<(OwnedImage, OwnedMask)> = LazyLock::new(|| {crate::OwnedImage::from_png_mask(include_bytes!("../asset/icon_uncommon.png"), 250).unwrap()});
static ICON_RARE: LazyLock<(OwnedImage, OwnedMask)> = LazyLock::new(|| {crate::OwnedImage::from_png_mask(include_bytes!("../asset/icon_rare.png"), 250).unwrap()});

/// Cheap "are we on the rewards screen" check.
///
/// This avoids OCR by template-matching the rarity icons strip.
///
/// Expects an image with a height of 1080.
pub(crate) fn is_screen(image: Image, _theme: Theme) -> bool {
	const REWARDS_AREA_WIDTH: u32 = 962;
	const RARITY_ICON_Y: u32 = 459;
	const RARITY_ICON_SIZE: u32 = 40;
	const ICON_SCAN_THRESHOLD: f32 = 0.24;

	let image = image.trimmed_centerh(REWARDS_AREA_WIDTH);
	let max_x = image.width().saturating_sub(RARITY_ICON_SIZE);
	for x in (0..=max_x).step_by(8) {
		let sub = image.sub_image(x, RARITY_ICON_Y, RARITY_ICON_SIZE, RARITY_ICON_SIZE);
		let mut best = f32::INFINITY;
		for icon in [&ICON_COMMON, &ICON_UNCOMMON, &ICON_RARE] {
			let dev = sub.average_deviation_masked(icon.0.as_image(), Mask(&icon.1.0));
			best = best.min(dev);
		}
		if best < ICON_SCAN_THRESHOLD {
			return true;
		}
	}
	false
}

// Gets the amount of rewards there are, not always equal to the amount of people
// in the party if someone forgot to select a relic.
fn reward_count(image: Image) -> u32 {
	const REWARDS_AREA_WIDTH: u32 = 962;
	const RARITY_ICON_Y: u32 = 459;
	const RARITY_ICON_SIZE: u32 = 40;
	const REWARD_SIZE: u32 = 235;
	const REWARD_SPACING: u32 = 8;
	const PITCH: f32 = (REWARD_SIZE + REWARD_SPACING) as f32; // ~243px
	// Threshold for icon template matching.
	// Lower = stricter. With luminance-distance matching we can tolerate more
	// in-game color variation (notably the gold "rare" icon).
	const ICON_MATCH_THRESHOLD: f32 = 0.20;
	// Used for the horizontal scan fallback.
	const ICON_SCAN_THRESHOLD: f32 = 0.24;
	
	let image = image.trimmed_centerh(REWARDS_AREA_WIDTH);

	fn best_icon_match(image: Image, x: u32) -> f32 {
		let mut best = f32::INFINITY;
		for icon in [&ICON_COMMON, &ICON_UNCOMMON, &ICON_RARE] {
			for jitter_y in -1..=1 { // sub-pixel shenanigans
				for jitter_x in -1..=1 {
					let sx = x as isize + jitter_x;
					let sy = RARITY_ICON_Y as isize + jitter_y;
					if sx < 0 || sy < 0 {
						continue;
					}
					let sx = sx as u32;
					let sy = sy as u32;
					if sx + RARITY_ICON_SIZE > image.width() || sy + RARITY_ICON_SIZE > image.height() {
						continue;
					}
					let sub = image.sub_image(sx, sy, RARITY_ICON_SIZE, RARITY_ICON_SIZE);
					let dev = sub.average_deviation_masked(icon.0.as_image(), Mask(&icon.1.0));
					best = best.min(dev);
				}
			}
		}
		best
	}

	// Primary approach: try each possible reward count (1..=4), score by sum of
	// best icon matches. Rewards are centered, so positions are computed relative
	// to the area center.
	let center_x = REWARDS_AREA_WIDTH as f32 / 2.0;
	let half_icon = RARITY_ICON_SIZE as f32 / 2.0;

	let mut best_n = 2u32;
	let mut best_score = f32::INFINITY;
	for n in 1u32..=4 {
		let mut score = 0.0;
		let mut ok = true;
		for i in 0u32..n {
			let offset = (i as f32 - (n as f32 - 1.0) / 2.0) * PITCH;
			let x = (center_x + offset - half_icon).round();
			if x < 0.0 {
				ok = false;
				break;
			}
			let dev = best_icon_match(image, x as u32);
			log::debug!("icon best deviation (n={n} i={i}) was {dev}");
			score += dev;
			if dev > ICON_MATCH_THRESHOLD {
				ok = false;
			}
		}
		if ok && score < best_score {
			best_score = score;
			best_n = n;
		}
	}

	// Fallback: if one icon (commonly the gold/rare one) doesn't match well enough
	// at the expected location, the above can under-count (e.g., 3 common + 1 rare
	// being detected as 3). To avoid dropping a reward card, do a horizontal scan
	// across the icon strip and count how many distinct icon matches we see.
	let scanned = {
		let mut hits: Vec<u32> = Vec::new();
		let max_x = image.width().saturating_sub(RARITY_ICON_SIZE);
		let step = 2usize;
		for x in (0..=max_x).step_by(step) {
			let dev = best_icon_match(image, x);
			if dev < ICON_SCAN_THRESHOLD {
				hits.push(x);
			}
		}
		if hits.is_empty() {
			0
		} else {
			hits.sort_unstable();
			// Cluster adjacent hits into distinct icon locations.
			let mut clusters = 0u32;
			let mut last = None::<u32>;
			for x in hits {
				match last {
					None => {
						clusters += 1;
						last = Some(x);
					}
					Some(prev) => {
						if x.saturating_sub(prev) > 14 {
							clusters += 1;
						}
						last = Some(x);
					}
				}
			}
			clusters.clamp(1, 4)
		}
	};
	best_n.max(scanned).clamp(1, 4)
}