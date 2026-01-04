use std::path::Path;

use anyhow::{Context, Result};

// Built-in fallback models.
//
// These make the app "just work" even if the release zip is unpacked without the `ocr/` folder.
// If the user *does* ship assets externally (or wants custom models), we still prefer those.
//
// Note: We embed the repo's `ocr/latin_*` assets as a sane default for Warframe.
static EMBEDDED_DET: &[u8] = include_bytes!("../../ocr/detection.mnn");
static EMBEDDED_REC: &[u8] = include_bytes!("../../ocr/latin_recognition.mnn");
static EMBEDDED_KEYS: &[u8] = include_bytes!("../../ocr/latin_charset.txt");

pub struct Ocr {
	engine: ocr_rs::OcrEngine,
}

impl Ocr {
	pub fn try_new(detection: impl AsRef<Path>, recognition: impl AsRef<Path>, charsset: impl AsRef<Path>) -> Result<Self> {
		let detection = detection.as_ref();
		let recognition = recognition.as_ref();
		let charsset = charsset.as_ref();

		let config = ocr_rs::OcrEngineConfig {
			backend: ocr_rs::Backend::CPU,
			// Keep this conservative; OCR runs often and must not starve the UI thread.
			thread_count: 1,
			// Low is fast but tends to drop short UI strings. We compensate by doing our own filtering.
			precision_mode: ocr_rs::PrecisionMode::Low,
			enable_parallel: false,
			// Let the model return low-confidence results; we pick the best candidate ourselves.
			min_result_confidence: 0.25,
			..Default::default()
		};

		let engine = match (
			detection.is_file(),
			recognition.is_file(),
			charsset.is_file(),
		) {
			(true, true, true) => ocr_rs::OcrEngine::new(
				detection,
				recognition,
				charsset,
				Some(config),
			),
				_ => ocr_rs::OcrEngine::from_bytes(
					EMBEDDED_DET,
					EMBEDDED_REC,
					EMBEDDED_KEYS,
					Some(config),
				),
			}
			.with_context(|| format!(
			"Failed to initialize OCR engine.\n  detection: {}\n  recognition: {}\n  charset: {}\n\nTip: ship the 'ocr/' folder next to the executable (or set WFBUDDY_ASSETS_DIR).",
			detection.display(),
			recognition.display(),
			charsset.display(),
		))?;
		
		Ok(Self { engine })
	}

	/// Backwards-compatible constructor.
	///
	/// Prefer [`Ocr::try_new`] so the caller can surface a useful error instead of panicking.
	pub fn new(detection: impl AsRef<Path>, recognition: impl AsRef<Path>, charsset: impl AsRef<Path>) -> Self {
		Self::try_new(detection, recognition, charsset)
			.expect("OCR initialization failed (see paths above)")
	}
	
	/// Runs OCR and returns (text, confidence).
	///
	/// Confidence is the *average* confidence across returned lines (0.0..=1.0).
	pub fn get_text_with_confidence(&self, image: crate::Image) -> (String, f32) {
		let image = ocr_rs::preprocess::rgb_to_image(&image.get_bytes(), image.width(), image.height());
		match self.engine.recognize(&image) {
			Ok(result) => {
				if result.is_empty() {
					return (String::new(), 0.0);
				}
				let mut conf_sum = 0.0f32;
				let mut parts = Vec::with_capacity(result.len());
				for v in result {
					conf_sum += v.confidence;
					parts.push(v.text);
				}
				(parts.join(" "), conf_sum / parts.len() as f32)
			}
			Err(err) => {
				// OCR errors should not crash the whole app.
				if std::env::var_os("WFBUDDY_DEBUG_OCR").is_some() {
					log::warn!("OCR recognize() failed: {err:?}");
				}
				(String::new(), 0.0)
			}
		}
	}

	/// Convenience wrapper that only returns text.
	pub fn get_text(&self, image: crate::Image) -> String {
		self.get_text_with_confidence(image).0
	}
}