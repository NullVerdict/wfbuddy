use std::sync::LazyLock;

use regex::Regex;

use crate::{
	image::{Image, Mask, OwnedImage, OwnedMask},
	ocr::Ocr,
	theme::Theme,
	util::DIGIT_REGEX,
};

/// Rarity tiers in the relic reward screen.
/// Warframe commonly refers to these visually as Bronze (Common), Silver (Uncommon), Gold (Rare).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rarity {
	Common,
	Uncommon,
	Rare,
}

impl Rarity {
	#[must_use]
	pub const fn label(self) -> &'static str {
		match self {
			Self::Common => "Bronze",
			Self::Uncommon => "Silver",
			Self::Rare => "Gold",
		}
	}
}

pub struct Reward {
	pub name: String,
	pub owned: u32,
	pub rarity: Rarity,
}

#[derive(Debug, Clone, Copy)]
pub struct RewardArea {
	pub x: u32,
	pub y: u32,
	pub w: u32,
	pub h: u32,
}

pub struct Rewards {
	pub timer: u32,
	/// Heuristic: whether the relic reward screen is likely visible.
	pub present: bool,
	/// Number of reward slots detected (1..=4), even if OCR misses a name.
	pub layout_count: u32,
	/// Rectangle of the reward-card area in *capture image pixel* coordinates.
	pub reward_area: RewardArea,
	pub rewards: Vec<Reward>,
}

static OWNED_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"Owned:\s*([0-9]+)").unwrap());

static ICON_COMMON: LazyLock<(OwnedImage, OwnedMask)> = LazyLock::new(|| {
	OwnedImage::from_png_mask(include_bytes!("../asset/icon_common.png"), 220).unwrap()
});
static ICON_UNCOMMON: LazyLock<(OwnedImage, OwnedMask)> = LazyLock::new(|| {
	OwnedImage::from_png_mask(include_bytes!("../asset/icon_uncommon.png"), 220).unwrap()
});
static ICON_RARE: LazyLock<(OwnedImage, OwnedMask)> = LazyLock::new(|| {
	OwnedImage::from_png_mask(include_bytes!("../asset/icon_rare.png"), 220).unwrap()
});

const REFERENCE_HEIGHT: f32 = 1080.0;
const ICON_BASE_SIZE: u32 = 40;

/// Scale a reference-space pixel measurement into the current image space.
#[inline]
fn px(reference_px: u32, image: Image, ui_scale: f32) -> u32 {
	let scale = (image.height() as f32 / REFERENCE_HEIGHT) * ui_scale;
	let v = (reference_px as f32 * scale).round();
	// Avoid zero-sized sub-images.
	v.max(1.0) as u32
}

#[inline]
fn clamp_sub_image(image: Image, x: i32, y: i32, w: u32, h: u32) -> Image {
	let max_x = image.width().saturating_sub(w) as i32;
	let max_y = image.height().saturating_sub(h) as i32;
	let x = x.clamp(0, max_x) as u32;
	let y = y.clamp(0, max_y) as u32;
	image.sub_image(x, y, w, h)
}

#[derive(Debug, Clone)]
struct RewardLayout {
	count: u32,
	hit_count: u32,
	rarities: Vec<Rarity>,
}

/// Find the best rarity icon match at `(x, y)` in `area`.
///
/// Returns `(rarity, deviation)` where lower deviation is better.
fn best_icon_match(area: Image, x: u32, y: u32, icon_size: u32) -> (Rarity, f32) {
	let icons: [(Rarity, &LazyLock<(OwnedImage, OwnedMask)>); 3] = [
		(Rarity::Common, &ICON_COMMON),
		(Rarity::Uncommon, &ICON_UNCOMMON),
		(Rarity::Rare, &ICON_RARE),
	];

	let mut best = (Rarity::Common, f32::INFINITY);

	// Small jitter helps with off-by-one rounding in scaling and capture.
	for jitter_y in -1..=1 {
		for jitter_x in -1..=1 {
			let sub = clamp_sub_image(area, x as i32 + jitter_x, y as i32 + jitter_y, icon_size, icon_size);

			for (rarity, icon) in icons {
				let icon_img = icon.0.as_image();
				let icon_mask = Mask(&icon.1 .0);

				// Normalize scale: resize the sampled icon region to the base template size.
				let sample = if icon_size == ICON_BASE_SIZE {
					sub.to_owned_image()
				} else {
					sub.to_owned_image().resized_exact(ICON_BASE_SIZE, ICON_BASE_SIZE)
				};

				let deviation = sample.as_image().average_deviation_masked(icon_img, icon_mask);
				if deviation < best.1 {
					best = (rarity, deviation);
				}
			}
		}
	}

	best
}

