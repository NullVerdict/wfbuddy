//! Image primitives and utilities.
//!
//! The project uses a lightweight owned RGB image type (`OwnedImage`) that is
//! optimized for repeated cropping/resizing of screen captures.
//!
//! For many operations we borrow a view (`Image<'a>`) instead of copying pixels.
//! This keeps UI/CV pipelines fast while still allowing easy conversion to owned
//! images when needed (OCR preprocessing, debug snapshots, etc.).

use anyhow::{Context, Result};

pub struct OwnedMask(pub Vec<u8>);
pub struct Mask<'a>(pub &'a [u8]);

/// Owned RGB image (no alpha).
#[derive(Clone, Debug)]
pub struct OwnedImage {
    width: u32,
    height: u32,
    data: Vec<Color>,
}

impl OwnedImage {
    /// Build an `OwnedImage` from RGBA bytes (alpha is discarded).
    ///
    /// The buffer is expected to be tightly packed: `width * height * 4` bytes.
    pub fn from_rgba(width: usize, bytes: &[u8]) -> Self {
        let height = bytes.len() as usize / width / 4;
        let data = bytes
            .chunks_exact(4)
            .map(|v| Color::new(v[0], v[1], v[2]))
            .collect::<Vec<_>>();

        Self {
            width: width as u32,
            height: height as u32,
            data,
        }
    }

    /// Load an RGBA PNG and return an `(OwnedImage, OwnedMask)` pair.
    ///
    /// The mask is a packed bitset (row-major) where each bit indicates whether
    /// the original alpha value was >= `alpha_threshold`.
    pub fn from_png_mask(bytes: &[u8], alpha_threshold: u8) -> Result<(Self, OwnedMask)> {
        let img = image::load_from_memory(bytes)
            .context("decode png (with alpha)")?
            .to_rgba8();
        let (width, height) = img.dimensions();
        let mut data = Vec::with_capacity((width * height) as usize);
        let mut mask = vec![0u8; (width * height) as usize / 8 + 1];

        for (i, p) in img.pixels().enumerate() {
            let [r, g, b, a] = p.0;
            data.push(Color::new(r, g, b));
            if a >= alpha_threshold {
                mask[i / 8] |= 1 << (i % 8);
            }
        }

        Ok((
            Self {
                width,
                height,
                data,
            },
            OwnedMask(mask),
        ))
    }

    /// Resize this image to the given height (preserving aspect ratio).
    ///
    /// Uses `fast_image_resize` (SIMD-optimized) and keeps output in `Vec<Color>`.
    pub fn resize_h(&mut self, height: u32) {
        if self.height == height {
            return;
        }

        let height = height.max(1);
        let width = (self.width as u64 * height as u64 / self.height.max(1) as u64) as u32;

        // SAFETY: `Color` is `#[repr(C)]` with 3 x `u8`, so it is layout-compatible
        // with `fast_image_resize::pixels::U8x3` (alignment 1).
        let src_pixels = unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr() as *const fast_image_resize::pixels::U8x3,
                self.data.len(),
            )
        };

        let src = fast_image_resize::images::ImageRef::from_pixels(self.width, self.height, src_pixels)
            .expect("fast_image_resize: ImageRef::from_pixels failed");

        let mut dst = fast_image_resize::images::Image::new(width, height, fast_image_resize::PixelType::U8x3);

        let mut resizer = fast_image_resize::Resizer::new();
        let options = fast_image_resize::ResizeOptions::new().resize_alg(
            fast_image_resize::ResizeAlg::Interpolation(fast_image_resize::FilterType::CatmullRom),
        );

        resizer
            .resize(&src, &mut dst, &Some(options))
            .expect("fast_image_resize: resize failed");

        let bytes: Vec<u8> = dst.into_vec();
        let mut data = Vec::with_capacity((width * height) as usize);
        for px in bytes.chunks_exact(3) {
            data.push(Color::new(px[0], px[1], px[2]));
        }

        self.width = width;
        self.height = height;
        self.data = data;
    }

    #[inline]
    pub fn resized_h(mut self, height: u32) -> Self {
        self.resize_h(height);
        self
    }

    pub fn map_pixels(&mut self, f: impl Fn(&mut Color)) {
        for v in &mut self.data {
            f(v);
        }
    }

    /// Create a borrowed view of this entire image.
    pub fn as_image<'a>(&'a self) -> Image<'a> {
        Image {
            x1: 0,
            y1: 0,
            x2: self.width,
            y2: self.height,
            true_width: self.width,
            data: &self.data,
        }
    }

    /// Convert to a grayscale `GrayImage` (luma).
    pub fn to_gray_image(&self) -> image::GrayImage {
        use image::{GrayImage, Luma};
        let mut out = GrayImage::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let c = self.data[(x + y * self.width) as usize];
                out.put_pixel(x, y, Luma([c.luma()]));
            }
        }
        out
    }

    /// Create an RGB `OwnedImage` from a grayscale image (each pixel repeated into RGB).
    pub fn from_gray_as_rgb(gray: &image::GrayImage) -> Self {
        let (w, h) = gray.dimensions();
        let mut data = Vec::with_capacity((w * h) as usize);
        for p in gray.pixels() {
            let v = p.0[0];
            data.push(Color::new(v, v, v));
        }
        Self {
            width: w,
            height: h,
            data,
        }
    }
}

