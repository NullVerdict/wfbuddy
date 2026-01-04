use std::io::Read;

use anyhow::{Context, Result};

pub mod relicarcane;

const URL_MANIFEST: &str = "http://content.warframe.com/PublicExport/Manifest/";
const URL_EN: &str = "https://origin.warframe.com/PublicExport/index_en.txt.lzma";

/// Minimal PublicExport helper: we only need the Relic/Arcane export to infer which relics exist
/// and which items they can drop.
pub struct PublicExport {
	pub relic_arcane_url: String,
}

impl PublicExport {
	pub fn new_english() -> Result<Self> {
		let mut res = ureq::get(URL_EN).call().context("GET PublicExport index")?;
		let mut data = Vec::new();
		res.body_mut()
			.read_to_end(&mut data)
			.context("Read PublicExport index bytes")?;

		let mut out = Vec::new();
		lzma_rs::lzma_decompress(&mut std::io::Cursor::new(data), &mut out)
			.context("Decompress PublicExport index (lzma)")?;
		let urls = String::from_utf8(out).context("PublicExport index is not utf8")?;
		let urls = urls.split("\r\n").collect::<Vec<_>>();

		let relic = select_url(&urls, "ExportRelicArcane")
			.context("PublicExport index didn't contain ExportRelicArcane")?;

		Ok(Self {
			relic_arcane_url: manifest_url(relic),
		})
	}
}

fn select_url(urls: &[&str], name: &str) -> Option<&str> {
	urls.iter().copied().find(|v| v.starts_with(name))
}

fn manifest_url(s: impl AsRef<str>) -> String {
	format!("{URL_MANIFEST}{}", s.as_ref())
}
