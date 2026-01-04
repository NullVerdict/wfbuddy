pub struct OwnedMask(pub Vec<u8>);
pub struct Mask<'a>(pub &'a [u8]);

pub struct OwnedImage {
	width: u32,
	height: u32,
	data: Vec<Color>
}

impl OwnedImage {
	pub fn from_rgba(width: usize, bytes: &[u8]) -> Self {
		let height = bytes.len() as usize / width / 4;
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
		let height = bytes.len() as usize / info.width as usize / 4;
		
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
		if self.height == height {return}
		println!("resizing");
		// cba implementing the trait on this
		let width = self.width * height / self.height;
		let img = fast_image_resize::images::ImageRef::from_pixels(self.width, self.height, unsafe{std::slice::from_raw_parts(self.data[..].as_ptr() as *const fast_image_resize::pixels::U8x3, self.data.len())}).unwrap();
		let mut dst = fast_image_resize::images::Image::new(width, height, fast_image_resize::PixelType::U8x3);
		
		let mut resizer = fast_image_resize::Resizer::new();
		resizer.resize(&img, &mut dst, &Some(fast_image_resize::ResizeOptions::new().resize_alg(fast_image_resize::ResizeAlg::Interpolation(fast_image_resize::FilterType::CatmullRom)))).unwrap();
		
		*self = Self {
			width,
			height,
			data: unsafe{std::mem::transmute(dst.into_vec())},
		}
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

	/// Get a pixel by *relative* coordinates within this view (0..width, 0..height).
	///
	/// Many internal routines work with relative coordinates; exposing this helper
	/// keeps image views encapsulated while allowing fast access when needed.
	#[inline(always)]
	pub fn pixel_rel(&self, x: u32, y: u32) -> Color {
		*self.pixel(self.x1 + x, self.y1 + y)
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
		let size = height.min(self.width());
		
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
		let size = height.min(self.width());
		
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
		let size = height.min(self.width());
		let size = (size >> 1) << 1; // make number even, since uneven numbers would break shit
		let spacing = (self.width() - size) / 2;
		
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
		// println!("{} {} {} {}", self.x1, self.x2, self.y1, self.y2);
		
		let mut r = 0u32;
		let mut g = 0u32;
		let mut b = 0u32;
		
		for x in self.x1..self.x2 {
			for y in self.y1..self.y2 {
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
		let mut count = 0;
		let mut r = 0u32;
		let mut g = 0u32;
		let mut b = 0u32;
		let mut i = 0;
		for x in self.x1..self.x2 {
			for y in self.y1..self.y2 {
				let yes = ((mask.0[i / 8] >> (i % 8)) & 1) == 1;
				i += 1;
				if !yes {continue}
				
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
		if self.x2 - self.x1 != other.x2 - other.x1 {return f32::MAX};
		if self.y2 - self.y1 != other.y2 - other.y1 {return f32::MAX};
		
		let mut count = 0;
		let mut deviation = 0.0;
		let mut i = 0;
		for x in 0..self.x2 - self.x1 {
			for y in 0..self.y2 - self.y1 {
				let yes = ((mask.0[i / 8] >> (i % 8)) & 1) == 1;
				i += 1;
				if !yes {continue}
				
				deviation += self.pixel(self.x1 + x, self.y1 + y).deviation(*other.pixel(other.x1 + x, other.y1 + y));
				count += 1;
			}
		}
		
		if count == 0 {
			return 0.0;
		}
		
		deviation / count as f32
	}
	
	pub fn get_text(&self, theme: crate::Theme, ocr: &crate::ocr::Ocr) -> String {
		let mut image = self.to_owned_image();
		image.map_pixels(|v| *v = if v.deviation(theme.primary) < 5.0 || v.deviation(theme.secondary) < 5.0 {Color::WHITE} else {Color::BLACK});
		
		let text = ocr.get_text(image.as_image());
		println!("get text: {text}");
		if std::env::var("WFBUDDY_WRITE_IMAGE") == Ok("1".to_string()) {
			let mut n = text.clone();
			n.retain(|v| v.is_ascii_alphanumeric());
			image.as_image().save_png(format!("./test_{n}.png")).unwrap();
		}
		
		text
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
}