// ----------

/// Borrowed image view into an `OwnedImage`.
#[derive(Clone, Copy)]
pub struct Image<'a> {
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    true_width: u32,
    data: &'a [Color],
}

impl<'a> Image<'a> {
    #[inline(always)]
    pub fn width(&self) -> u32 {
        self.x2 - self.x1
    }

    #[inline(always)]
    pub fn height(&self) -> u32 {
        self.y2 - self.y1
    }

    #[inline(always)]
    fn pixel(&self, x: u32, y: u32) -> &Color {
        &self.data[(x + y * self.true_width) as usize]
    }

    pub fn to_owned_image(self) -> OwnedImage {
        let mut data = Vec::with_capacity((self.width() * self.height()) as usize);
        for y in self.y1..self.y2 {
            for x in self.x1..self.x2 {
                data.push(*self.pixel(x, y));
            }
        }

        OwnedImage {
            width: self.width(),
            height: self.height(),
            data,
        }
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0; (self.width() * self.height() * 3) as usize];
        let mut i = 0;
        for y in self.y1..self.y2 {
            for x in self.x1..self.x2 {
                let clr = self.pixel(x, y);
                bytes[i] = clr.r;
                bytes[i + 1] = clr.g;
                bytes[i + 2] = clr.b;
                i += 3;
            }
        }
        bytes
    }

    pub fn save_png<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let bytes = self.get_bytes();
        let img = image::RgbImage::from_raw(self.width(), self.height(), bytes)
            .context("RgbImage::from_raw failed")?;
        img.save_with_format(path, image::ImageFormat::Png)
            .context("save png")?;
        Ok(())
    }

    /// Gets a subimage with the same height and provided width aligned to the left with the right side trimmed.
    pub fn trimmed_left(&self, width: u32) -> Self {
        let size = width.min(self.width());

        Self {
            x1: self.x1,
            y1: self.y1,
            x2: self.x1 + size,
            y2: self.y2,
            true_width: self.true_width,
            data: self.data,
        }
    }

    /// Gets a subimage with the same height and provided width aligned to the right with the left side trimmed.
    pub fn trimmed_right(&self, width: u32) -> Self {
        let size = width.min(self.width());

        Self {
            x1: self.x2 - size,
            y1: self.y1,
            x2: self.x2,
            y2: self.y2,
            true_width: self.true_width,
            data: self.data,
        }
    }

    /// Gets a subimage with the same height and provided width aligned in the center with both sides trimmed.
    pub fn trimmed_centerh(&self, width: u32) -> Self {
        let size = width.min(self.width());
        let size = (size >> 1) << 1; // make number even (prevents off-by-one splits)
        let spacing = (self.width() - size) / 2;

        Self {
            x1: self.x1 + spacing,
            y1: self.y1,
            x2: self.x2 - spacing,
            y2: self.y2,
            true_width: self.true_width,
            data: self.data,
        }
    }

    /// Gets a subimage with the same width and provided height aligned to the top with the bottom side trimmed.
    pub fn trimmed_top(&self, height: u32) -> Self {
        let size = height.min(self.height());

        Self {
            x1: self.x1,
            y1: self.y1,
            x2: self.x2,
            y2: self.y1 + size,
            true_width: self.true_width,
            data: self.data,
        }
    }

    /// Gets a subimage with the same width and provided height aligned to the bottom with the top side trimmed.
    pub fn trimmed_bottom(&self, height: u32) -> Self {
        let size = height.min(self.height());

        Self {
            x1: self.x1,
            y1: self.y2 - size,
            x2: self.x2,
            y2: self.y2,
            true_width: self.true_width,
            data: self.data,
        }
    }

    /// Gets a subimage with the same width and provided height aligned in the center with both sides trimmed.
    pub fn trimmed_centerv(&self, height: u32) -> Self {
        let size = height.min(self.height());
        let size = (size >> 1) << 1; // make number even
        let spacing = (self.height() - size) / 2;

        Self {
            x1: self.x1,
            y1: self.y1 + spacing,
            x2: self.x2,
            y2: self.y2 - spacing,
            true_width: self.true_width,
            data: self.data,
        }
    }

    /// Create an arbitrary subimage (relative coordinates).
    pub fn sub_image(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
        let x = x.min(self.width());
        let y = y.min(self.height());
        let width = width.min(self.width() - x);
        let height = height.min(self.height() - y);

        Self {
            x1: self.x1 + x,
            y1: self.y1 + y,
            x2: self.x1 + x + width,
            y2: self.y1 + y + height,
            true_width: self.true_width,
            data: self.data,
        }
    }

    pub fn average_color(&self) -> Color {
        let mut r = 0u32;
        let mut g = 0u32;
        let mut b = 0u32;

        for y in self.y1..self.y2 {
            for x in self.x1..self.x2 {
                let clr = self.pixel(x, y);
                r += clr.r as u32;
                g += clr.g as u32;
                b += clr.b as u32;
            }
        }

        let count = (self.width() * self.height()) as u32;
        Color {
            r: (r / count) as u8,
            g: (g / count) as u8,
            b: (b / count) as u8,
        }
    }

    pub fn average_color_masked(&self, mask: Mask) -> Color {
        let mut count = 0u32;
        let mut r = 0u32;
        let mut g = 0u32;
        let mut b = 0u32;

        let mut i = 0usize;
        for y in 0..self.height() {
            for x in 0..self.width() {
                let yes = ((mask.0[i / 8] >> (i % 8)) & 1) == 1;
                i += 1;
                if !yes {
                    continue;
                }

                let clr = self.pixel(self.x1 + x, self.y1 + y);
                r += clr.r as u32;
                g += clr.g as u32;
                b += clr.b as u32;
                count += 1;
            }
        }

        if count == 0 {
            return Color::BLACK;
        }

        Color {
            r: (r / count) as u8,
            g: (g / count) as u8,
            b: (b / count) as u8,
        }
    }

    pub fn average_deviation_masked(&self, other: Image, mask: Mask) -> f32 {
        if self.width() != other.width() {
            return f32::MAX;
        }
        if self.height() != other.height() {
            return f32::MAX;
        }

        let mut count = 0u32;
        let mut deviation = 0.0f32;

        let mut i = 0usize;
        for y in 0..self.height() {
            for x in 0..self.width() {
                let yes = ((mask.0[i / 8] >> (i % 8)) & 1) == 1;
                i += 1;
                if !yes {
                    continue;
                }

                deviation += self
                    .pixel(self.x1 + x, self.y1 + y)
                    .deviation(*other.pixel(other.x1 + x, other.y1 + y));
                count += 1;
            }
        }

        if count == 0 {
            return 0.0;
        }
        deviation / count as f32
    }

    /// Extract text using OCR with preprocessing (grayscale, thresholding, upscale).
    ///
    /// The function tries multiple preprocessing strategies (adaptive threshold,
    /// Otsu threshold, theme-guided) and picks the most plausible result.
    pub fn get_text(&self, theme: crate::Theme, ocr: &crate::ocr::Ocr) -> String {
        use imageproc::contrast::{adaptive_threshold, equalize_histogram, otsu_level, threshold, ThresholdType};

        // Upscale small crops – OCR generally performs better on larger glyphs.
        let mut base = self.to_owned_image();
        const MIN_H: u32 = 80;
        if base.height < MIN_H {
            base = base.resized_h(MIN_H);
        }

        // Candidate 1: adaptive threshold (handles gradients/transparency).
        let adaptive = {
            let gray = equalize_histogram(&base.to_gray_image());
            let bin = adaptive_threshold(&gray, 7, 10);
            OwnedImage::from_gray_as_rgb(&ensure_dark_text_on_light(bin))
        };

        // Candidate 2: global Otsu.
        let otsu = {
            let gray = equalize_histogram(&base.to_gray_image());
            let level = otsu_level(&gray);
            let bin = threshold(&gray, level, ThresholdType::Binary);
            OwnedImage::from_gray_as_rgb(&ensure_dark_text_on_light(bin))
        };

        // Candidate 3: theme-guided (fallback).
        let theme_bin = {
            let mut img = base.clone();
            img.map_pixels(|v| {
                let d1 = v.deviation(theme.primary);
                let d2 = v.deviation(theme.secondary);
                *v = if d1 < d2 { Color::WHITE } else { Color::BLACK };
            });
            img
        };

        let mut best = String::new();
        let mut best_score = i64::MIN;

        for cand in [adaptive, otsu, theme_bin] {
            let text = ocr.get_text(cand.as_image());
            let score = score_ocr_text(&text);
            if score > best_score {
                best_score = score;
                best = text;
            }
        }

        // Optional debug snapshots.
        if std::env::var("WFBUDDY_WRITE_IMAGE").as_deref() == Ok("1") {
            if let Some(name) = best.chars().filter(|c| c.is_ascii_alphanumeric()).take(40).collect::<String>().get(0..) {
                let _ = self.save_png(format!("./debug_ocr_{}.png", name));
            }
        }

        best
    }
}

