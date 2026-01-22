//! OCR wrapper.
//!
//! The project primarily relies on `ocr-rs` (Rust PaddleOCR bindings).
//! OCR engines are sensitive to input quality, so most preprocessing is done
//! in `Image::get_text(...)` before calling into this module.

use std::path::Path;

use anyhow::Context;

pub struct Ocr {
    engine: ocr_rs::OcrEngine,
}

impl Ocr {
    /// Initialize the OCR engine with the given model paths.
    ///
    /// This is kept infallible from the perspective of the rest of the codebase
    /// (errors are mapped to a panic) because failing to load OCR models is a
    /// hard configuration error for the application.
    pub fn new(
        detection: impl AsRef<Path>,
        recognition: impl AsRef<Path>,
        charsset: impl AsRef<Path>,
    ) -> Self {
        let thread_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        let engine = ocr_rs::OcrEngine::new(
            detection,
            recognition,
            charsset,
            Some(ocr_rs::OcrEngineConfig {
                backend: ocr_rs::Backend::CPU,
                thread_count,
                // Accuracy-focused: preprocessing is usually more important than
                // the precision mode, but High generally improves results on
                // small stylized fonts at a CPU cost.
                precision_mode: ocr_rs::PrecisionMode::High,
                enable_parallel: thread_count > 1,
                min_result_confidence: 0.5,
                ..Default::default()
            }),
        )
        .context("failed to initialize OCR engine")
        .expect("OCR engine init failed (missing or invalid model files?)");

        Self { engine }
    }

    /// Recognize text from an RGB image view.
    pub fn get_text(&self, image: crate::Image) -> String {
        let image = ocr_rs::preprocess::rgb_to_image(&image.get_bytes(), image.width(), image.height());

        match self.engine.recognize(&image) {
            Ok(results) => results
                .into_iter()
                .map(|v| v.text)
                .collect::<Vec<_>>()
                .join(" "),
            Err(_) => String::new(),
        }
    }
}
