use std::sync::LazyLock;

use crate::{Image, Mask, OwnedImage, OwnedMask, Theme};

pub struct Rewards {
	pub timer: u32,
	pub rewards: Vec<Reward>,
}

pub struct Reward {
	pub name: String,
	pub owned: u32,
}

/// Expects an image with a height of 1080
pub(crate) fn get_rewards(image: Image, theme: Theme, ocr: &crate::ocr::Ocr) -> Rewards {
	const CRAFTED_AREA_SIZE: u32 = 32;
	const NAME_AREA_SIZE: u32 = 70;
	const TIMER_Y: u32 = 135;
	const TIMER_W: u32 = 64;
	const TIMER_H: u32 = 64;

	static OWNED_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
		regex::Regex::new(r"(?<amount>\d+)?\s*(?:Owned|Crafted)").unwrap()
	});

	let rewards = get_reward_subimages(image)
		.into_iter()
		.map(|image| {
			let name = image.trimmed_bottom(NAME_AREA_SIZE).get_text(theme, ocr);

			let owned_text = image.trimmed_top(CRAFTED_AREA_SIZE).get_text(theme, ocr);
			let owned = OWNED_REGEX
				.captures(&owned_text)
				.map(|cap| {
					cap.name("amount")
						.and_then(|m| m.as_str().parse::<u32>().ok())
						.unwrap_or(1)
				})
				.unwrap_or(0);

			Reward { name, owned }
		})
		.collect();

	let timer_text = image
		.trimmed_centerh(TIMER_W)
		.sub_image(0, TIMER_Y, TIMER_W, TIMER_H)
		.get_text(Theme::WHITE, ocr);

	let timer = crate::util::DIGIT_REGEX
		.captures(&timer_text)
		.and_then(|cap| cap.name("digits"))
		.and_then(|m| m.as_str().parse::<u32>().ok())
		.unwrap_or(10);

	Rewards { timer, rewards }
}

/// Expects an image with a height of 1080
pub(crate) fn get_selected(image: Image, theme: Theme) -> u32 {
	let rewards = get_reward_subimages(image);

	let mut picked = 0;
	let mut best_dev = f32::INFINITY;

	for (i, image) in rewards.iter().enumerate() {
		let dev = image
			.sub_image(image.width() - 12, 0, 12, 12)
			.average_color()
			.deviation(theme.secondary);

		if dev < best_dev {
			picked = i as u32;
			best_dev = dev;
		}
	}

	picked
}

fn get_reward_subimages<'a>(image: Image<'a>) -> Vec<Image<'a>> {
	const REWARD_SIZE: u32 = 235;
	const REWARD_SPACING: u32 = 8;
	const REWARD_Y: u32 = 225;

	let count = reward_count(image).max(1);

	let area_width = count * REWARD_SIZE + (count - 1) * REWARD_SPACING;
	let image = image.trimmed_centerh(area_width);

	let mut images = Vec::with_capacity(count as usize);
	for i in 0..count {
		let offset = (REWARD_SIZE + REWARD_SPACING) * i;
		images.push(image.sub_image(offset, REWARD_Y, REWARD_SIZE, REWARD_SIZE));
	}

	images
}

static ICON_COMMON: LazyLock<(OwnedImage, OwnedMask)> = LazyLock::new(|| {
	crate::OwnedImage::from_png_mask(include_bytes!("../asset/icon_common.png"), 250).unwrap()
});
static ICON_UNCOMMON: LazyLock<(OwnedImage, OwnedMask)> = LazyLock::new(|| {
	crate::OwnedImage::from_png_mask(include_bytes!("../asset/icon_uncommon.png"), 250).unwrap()
});
static ICON_RARE: LazyLock<(OwnedImage, OwnedMask)> = LazyLock::new(|| {
	crate::OwnedImage::from_png_mask(include_bytes!("../asset/icon_rare.png"), 250).unwrap()
});

