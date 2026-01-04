pub mod relicarcane;
pub mod recipes;
pub mod resources;
pub mod warframes;
pub mod weapons;
pub mod sentinels;

const URL_MANIFEST: &str = "http://content.warframe.com/PublicExport/Manifest/";
const URL_EN: &str = "https://origin.warframe.com/PublicExport/index_en.txt.lzma";

// TODO: maybe lazyload the subpages
pub struct PublicExport {
	pub relic_arcane_url: String,
	pub recipes_url: String,
	pub resources_url: String,
	pub warframes_url: String,
	pub weapons_url: String,
	pub sentinels_url: String,
}

impl PublicExport {
	fn new_url(url: &str) -> Result<Self, anyhow::Error> {
		let data = ureq::get(url)
			.call()?
			.body_mut()
			.read_to_vec()?;
		
		let mut urls = Vec::new();
		lzma_rs::lzma_decompress(&mut std::io::Cursor::new(data), &mut urls)?;
		let urls = String::from_utf8(urls)?;
		let urls = urls.split("\r\n").collect::<Vec<_>>();
		
		println!("urls: {urls:#?}");
		
		Ok(Self {
			relic_arcane_url: manifest_url(select_url(&urls, "ExportRelicArcane").ok_or(anyhow::Error::msg(format!("index didn't contain ExportRelicArcane")))?),
			recipes_url: manifest_url(select_url(&urls, "ExportRecipes").ok_or(anyhow::Error::msg(format!("index didn't contain ExportRecipes")))?),
			resources_url: manifest_url(select_url(&urls, "ExportResources").ok_or(anyhow::Error::msg(format!("index didn't contain ExportResources")))?),
			warframes_url: manifest_url(select_url(&urls, "ExportWarframes").ok_or(anyhow::Error::msg(format!("index didn't contain ExportWarframes")))?),
			weapons_url: manifest_url(select_url(&urls, "ExportWeapons").ok_or(anyhow::Error::msg(format!("index didn't contain ExportWeapons")))?),
			sentinels_url: manifest_url(select_url(&urls, "ExportSentinels").ok_or(anyhow::Error::msg(format!("index didn't contain ExportSentinels")))?),
		})
	}
	
	pub fn new(lang: crate::Language) -> Result<Self, anyhow::Error> {
		match lang {
			crate::Language::English => Self::new_url(URL_EN),
		}
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