fn icon_positions_for_count(count: u32, center_x: i32, offset: i32) -> Vec<i32> {
	match count {
		1 => vec![center_x],
		2 => vec![center_x - offset / 2, center_x + offset / 2],
		3 => vec![center_x - offset, center_x, center_x + offset],
		4 => vec![
			center_x - (3 * offset) / 2,
			center_x - offset / 2,
			center_x + offset / 2,
			center_x + (3 * offset) / 2,
		],
		_ => vec![center_x],
	}
}

/// Detect how many reward cards are present, and the rarity tier for each.
///
/// Critical bug fix: the old logic could miss a reward when mixed rarity icons were present
/// (e.g., 3 Bronze + 1 Gold) because it only probed a couple of icon slots and assumed
/// perfect matching. We now evaluate *all* expected icon positions for counts 1..=4 and
/// pick the best match, ensuring all tiers are detected and counted.
fn reward_layout(image: Image, ui_scale: f32) -> RewardLayout {
	const REWARDS_AREA_WIDTH: u32 = 962;
	const RARITY_ICON_OFFSET: u32 = 242;
	const RARITY_ICON_Y: u32 = 459;

	let area_width = px(REWARDS_AREA_WIDTH, image, ui_scale);
	let icon_offset = px(RARITY_ICON_OFFSET, image, ui_scale) as i32;
	let icon_y = px(RARITY_ICON_Y, image, ui_scale);
	let icon_size = px(ICON_BASE_SIZE, image, ui_scale);

	let area = image.trimmed_centerh(area_width);
	let center_x = (area_width as i32 / 2) - (icon_size as i32 / 2);

	// Threshold carried over from the previous implementation.
	const HIT_THRESHOLD: f32 = 25.0;

	let mut best_count = 1;
	let mut best_hits: i32 = -1;
	let mut best_avg_dev: f32 = f32::INFINITY;
	let mut best_rarities: Vec<Rarity> = vec![Rarity::Common];

	for candidate in 1..=4 {
		let mut hits: i32 = 0;
		let mut dev_sum = 0.0;
		let mut rarities = Vec::with_capacity(candidate as usize);

		for x in icon_positions_for_count(candidate, center_x, icon_offset) {
			let (rarity, dev) = best_icon_match(area, x as u32, icon_y, icon_size);
			rarities.push(rarity);

			if dev <= HIT_THRESHOLD {
				hits += 1;
				dev_sum += dev;
			}
		}

		let avg_dev = if hits > 0 { dev_sum / hits as f32 } else { f32::INFINITY };

		// Prefer: more hits, then larger candidate (avoid "dropping" a slot), then lower deviation.
		let better = (hits > best_hits)
			|| (hits == best_hits && candidate > best_count)
			|| (hits == best_hits && candidate == best_count && avg_dev < best_avg_dev);

		if better {
			best_count = candidate;
			best_hits = hits;
			best_avg_dev = avg_dev;
			best_rarities = rarities;
		}
	}

	RewardLayout {
		count: best_count,
		hit_count: best_hits.max(0) as u32,
		rarities: best_rarities,
	}
}