/// Gets the amount of reward cards shown.
///
/// This is **not always equal** to the party size if someone forgot to select a relic.
///
/// Bugfix note:
/// The previous implementation only probed one or two icon positions and could miss the
/// last card when rarities were mixed (e.g. 3 common + 1 rare). We now evaluate all
/// expected slots for 1–4 cards and pick the layout with the best match coverage.
fn reward_count(image: Image) -> u32 {
	// The full reward strip when 4 cards are shown is ~964 px wide at 1080p.
	// The capture is trimmed to a slightly smaller, stable width.
	const REWARDS_AREA_WIDTH: u32 = 962;

	// Card geometry (must match `get_reward_subimages`).
	const REWARD_SIZE: u32 = 235;
	const REWARD_SPACING: u32 = 8;

	// Rarity icon geometry inside the reward strip.
	const RARITY_ICON_Y: u32 = 459;
	const RARITY_ICON_SIZE: u32 = 40;

	// Template-matching settings.
	const JITTER: i32 = 1;
	// Luma-only deviation makes this far less sensitive to UI theme tinting.
	const LUMA_DEVIATION_THRESHOLD: f32 = 0.10;

	let image = image.trimmed_centerh(REWARDS_AREA_WIDTH);

	#[inline]
	fn luma(c: crate::Color) -> f32 {
		// Rec. 709 luma in 0..1
		(0.2126 * c.r as f32 + 0.7152 * c.g as f32 + 0.0722 * c.b as f32) / 255.0
	}

	fn best_luma_deviation(sub: Image) -> f32 {
		let mut best = f32::INFINITY;

		for (icon_img, icon_mask) in [&*ICON_COMMON, &*ICON_UNCOMMON, &*ICON_RARE] {
			let templ = icon_img.as_image();
			let mask = Mask(&icon_mask.0);

			let mut sum = 0.0f32;
			let mut count = 0u32;

			let w = templ.width().min(sub.width());
			let h = templ.height().min(sub.height());
			for y in 0..h {
				for x in 0..w {
					let i = (x + y * templ.width()) as usize;
					let yes = ((mask.0[i / 8] >> (i % 8)) & 1) == 1;
					if !yes {
						continue;
					}
					let a = luma(sub.pixel_rel(x, y));
					let b = luma(templ.pixel_rel(x, y));
					sum += (a - b).abs();
					count += 1;
				}
			}

			if count > 0 {
				best = best.min(sum / count as f32);
			}
		}

		best
	}

	fn icon_present_at(image: Image, x_left: i32) -> bool {
		let x_left = x_left.max(0) as u32;

		if x_left + RARITY_ICON_SIZE > image.width() {
			return false;
		}
		if RARITY_ICON_Y + RARITY_ICON_SIZE > image.height() {
			return false;
		}

		for dy in -JITTER..=JITTER {
			for dx in -JITTER..=JITTER {
				let x = (x_left as i32 + dx).clamp(0, (image.width() - RARITY_ICON_SIZE) as i32) as u32;
				let y = (RARITY_ICON_Y as i32 + dy).clamp(0, (image.height() - RARITY_ICON_SIZE) as i32) as u32;

				let sub = image.sub_image(x, y, RARITY_ICON_SIZE, RARITY_ICON_SIZE);
				if best_luma_deviation(sub) <= LUMA_DEVIATION_THRESHOLD {
					return true;
				}
			}
		}

		false
	}

	#[derive(Clone, Copy)]
	struct Candidate {
		count: u32,
		matches: u32,
		avg_dev: f32,
	}

	fn evaluate(image: Image, count: u32) -> Candidate {
		let total_width = count * REWARD_SIZE + count.saturating_sub(1) * REWARD_SPACING;
		let margin = (REWARDS_AREA_WIDTH as i32 - total_width as i32) / 2;

		let mut matches = 0u32;
		let mut dev_sum = 0.0f32;

		for i in 0..count {
			let reward_left = margin + (i * (REWARD_SIZE + REWARD_SPACING)) as i32;
			let icon_center = reward_left + (REWARD_SIZE as i32 / 2);
			let x_left = icon_center - (RARITY_ICON_SIZE as i32 / 2);

			// Track deviation for tie-breaking even if not present.
			let x_left_u = x_left.max(0) as u32;
			let x_left_u = x_left_u.min(image.width().saturating_sub(RARITY_ICON_SIZE));
			let sub = image.sub_image(x_left_u, RARITY_ICON_Y, RARITY_ICON_SIZE, RARITY_ICON_SIZE);
			let dev = best_luma_deviation(sub);
			dev_sum += dev;

			if icon_present_at(image, x_left) {
				matches += 1;
			}
		}

		Candidate {
			count,
			matches,
			avg_dev: dev_sum / count.max(1) as f32,
		}
	}

	let mut best = Candidate {
		count: 1,
		matches: 0,
		avg_dev: f32::INFINITY,
	};

	for count in 1..=4 {
		let c = evaluate(image, count);

		let best_is_perfect = best.matches == best.count;
		let c_is_perfect = c.matches == c.count;

		best = match (best_is_perfect, c_is_perfect) {
			(false, true) => c,
			(true, false) => best,
			_ => {
				// Both perfect OR both imperfect: choose higher match coverage, then lower deviation.
				if (c.matches, -c.avg_dev) > (best.matches, -best.avg_dev) {
					c
				} else {
					best
				}
			}
		};
	}

	// If the best candidate isn't perfect, fall back to "how many icons we actually saw".
	if best.matches < best.count {
		best.matches.max(1)
	} else {
		best.count
	}
}
