//! Relic reward screen detection.
//!
//! Goal: be robust to different screen resolutions and avoid hard-coded pixel
//! coordinates by using relative ROIs and contour-based segmentation.
//!
//! The logic here is intentionally conservative: if we cannot confidently detect
//! slots, we return an empty result instead of panicking.

use regex::Regex;

use crate::{Color, Image, Theme};

#[derive(Debug, Clone)]
pub struct Rewards {
    pub timer: u32,
    pub rewards: Vec<RelicReward>,
}

#[derive(Debug, Clone)]
pub struct RelicReward {
    pub name: String,
    pub owned: u32,
}

/// Axis-aligned rectangle in image coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Rect {
    fn right(&self) -> u32 {
        self.x + self.w
    }
    fn bottom(&self) -> u32 {
        self.y + self.h
    }
    fn center_x(&self) -> u32 {
        self.x + self.w / 2
    }
    fn center_y(&self) -> u32 {
        self.y + self.h / 2
    }

    fn iou(&self, other: &Rect) -> f32 {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());

        if x2 <= x1 || y2 <= y1 {
            return 0.0;
        }

        let inter = (x2 - x1) as f32 * (y2 - y1) as f32;
        let a1 = (self.w as f32) * (self.h as f32);
        let a2 = (other.w as f32) * (other.h as f32);
        inter / (a1 + a2 - inter)
    }
}

pub fn get_rewards(image: Image, theme: Theme, ocr: &crate::ocr::Ocr) -> Rewards {
    let slots = detect_reward_slots(image);
    if slots.is_empty() {
        return Rewards {
            timer: 0,
            rewards: vec![],
        };
    }

    let timer = detect_timer(image, &slots, theme, ocr);

    let rewards = slots
        .iter()
        .map(|slot| parse_reward(image, *slot, theme, ocr))
        .collect();

    Rewards { timer, rewards }
}

pub fn get_selected(image: Image, theme: Theme) -> Option<usize> {
    let slots = detect_reward_slots(image);
    if slots.is_empty() {
        return None;
    }

    // Selected highlight is a small square near the top-right of a slot.
    // We compare it to the sampled theme secondary color.
    let mut best: Option<(usize, f32)> = None;

    for (i, slot) in slots.iter().enumerate() {
        let size = ((slot.w as f32) * 12.0 / 235.0).round().max(6.0) as u32;
        let pad_r = ((slot.w as f32) * 5.0 / 235.0).round().max(1.0) as u32;
        let pad_t = ((slot.h as f32) * 4.0 / 235.0).round().max(1.0) as u32;

        let x = slot.x.saturating_add(slot.w.saturating_sub(size + pad_r));
        let y = slot.y.saturating_add(pad_t);

        let sw = size.min(image.width().saturating_sub(x));
        let sh = size.min(image.height().saturating_sub(y));
        if sw == 0 || sh == 0 {
            continue;
        }

        let avg = image.sub_image(x, y, sw, sh).average_color();
        let dev = avg.deviation(theme.secondary);

        match best {
            None => best = Some((i, dev)),
            Some((_, best_dev)) if dev < best_dev => best = Some((i, dev)),
            _ => {}
        }
    }

    // Threshold is intentionally loose; false positives are filtered by comparing
    // the winner to the runner-up if necessary.
    best.and_then(|(idx, dev)| if dev < 12.0 { Some(idx) } else { None })
}

fn parse_reward(image: Image, slot: Rect, theme: Theme, ocr: &crate::ocr::Ocr) -> RelicReward {
    let slot_img = image.sub_image(slot.x, slot.y, slot.w, slot.h);

    let margin = ((slot.w as f32) * 0.05).round().max(1.0) as u32;

    // Name is typically at the bottom of the slot.
    let name_h = ((slot.h as f32) * 0.30).round().max(12.0) as u32;
    let name_y = slot.h.saturating_sub(name_h);
    let name_w = slot.w.saturating_sub(margin * 2).max(1);
    let name_img = slot_img.sub_image(margin, name_y, name_w, name_h);

    let mut name = name_img.get_text(theme, ocr);
    name = normalize_name(&name);

    // Owned/crafted count is often near the top of the slot.
    let owned_h = ((slot.h as f32) * 0.14).round().max(10.0) as u32;
    let owned_img = slot_img.sub_image(margin, 0, name_w, owned_h);
    let owned_text = owned_img.get_text(theme, ocr);

    let owned = parse_owned_count(&owned_text).unwrap_or(0);

    RelicReward { name, owned }
}

fn normalize_name(raw: &str) -> String {
    raw.replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn parse_owned_count(text: &str) -> Option<u32> {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?i)\b(?:OWNED|CRAFTED)\s*x?\s*(\d+)").expect("regex")
    });

    RE.captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