fn reward_area_rect(image: Image, ui_scale: f32, count: u32) -> RewardArea {
	const REWARD_SIZE: u32 = 235;
	const REWARD_SPACING: u32 = 8;
	const REWARD_Y: u32 = 225;

	let reward_size = px(REWARD_SIZE, image, ui_scale);
	let spacing = px(REWARD_SPACING, image, ui_scale);
	let reward_y = px(REWARD_Y, image, ui_scale);

	let mut area_w = count * reward_size + count.saturating_sub(1) * spacing;
	// Match `trimmed_centerh`: clamp to image width and force even.
	area_w = area_w.min(image.width());
	area_w = (area_w >> 1) << 1;

	let x = (image.width().saturating_sub(area_w)) / 2;
	RewardArea { x, y: reward_y, w: area_w, h: reward_size }
}

fn get_reward_subimages(image: Image, ui_scale: f32, count: u32) -> Vec<Image> {
	const REWARD_SIZE: u32 = 235;
	const REWARD_SPACING: u32 = 8;
	const REWARD_Y: u32 = 225;

	let reward_size = px(REWARD_SIZE, image, ui_scale);
	let spacing = px(REWARD_SPACING, image, ui_scale);
	let reward_y = px(REWARD_Y, image, ui_scale);

	let area_width = count * reward_size + count.saturating_sub(1) * spacing;
	let image = image.trimmed_centerh(area_width);

	(0..count)
		.map(|i| {
			let x = (reward_size + spacing) * i;
			image.sub_image(x, reward_y, reward_size, reward_size)
		})
		.collect()
}

pub(crate) fn get_rewards(image: Image, theme: Theme, ocr: &Ocr, ui_scale: f32) -> Rewards {
	const TIMER_W: u32 = 80;
	const TIMER_H: u32 = 45;
	const TIMER_Y: u32 = 150;

	const CRAFTED_AREA_SIZE: u32 = 35;
	const NAME_AREA_SIZE: u32 = 80;

	let timer_w = px(TIMER_W, image, ui_scale);
	let timer_h = px(TIMER_H, image, ui_scale);
	let timer_y = px(TIMER_Y, image, ui_scale);

	let crafted_area = px(CRAFTED_AREA_SIZE, image, ui_scale);
	let name_area = px(NAME_AREA_SIZE, image, ui_scale);

	let timer_text = image
		.trimmed_centerh(timer_w)
		.sub_image(0, timer_y, timer_w, timer_h)
		.get_text(theme, ocr);

	let timer = DIGIT_REGEX
		.find(&timer_text)
		.and_then(|m| m.as_str().parse().ok())
		.unwrap_or(0);

	let layout = reward_layout(image, ui_scale);
	let reward_images = get_reward_subimages(image, ui_scale, layout.count);

	let rewards = reward_images
		.into_iter()
		.zip(layout.rarities)
		.filter_map(|(reward_image, rarity)| {
			let name = reward_image.trimmed_bottom(name_area).get_text(theme, ocr).trim().to_owned();
			if name.is_empty() {
				return None;
			}

			let owned_text = reward_image.trimmed_top(crafted_area).get_text(theme, ocr);
			let owned = OWNED_REGEX
				.captures(&owned_text)
				.and_then(|cap| cap.get(1))
				.and_then(|m| m.as_str().parse().ok())
				.unwrap_or(0);

			Some(Reward { name, owned, rarity })
		})
		.collect();

	let present = layout.hit_count > 0 || timer > 0 || !rewards.is_empty();

	Rewards { timer, present, layout_count: layout.count, reward_area: reward_area_rect(image, ui_scale, layout.count), rewards }
}

pub(crate) fn get_selected(image: Image, theme: Theme, ui_scale: f32) -> u32 {
	// Size of the highlight patch in the top-right of the selected card.
	const SELECTED_SIZE: u32 = 12;

	let layout = reward_layout(image, ui_scale);
	let reward_images = get_reward_subimages(image, ui_scale, layout.count);

	let size = px(SELECTED_SIZE, image, ui_scale);

	let mut best_idx = 0;
	let mut best_dev = f32::INFINITY;

	for (idx, reward_image) in reward_images.into_iter().enumerate() {
		let x = reward_image.width().saturating_sub(size);
		let patch = reward_image.sub_image(x, 0, size, size);
		let dev = patch.average_deviation(theme.secondary);
		if dev < best_dev {
			best_dev = dev;
			best_idx = idx as u32;
		}
	}

	best_idx
}
