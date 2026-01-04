use std::path::Path;

/// Thin wrapper around `ocr_rs::OcrEngine`.
///
/// In release distributions the OCR model files live next to the executable
/// (e.g. `./ocr/detection.mnn`). Those files may be missing/misplaced, so OCR
/// initialization must be fallible and must **not** panic.
pub struct Ocr {
	engine: Option<ocr_rs::OcrEngine>,
	init_error: Option<String>,
}

impl Ocr {
	pub fn new(
		detection: impl AsRef<Path>,
		recognition: impl AsRef<Path>,
		charsset: impl AsRef<Path>,
	) -> Self {
		let engine = ocr_rs::OcrEngine::new(
			detection,
			recognition,
			charsset,
			Some(ocr_rs::OcrEngineConfig {
				backend: ocr_rs::Backend::CPU,
				thread_count: 1,
				precision_mode: ocr_rs::PrecisionMode::Low,
				enable_parallel: false,
				min_result_confidence: 0.5,
				..Default::default()
			}),
		);

		match engine {
			Ok(engine) => Self {
				engine: Some(engine),
				init_error: None,
			},
			Err(err) => Self {
				engine: None,
				init_error: Some(err.to_string()),
			},
		}
	}

	pub fn is_available(&self) -> bool {
		self.engine.is_some()
	}

	pub fn init_error(&self) -> Option<&str> {
		self.init_error.as_deref()
	}

	pub fn get_text(&self, image: crate::Image) -> String {
		let Some(engine) = &self.engine else {
			return String::new();
		};

		let image = ocr_rs::preprocess::rgb_to_image(&image.get_bytes(), image.width(), image.height());
		match engine.recognize(&image) {
			Ok(results) => results
				.into_iter()
				.map(|v| v.text)
				.collect::<Vec<_>>()
				.join(" "),
			Err(_) => String::new(),
		}
	}
}
