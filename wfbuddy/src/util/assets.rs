use std::path::PathBuf;

use anyhow::{bail, Result};

#[derive(Debug, Clone)]
pub struct OcrAssets {
	pub detection: PathBuf,
	pub recognition: PathBuf,
	pub charset: PathBuf,
}

fn normalize_ocr_dir(dir: PathBuf) -> PathBuf {
	// Allow the env var to point either to the repo/app root (containing `ocr/`)
	// or directly to the `ocr/` folder.
	if dir.join("detection.mnn").is_file() {
		dir
	} else {
		dir.join("ocr")
	}
}

/// Resolve OCR model paths in a way that works both:
/// - when running from the repo (`cargo run`), and
/// - when running a packaged binary (assets next to the executable).
///
/// You can override discovery by setting `WFBUDDY_ASSETS_DIR`.
pub fn resolve_ocr_assets(lang_code: &str) -> Result<OcrAssets> {
	let recognition_name = format!("{lang_code}_recognition.mnn");
	let charset_name = format!("{lang_code}_charset.txt");

	let mut tried = Vec::new();

	let mut candidates: Vec<PathBuf> = Vec::new();
	if let Some(dir) = std::env::var_os("WFBUDDY_ASSETS_DIR") {
		candidates.push(PathBuf::from(dir));
	}
	if let Ok(exe) = std::env::current_exe()
		&& let Some(dir) = exe.parent()
	{
		candidates.push(dir.to_path_buf());
	}
	if let Ok(cwd) = std::env::current_dir() {
		candidates.push(cwd);
	}
	// Compile-time path to the `wfbuddy/` crate. Useful during local dev if the app is launched with a different CWD.
	#[cfg(debug_assertions)]
	candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."));

	for base in candidates {
		let ocr_dir = normalize_ocr_dir(base.clone());
		let detection = ocr_dir.join("detection.mnn");
		let recognition = ocr_dir.join(&recognition_name);
		let charset = ocr_dir.join(&charset_name);

		if detection.is_file() && recognition.is_file() && charset.is_file() {
			return Ok(OcrAssets { detection, recognition, charset });
		}

		tried.push(ocr_dir);
	}

	bail!(
		"OCR model files not found. Expected these files:\n  - ocr/detection.mnn\n  - ocr/{recognition_name}\n  - ocr/{charset_name}\n\nSearched in:\n{}\n\nFix: copy the 'ocr/' folder next to the executable (or set WFBUDDY_ASSETS_DIR to the folder that contains it).",
		tried
			.into_iter()
			.map(|p| format!("  - {}", p.display()))
			.collect::<Vec<_>>()
			.join("\n")
	)
}