fn ensure_dark_text_on_light(mut bin: image::GrayImage) -> image::GrayImage {
    // If the image is mostly black, invert it so background becomes light.
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
    bin
}

fn score_ocr_text(text: &str) -> i64 {
    // Prefer strings with more alphanumerics (less noise) and slightly longer length.
    let mut score = 0i64;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            score += 3;
        } else if ch.is_whitespace() {
            score += 0;
        } else {
            score += 1;
        }
    }
    score + text.len() as i64
}

// ----------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const WHITE: Self = Self::new(255, 255, 255);
    pub const BLACK: Self = Self::new(0, 0, 0);

    #[inline]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Deviation metric used throughout the project for UI color checks.
    ///
    /// NOTE: This is intentionally *not* Euclidean distance; it is tuned for
    /// robust thresholding in the presence of compression and post-processing.
    pub fn deviation(&self, other: Color) -> f32 {
        (((self.r as f32 - other.r as f32).abs() / 255.0 / 3.0
            + (self.g as f32 - other.g as f32).abs() / 255.0 / 3.0
            + (self.b as f32 - other.b as f32).abs() / 255.0 / 3.0)
            / 0.05)
            .powi(3)
    }

    /// Compute luma (grayscale intensity).
    pub fn luma(&self) -> u8 {
        let r = self.r as u32;
        let g = self.g as u32;
        let b = self.b as u32;
        ((299 * r + 587 * g + 114 * b) / 1000) as u8
    }
}
