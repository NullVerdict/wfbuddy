pub mod relicarcane;

use anyhow::{Context, Result};

const URL_MANIFEST: &str = "http://content.warframe.com/PublicExport/Manifest/";
const URL_EN: &str = "https://origin.warframe.com/PublicExport/index_en.txt.lzma";

/// Minimal PublicExport helper: we only need ExportRelicArcane for relic rewards.
pub struct PublicExport {
	pub relic_arcane_url: String,
}

impl PublicExport {
	pub fn new() -> Result<Self> {
		Self::new_url(URL_EN)
	}

	fn new_url(url: &str) -> Result<Self> {
		use std::io::Read;

		let mut reader = ureq::get(url).call().context("GET publicexport index")?.into_reader();
		let mut data = Vec::new();
		reader.read_to_end(&mut data).context("Read publicexport index bytes")?;

		let mut decompressed = Vec::new();
		lzma_rs::lzma_decompress(&mut std::io::Cursor::new(data), &mut decompressed)
			.context("LZMA decompress publicexport index")?;

		let urls = String::from_utf8(decompressed).context("UTF-8 decode publicexport index")?;
		let urls = urls.split("\r\n").collect::<Vec<_>>();

		let relic_arcane = select_url(&urls, "ExportRelicArcane")
			.ok_or_else(|| anyhow::anyhow!("PublicExport index did not contain ExportRelicArcane"))?;

		Ok(Self {
			relic_arcane_url: manifest_url(relic_arcane),
		})
	}
}

fn select_url(urls: &[&str], name: &str) -> Option<String> {
	urls.iter()
		.find(|v| v.starts_with(name))
		.map(|v| v.to_string())
}

fn manifest_url(s: impl AsRef<str>) -> String {
	format!("{URL_MANIFEST}{}", s.as_ref())
}
