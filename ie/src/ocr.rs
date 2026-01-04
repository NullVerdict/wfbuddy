use std::path::Path;

pub struct Ocr {
	engine: ocr_rs::OcrEngine,
}

impl Ocr {
	pub fn new(detection: impl AsRef<Path>, recognition: impl AsRef<Path>, charsset: impl AsRef<Path>) -> Self {
		let engine = ocr_rs::OcrEngine::new(detection, recognition, charsset, Some(ocr_rs::OcrEngineConfig {
			backend: ocr_rs::Backend::CPU,
			thread_count: 1,
			precision_mode: ocr_rs::PrecisionMode::Low,
			enable_parallel: false,
			min_result_confidence: 0.5,
			..Default::default()
		})).unwrap();
		
		Self{engine}
	}
	
	pub fn get_text(&self, image: crate::Image) -> String {
		let image = ocr_rs::preprocess::rgb_to_image(&image.get_bytes(), image.width(), image.height());
		self.engine.recognize(&image)
			.unwrap()
			.into_iter()
			.map(|v| v.text)
			.collect::<Vec<_>>()
			.join(" ")
	}
}