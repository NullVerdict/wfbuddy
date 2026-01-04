pub struct OwnedMask(pub Vec<u8>);
pub struct Mask<'a>(pub &'a [u8]);

#[derive(Clone)]
pub struct OwnedImage {
	width: u32,
	height: u32,
	data: Vec<Color>
}

impl OwnedImage {
	pub fn from_rgba(width: usize, bytes: &[u8]) -> Self {
		let height = bytes.len() / width / 4;
		let data = bytes
			.chunks_exact(4)
			.map(|v| Color::new(v[0], v[1], v[2]))
			.collect::<Vec<_>>();
		
		Self {
			width: width as u32,
			height: height as u32,
			data
		}
	}
	
	pub fn from_png_mask(bytes: &[u8], alpha_thresshold: u8) -> Result<(Self, OwnedMask), Box<dyn std::error::Error>> {
		let mut reader = png::Decoder::new(std::io::Cursor::new(bytes));
		reader.set_transformations(png::Transformations::all());
		let mut reader = reader.read_info()?;
		let mut buf = vec![0u8; reader.output_buffer_size().ok_or("Png too big for this systems memory (how tf)")?];
		let info = reader.next_frame(&mut buf)?;
		let bytes = &buf[..info.buffer_size()];
		let height = bytes.len() / info.width as usize / 4;
		
		let mut data = Vec::with_capacity(info.width as usize * height);
		let mut mask = vec![0u8; info.width as usize * height / 8 + 1];
		
		for (i, v) in bytes.chunks_exact(4).enumerate() {
			data.push(Color::new(v[0], v[1], v[2]));
			if v[3] >= alpha_thresshold {
				mask[i / 8] |= 1 << (i % 8);
			}
		}
		
		Ok((
			Self {
				width: info.width,
				height: height as u32,
				data
			},
			OwnedMask(mask),
		))
	}
	