fn detect_timer(image: Image, slots: &[Rect], theme: Theme, ocr: &crate::ocr::Ocr) -> u32 {
    let avg_h = (slots.iter().map(|r| r.h as u64).sum::<u64>() / slots.len().max(1) as u64) as u32;
    let top_y = slots.iter().map(|r| r.y).min().unwrap_or(0);

    let timer_size = ((avg_h as f32) * 64.0 / 235.0).round().max(16.0) as u32;
    let timer_offset = ((avg_h as f32) * 90.0 / 235.0).round().max(10.0) as u32;

    let center_x = (slots.iter().map(|r| r.center_x() as u64).sum::<u64>() / slots.len().max(1) as u64) as u32;

    let x = center_x.saturating_sub(timer_size / 2);
    let y = top_y.saturating_sub(timer_offset);

    let w = timer_size.min(image.width().saturating_sub(x));
    let h = timer_size.min(image.height().saturating_sub(y));
    if w == 0 || h == 0 {
        return 0;
    }

    let timer_img = image.sub_image(x, y, w, h);
    let text = timer_img.get_text(theme, ocr);

    // Extract the first number we can find.
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<u32>().unwrap_or(0)
}

fn detect_reward_slots(image: Image) -> Vec<Rect> {
    use imageproc::contrast::{equalize_histogram, otsu_level, threshold, ThresholdType};
    use imageproc::contours::{find_contours, BorderType};

    let w = image.width();
    let h = image.height();
    if w == 0 || h == 0 {
        return vec![];
    }

    // Restrict to a broad ROI around the expected rewards area (relative coordinates).
    // This is *not* a fixed-pixel approach: it scales with resolution.
    let roi_x1 = (w as f32 * 0.15).round() as u32;
    let roi_x2 = (w as f32 * 0.85).round() as u32;
    let roi_y1 = (h as f32 * 0.18).round() as u32;
    let roi_y2 = (h as f32 * 0.75).round() as u32;

    let roi_w = roi_x2.saturating_sub(roi_x1).max(1);
    let roi_h = roi_y2.saturating_sub(roi_y1).max(1);

    let roi = image.sub_image(roi_x1, roi_y1, roi_w, roi_h).to_owned_image();
    let gray = equalize_histogram(&roi.to_gray_image());
    let level = otsu_level(&gray);
    let mut bin = threshold(&gray, level, ThresholdType::Binary);

    // Some UI themes produce inverted results; normalize so the background tends to be light.
    normalize_binary(&mut bin);

    // Find contours on the binarized ROI.
    let contours = find_contours::<i32>(&bin);

    let min_side = (h as f32 * 0.12) as u32;
    let max_side = (h as f32 * 0.40) as u32;

    let mut rects = Vec::new();
    for c in contours {
        if c.border_type != BorderType::Outer {
            continue;
        }

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for p in &c.points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }

        if min_x < 0 || min_y < 0 {
            continue;
        }

        let rw = (max_x - min_x + 1).max(0) as u32;
        let rh = (max_y - min_y + 1).max(0) as u32;
        if rw == 0 || rh == 0 {
            continue;
        }

        // Filter by approximate size and aspect ratio (reward slots are close to square).
        if rw < min_side || rh < min_side || rw > max_side || rh > max_side {
            continue;
        }
        let aspect = rw as f32 / rh as f32;
        if !(0.80..=1.25).contains(&aspect) {
            continue;
        }

        rects.push(Rect {
            x: roi_x1 + (min_x as u32),
            y: roi_y1 + (min_y as u32),
            w: rw,
            h: rh,
        });
    }

    if rects.is_empty() {
        return rects;
    }

    // Group candidates by approximate row (y coordinate) and keep the row with the most slots.
    let tol = (h as f32 * 0.06).round().max(1.0) as u32;

    use std::collections::HashMap;
    let mut buckets: HashMap<u32, Vec<Rect>> = HashMap::new();
    for r in rects {
        let key = r.center_y() / tol;
        buckets.entry(key).or_default().push(r);
    }

    let mut best_row = buckets
        .into_values()
        .max_by_key(|v| v.len())
        .unwrap_or_default();

    // Sort left-to-right and deduplicate heavy overlaps.
    best_row.sort_by_key(|r| r.x);
    let mut dedup = Vec::new();
    for r in best_row {
        if let Some(prev) = dedup.last_mut() {
            if prev.iou(&r) > 0.5 {
                // Keep the larger rect.
                let prev_area = (prev.w as u64) * (prev.h as u64);
                let r_area = (r.w as u64) * (r.h as u64);
                if r_area > prev_area {
                    *prev = r;
                }
                continue;
            }
        }
        dedup.push(r);
    }

    dedup
}

fn normalize_binary(bin: &mut image::GrayImage) {
    // Decide whether to invert the thresholded image based on white/black ratio.
    let mut white = 0u64;
    let mut black = 0u64;
    for p in bin.pixels() {
        if p.0[0] > 0 {
            white += 1;
        } else {
            black += 1;
        }
    }
    if black > white {
        for p in bin.pixels_mut() {
            p.0[0] = 255u8.saturating_sub(p.0[0]);
        }
    }
}