	pub fn resize_h(&mut self, height: u32) {
		if self.height == height {
			return;
		}

		if std::env::var_os("WFBUDDY_DEBUG_OCR").is_some() {
			log::debug!("[ocr] resizing image: {}x{} -> ?x{}", self.width, self.height, height);
		}

		// Preserve aspect ratio.
		let width = self.width * height / self.height;

		// fast_image_resize::ImageRef expects a byte buffer; build RGB bytes from our Color pixels.
		let src_bytes: Vec<u8> = self.data.iter().flat_map(|c| [c.r, c.g, c.b]).collect();

		let img = fast_image_resize::images::ImageRef::new(
			self.width,
			self.height,
			&src_bytes,
			fast_image_resize::PixelType::U8x3,
		)
		.unwrap();

		let mut dst = fast_image_resize::images::Image::new(
			width,
			height,
			fast_image_resize::PixelType::U8x3,
		);

		let mut resizer = fast_image_resize::Resizer::new();
		resizer
			.resize(
				&img,
				&mut dst,
				&Some(
					fast_image_resize::ResizeOptions::new().resize_alg(
						fast_image_resize::ResizeAlg::Convolution(
							fast_image_resize::FilterType::CatmullRom,
						),
					),
				),
			)
			.unwrap();

		let dst_bytes = dst.into_vec();
		let mut data = Vec::with_capacity((width * height) as usize);
		for rgb in dst_bytes.chunks_exact(3) {
			data.push(Color {
				r: rgb[0],
				g: rgb[1],
				b: rgb[2],
			});
		}

		*self = Self { width, height, data };
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
	
	// Since we cant deref to a lifetime object
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

	/// Adds a constant border around the image.
	///
	/// This helps OCR if the crop is tight and characters touch the edge.
	pub fn padded(self, pad: u32, color: Color) -> Self {
		if pad == 0 {
			return self;
		}
		let new_w = self.width + pad.saturating_mul(2);
		let new_h = self.height + pad.saturating_mul(2);
		let mut data = vec![color; (new_w * new_h) as usize];
		for y in 0..self.height {
			let src_row = (y * self.width) as usize;
			let dst_row = ((y + pad) * new_w + pad) as usize;
			data[dst_row..dst_row + self.width as usize]
				.copy_from_slice(&self.data[src_row..src_row + self.width as usize]);
		}
		Self {
			width: new_w,
			height: new_h,
			data,
		}
	}
}

// ----------

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
				bytes[i    ] = clr.r;
				bytes[i + 1] = clr.g;
				bytes[i + 2] = clr.b;
				i += 3;
			}
		}
		
		bytes
	}
	
	pub fn save_png<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
		let f = std::fs::File::create(path)?;
		let mut e = png::Encoder::new(std::io::BufWriter::new(f), self.x2 - self.x1, self.y2 - self.y1);
		e.set_color(png::ColorType::Rgb);
		e.set_depth(png::BitDepth::Eight);
		let mut w = e.write_header()?;
		// w.write_image_data(unsafe{std::slice::from_raw_parts(self.data[..].as_ptr() as *const _, self.data.len() * 3)})?;
		w.write_image_data(&self.get_bytes())?;
		
		Ok(())
	}
	
	/// Gets a subimage with the same height and provided width aligned to the left with the right side trimmed
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
	
	/// Gets a subimage with the same height and provided width aligned to the right with the left side trimmed
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
	
	/// Gets a subimage with the same height and provided width aligned in the center with both sides trimmed
	pub fn trimmed_centerh(&self, width: u32) -> Self {
		let size = width.min(self.width());
		let size = (size >> 1) << 1; // make number even, since uneven numbers would break shit
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
	
	/// Gets a subimage with the same width and provided height aligned to the top with the bottom side trimmed
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

	/// Gets a subimage with the same width and provided height aligned to the bottom with the top side trimmed
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

	/// Gets a subimage with the same width and provided height aligned in the center with both sides trimmed
	pub fn trimmed_centerv(&self, height: u32) -> Self {
		let size = height.min(self.height());
		let size = (size >> 1) << 1; // keep even; some downstream code assumes even dimensions
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

		let count = self.width() * self.height();
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

		// Mask bits are laid out row-major (x increases fastest), matching this iteration order.
		let mut i = 0usize;
		for y in self.y1..self.y2 {
			for x in self.x1..self.x2 {
				let yes = ((mask.0[i / 8] >> (i % 8)) & 1) == 1;
				i += 1;
				if !yes {
					continue;
				}

				let clr = self.pixel(x, y);
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
		if self.x2 - self.x1 != other.x2 - other.x1 {
			return f32::MAX;
		}
		if self.y2 - self.y1 != other.y2 - other.y1 {
			return f32::MAX;
		}

		// NOTE: This function is ONLY used for template/icon matching.
		// Use luminance distance instead of RGB distance so matching is robust to
		// in-game tint/grade changes (e.g., the gold "rare" icon).
		#[inline]
		fn luma(c: Color) -> u8 {
			// Integer approximation of Rec.601 luma.
			((c.r as u16 * 77 + c.g as u16 * 150 + c.b as u16 * 29) >> 8) as u8
		}

		let w = self.x2 - self.x1;
		let h = self.y2 - self.y1;

		let mut count = 0u32;
		let mut deviation = 0.0f32;

		let mut i = 0usize;
		for y in 0..h {
			for x in 0..w {
				let yes = ((mask.0[i / 8] >> (i % 8)) & 1) == 1;
				i += 1;
				if !yes {
					continue;
				}

				let a = luma(*self.pixel(self.x1 + x, self.y1 + y));
				let b = luma(*other.pixel(other.x1 + x, other.y1 + y));
				deviation += (a.abs_diff(b) as f32) / 255.0;
				count += 1;
			}
		}

		if count == 0 {
			return 0.0;
		}

		deviation / count as f32
	}

	pub fn get_text(&self, theme: crate::Theme, ocr: &crate::ocr::Ocr) -> String {
		#[derive(Clone, Copy)]
		enum Polarity {
			BlackOnWhite,
			WhiteOnBlack,
		}

		#[inline]
		fn luma(c: Color) -> u8 {
			// Integer approximation of Rec.601 luma.
			((c.r as u16 * 77 + c.g as u16 * 150 + c.b as u16 * 29) >> 8) as u8
		}


		fn otsu_threshold(img: &OwnedImage) -> u8 {
			let mut hist = [0u32; 256];
			for px in &img.data {
				hist[luma(*px) as usize] += 1;
			}
			let total = img.data.len() as f32;
			if total <= 1.0 {
				return 128;
			}

			let mut sum_total = 0.0f32;
			for (i, &h) in hist.iter().enumerate() {
				sum_total += i as f32 * h as f32;
			}

			let mut sum_b = 0.0f32;
			let mut w_b = 0.0f32;
			let mut best_var = -1.0f32;
			let mut best_t = 128u8;

			for (t, &h) in hist.iter().enumerate() {
				let h = h as f32;
				w_b += h;
				if w_b == 0.0 {
					continue;
				}
				let w_f = total - w_b;
				if w_f == 0.0 {
					break;
				}
				sum_b += t as f32 * h;
				let m_b = sum_b / w_b;
				let m_f = (sum_total - sum_b) / w_f;
				let var_between = w_b * w_f * (m_b - m_f) * (m_b - m_f);
				if var_between > best_var {
					best_var = var_between;
					best_t = t as u8;
				}
			}
			best_t
		}

		fn dilate_binary(image: &mut OwnedImage, text: Color, bg: Color) {
			let w = image.width as i32;
			let h = image.height as i32;
			let src = image.data.clone();
			let mut dst = vec![bg; src.len()];

			for y in 0..h {
				for x in 0..w {
					let mut hit = false;
					for dy in -1..=1 {
						for dx in -1..=1 {
							let nx = x + dx;
							let ny = y + dy;
							if nx < 0 || ny < 0 || nx >= w || ny >= h {
								continue;
							}
							let idx = (ny * w + nx) as usize;
							if src[idx] == text {
								hit = true;
								break;
							}
						}
						if hit {
							break;
						}
					}
					let idx = (y * w + x) as usize;
					dst[idx] = if hit { text } else { bg };
				}
			}

			image.data = dst;
		}

		fn binarize_theme(image: &mut OwnedImage, theme: crate::Theme, thr: f32, polarity: Polarity) {
			image.map_pixels(|v| {
				let is_text = v.deviation(theme.primary) < thr || v.deviation(theme.secondary) < thr;
				*v = match (is_text, polarity) {
					(true, Polarity::BlackOnWhite) => Color::BLACK,
					(false, Polarity::BlackOnWhite) => Color::WHITE,
					(true, Polarity::WhiteOnBlack) => Color::WHITE,
					(false, Polarity::WhiteOnBlack) => Color::BLACK,
				};
			});
		}

		fn binarize_luma(image: &mut OwnedImage, thr: u8, polarity: Polarity) {
			image.map_pixels(|v| {
				let y = luma(*v);
				let is_light = y >= thr;
				let is_text = match polarity {
					Polarity::BlackOnWhite => !is_light, // dark text on light background
					Polarity::WhiteOnBlack => is_light,  // light text on dark background
				};
				*v = match (is_text, polarity) {
					(true, Polarity::BlackOnWhite) => Color::BLACK,
					(false, Polarity::BlackOnWhite) => Color::WHITE,
					(true, Polarity::WhiteOnBlack) => Color::WHITE,
					(false, Polarity::WhiteOnBlack) => Color::BLACK,
				};
			});
		}

		fn normalize_text(s: String) -> String {
			// Collapse whitespace to make matching more stable.
			let mut out = String::with_capacity(s.len());
			let mut prev_space = false;
			for ch in s.chars() {
				let is_space = ch.is_whitespace();
				if is_space {
					if !prev_space {
						out.push(' ');
					}
					prev_space = true;
				} else {
					out.push(ch);
					prev_space = false;
				}
			}
			out.trim().to_string()
		}

		fn score_text(text: &str, conf: f32) -> i32 {
			let t = text.trim();
			if t.is_empty() {
				return i32::MIN / 2;
			}
			let total = t.chars().count() as i32;
			let allowed = t
				.chars()
				.filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '\'' | '.' | '(' | ')' | '/' ))
				.count() as i32;
			let weird = total - allowed;
			let mut score = allowed * 2 - weird * 4 + (conf * 100.0) as i32;
			if t.chars().any(|c| c.is_ascii_alphabetic()) {
				score += 10;
			}
			if total < 3 {
				score -= 25;
			}
			score
		}

		fn prep_for_ocr(mut img: OwnedImage, pad_color: Color) -> OwnedImage {
			// Make room around glyphs so detectors don't clip characters.
			img = img.padded(8, pad_color);

			// UI text is often small; upscaling cropped regions improves recognition.
			// Avoid downscaling unless the crop is huge.
			let desired_h = if img.height <= 90 { 96 } else { 64 };
			let desired_h = if img.height < desired_h { desired_h } else { img.height };
			let desired_h = desired_h.min(160);
			img.resize_h(desired_h);
			img
		}

		let debug = std::env::var_os("WFBUDDY_DEBUG_OCR").is_some();
		let mut best_text = String::new();
		let mut best_score = i32::MIN / 2;
		let mut best_img: Option<OwnedImage> = None;

		// Candidate 1: raw crop (sometimes the model likes the original colors/anti-aliasing).
		{
			let img = prep_for_ocr(self.to_owned_image(), Color::BLACK);
			let (text, conf) = ocr.get_text_with_confidence(img.as_image());
			let text = normalize_text(text);
			let sc = score_text(&text, conf);
			if debug {
				log::debug!("[ocr/raw] conf={conf:.2} score={sc} text='{text}'");
			}
			if sc > best_score {
				best_score = sc;
				best_text = text;
				best_img = Some(img);
			}
		}

		// Candidate 2-4: theme-guided binarization.
		for (thr, label) in [(5.0f32, "bw_strict"), (8.0f32, "bw_loose"), (11.0f32, "bw_vloose")] {
			let mut img = self.to_owned_image();
			binarize_theme(&mut img, theme, thr, Polarity::BlackOnWhite);
			dilate_binary(&mut img, Color::BLACK, Color::WHITE);
			let img = prep_for_ocr(img, Color::WHITE);
			let (text, conf) = ocr.get_text_with_confidence(img.as_image());
			let text = normalize_text(text);
			let sc = score_text(&text, conf);
			if debug {
				log::debug!("[ocr/{label}] conf={conf:.2} score={sc} text='{text}'");
			}
			if sc > best_score {
				best_score = sc;
				best_text = text;
				best_img = Some(img);
			}
			// Early exit if we have a very confident, non-trivial read.
			if conf >= 0.92 && best_text.len() >= 6 {
				break;
			}
		}

		// Candidate 5: theme-guided inverted.
		{
			let mut img = self.to_owned_image();
			binarize_theme(&mut img, theme, 7.0, Polarity::WhiteOnBlack);
			dilate_binary(&mut img, Color::WHITE, Color::BLACK);
			let img = prep_for_ocr(img, Color::BLACK);
			let (text, conf) = ocr.get_text_with_confidence(img.as_image());
			let text = normalize_text(text);
			let sc = score_text(&text, conf);
			if debug {
				log::debug!("[ocr/inv] conf={conf:.2} score={sc} text='{text}'");
			}
			if sc > best_score {
				best_score = sc;
				best_text = text;
				best_img = Some(img);
			}
		}

		// Candidate 6-7: luminance/Otsu binarization (robust when the UI text color isn't tied to the theme).
		{
			let base = self.to_owned_image();
			let t = otsu_threshold(&base);

			for (polarity, label) in [(Polarity::BlackOnWhite, "otsu_bw"), (Polarity::WhiteOnBlack, "otsu_wb")] {
				let mut img = base.clone();
				binarize_luma(&mut img, t, polarity);
				match polarity {
					Polarity::BlackOnWhite => dilate_binary(&mut img, Color::BLACK, Color::WHITE),
					Polarity::WhiteOnBlack => dilate_binary(&mut img, Color::WHITE, Color::BLACK),
				}
				let pad = match polarity {
					Polarity::BlackOnWhite => Color::WHITE,
					Polarity::WhiteOnBlack => Color::BLACK,
				};
				let img = prep_for_ocr(img, pad);
				let (text, conf) = ocr.get_text_with_confidence(img.as_image());
				let text = normalize_text(text);
				let sc = score_text(&text, conf);
				if debug {
					log::debug!("[ocr/{label}] thr={t} conf={conf:.2} score={sc} text='{text}'");
				}
				if sc > best_score {
					best_score = sc;
					best_text = text;
					best_img = Some(img);
				}
			}
		}

		if std::env::var("WFBUDDY_WRITE_IMAGE").as_deref() == Ok("1")
			&& let Some(img) = best_img
		{
			let mut n = best_text.clone();
			n.retain(|v| v.is_ascii_alphanumeric());
			let _ = img.as_image().save_png(format!("./ocr_debug_{n}.png"));
		}

		best_text
	}
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
		Self{r, g, b}
	}
	
	pub fn deviation(&self, other: Color) -> f32 {
		(((self.r as f32 - other.r as f32).abs() / 255.0 / 3.0 +
		(self.g as f32 - other.g as f32).abs() / 255.0 / 3.0 +
		(self.b as f32 - other.b as f32).abs() / 255.0 / 3.0) / 0.05).powi(3)
	}

	/// A simple bounded color distance in the range ~0.0..=1.0.
	///
	/// This is useful for template/icon matching where we need predictable thresholds.
	#[inline]
	pub fn diff01(&self, other: Color) -> f32 {
		((self.r.abs_diff(other.r) as f32)
			+ (self.g.abs_diff(other.g) as f32)
			+ (self.b.abs_diff(other.b) as f32))
			/ (255.0 * 3.0)
	}
}