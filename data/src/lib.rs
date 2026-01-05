use std::{
	collections::{HashMap, HashSet},
	fs::File,
	io::{BufReader, BufWriter, Write},
	path::PathBuf,
};

use anyhow::{Context, Result};

mod schema;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Data {
	pub platinum_values: HashMap<String, f32>,
	pub ducat_values: HashMap<String, u32>,
	pub relic_items: HashSet<String>,
	pub vaulted_items: HashSet<String>,
	#[serde(default)]
	pub item_meta: HashMap<String, ItemMeta>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ItemMeta {
	/// Item type from WarframeStat (field name `type`).
	///
	/// Examples include things like `Blueprint`, `PrimePart`, etc.
	pub item_type: Option<String>,
	pub category: Option<String>,
	pub product_category: Option<String>,
}

impl ItemMeta {
	/// Best-effort Russian label for the item's `type`.
	///
	/// The game client often doesn't show this explicitly for relic rewards.
	pub fn type_ru(&self) -> Option<&'static str> {
		match self.item_type.as_deref()? {
			"Blueprint" => Some("Чертёж"),
			"PrimePart" => Some("Прайм-часть"),
			"PrimeSet" => Some("Прайм-набор"),
			"Relic" => Some("Реликвия"),
			"Mod" => Some("Мод"),
			_ => None,
		}
	}
}

impl Default for Data {
	fn default() -> Self {
		let mut s = Self {
			platinum_values: HashMap::new(),
			ducat_values: HashMap::new(),
			relic_items: HashSet::new(),
			vaulted_items: HashSet::new(),
			item_meta: HashMap::new(),
		};

		// Keep Forma in the dataset so the UI doesn’t special-case “missing data”.
		// (The value is just a rough ducat-to-plat approximation; adjust if you want.)
		s.platinum_values.insert("Forma Blueprint".to_string(), (350.0f32 / 3.0).floor() * 0.1);
		s.relic_items.insert("Forma Blueprint".to_string());
		s.platinum_values.insert("2 X Forma Blueprint".to_string(), (350.0f32 / 3.0).floor() * 0.2);
		s.relic_items.insert("2 X Forma Blueprint".to_string());

		s
	}
}

impl Data {
	fn cache_path() -> Option<PathBuf> {
		dirs::cache_dir().map(|p| p.join("WFBuddy").join("data_cache.json"))
	}

	fn load_cache() -> Result<Self> {
		let path = Self::cache_path().context("No cache_dir available")?;
		let file = File::open(&path).with_context(|| format!("Open cache {}", path.display()))?;
		let reader = BufReader::new(file);
		let data: Self = serde_json::from_reader(reader).with_context(|| format!("Parse cache {}", path.display()))?;
		Ok(data)
	}

	fn save_cache(&self) -> Result<()> {
		let Some(path) = Self::cache_path() else {
			return Ok(());
		};
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent).with_context(|| format!("Create cache dir {}", parent.display()))?;
		}

		let tmp = path.with_extension("json.tmp");
		let file = File::create(&tmp).with_context(|| format!("Write cache temp {}", tmp.display()))?;
		let mut writer = BufWriter::new(file);
		serde_json::to_writer(&mut writer, self).context("Serialize cache")?;
		writer.flush().context("Flush cache")?;

		// Replace existing file (Windows-friendly).
		if std::fs::rename(&tmp, &path).is_err() {
			let _ = std::fs::remove_file(&path);
			std::fs::rename(&tmp, &path).with_context(|| format!("Persist cache {}", path.display()))?;
		}
		Ok(())
	}

	fn fetch_remote() -> Result<Self> {
		let mut res = ureq::get(schema::items::URL)
			.call()
			.context("GET items")?;
		let items = res
			.body_mut()
			.read_json::<schema::items::Items>()
			.context("Decode items JSON")?;

		let mut res = ureq::get(schema::ducats::URL)
			.call()
			.context("GET ducats")?;
		let ducats = res
			.body_mut()
			.read_json::<schema::ducats::Ducats>()
			.context("Decode ducats JSON")?;

		let name_map = items
			.data
			.iter()
			.map(|v| (v.id.clone(), v.i18n.en.name.clone()))
			.collect::<HashMap<_, _>>();

		let mut s = Self {
			platinum_values: HashMap::new(),
			ducat_values: HashMap::new(),
			relic_items: HashSet::new(),
			vaulted_items: HashSet::new(),
			item_meta: HashMap::new(),
		};

		// Populate vaulted status + item metadata using WarframeStat's processing dataset.
		// Best-effort: if the endpoint is unavailable we still want the app to work.
		if let Ok((vaulted, meta)) = fetch_warframestat_items() {
			s.vaulted_items = vaulted;
			s.item_meta = meta;
		}

		for v in &ducats.payload.previous_hour {
			let name = name_map
				.get(&v.item)
				.with_context(|| format!("Missing name for item id {}", v.item))?
				.clone();
			s.platinum_values.insert(name.clone(), v.wa_price);
			s.ducat_values.insert(name.clone(), v.ducats);
			s.relic_items.insert(name);
		}

		// Ensure Forma entries exist even if the remote feed changes.
		let mut out = Self::default();
		out.platinum_values.extend(s.platinum_values);
		out.ducat_values.extend(s.ducat_values);
		out.relic_items.extend(s.relic_items);
		out.vaulted_items.extend(s.vaulted_items);
		out.item_meta.extend(s.item_meta);
		Ok(out)
	}

	pub fn item_meta(&self, name: &str) -> Option<&ItemMeta> {
		self.item_meta.get(name)
	}

	/// Fetch data from the network; if it fails, fall back to a cached copy (if available).
	pub fn try_populated() -> Result<Self> {
		match Self::fetch_remote() {
			Ok(data) => {
				let _ = data.save_cache();
				Ok(data)
			}
			Err(err) => {
				if let Ok(cached) = Self::load_cache() {
					log::warn!("Using cached market data due to network error: {err:#}");
					Ok(cached)
				} else {
					Err(err)
				}
			}
		}
	}

	/// Backwards compatible helper: never errors (uses empty defaults on failure).
	pub fn populated() -> Self {
		Self::try_populated().unwrap_or_else(|err| {
			log::warn!("Failed to load market data (no cache): {err:#}");
			Self::default()
		})
	}

	/// Attempts to find the closest item name from a dirty ocr string
	pub fn find_item_name(&self, name: &str) -> String {
		let name = name.trim_ascii();
		// When OCR returns an empty/near-empty string, *don't* guess.
		// The old behavior (Levenshtein over all items) tends to pick the shortest
		// item name (often "Bo Prime Set"), which makes the UI look "stuck".
		if name.len() < 3 {
			return "(unreadable)".to_string();
		}
		if self.relic_items.contains(name) {
			return name.to_owned();
		}

		let mut start = 0;
		while let Some(index) = name[start..].find(' ') {
			start += index + 1;
			let sub = &name[start..];
			if self.relic_items.contains(sub) {
				return sub.to_owned();
			}
		}

		let mut min_name = name;
		let mut min = usize::MAX;
		for item_name in self.relic_items.iter() {
			let lev = levenshtein::levenshtein(name, item_name);
			if lev < min {
				min_name = item_name.as_str();
				min = lev;
			}
		}

		// If the best match is still very far away, show the raw OCR text
		// so it's obvious OCR failed instead of silently "guessing".
		let max_len = name.len().max(min_name.len());
		if min > (max_len / 2).max(3) {
			return format!("{name}?");
		}

		min_name.to_string()
	}
}

/// Best-effort fetch of vaulted status from WarframeStat.
///
/// We use the processing dataset at `/items` and request only the fields we
/// need to keep the payload small.
fn fetch_warframestat_items() -> Result<(HashSet<String>, HashMap<String, ItemMeta>)> {
	let mut res = ureq::get("https://api.warframestat.us/items")
		.query("only", "name,vaulted,type,category,productCategory")
		.call()
		.context("GET https://api.warframestat.us/items?only=name,vaulted,type,category,productCategory")?;

	let items = res
		.body_mut()
		.read_json::<Vec<schema::vaulted::WarframeStatItem>>()
		.context("Decode vaulted items JSON")?;

	let mut vaulted = HashSet::new();
	let mut meta = HashMap::new();
	for item in items {
		let Some(name) = item.name else { continue };
		if item.vaulted.unwrap_or(false) {
			vaulted.insert(name.clone());
		}
		if item.item_type.is_some() || item.category.is_some() || item.product_category.is_some() {
			meta.insert(
				name,
				ItemMeta {
					item_type: item.item_type,
					category: item.category,
					product_category: item.product_category,
				},
			);
		}
	}
	Ok((vaulted, meta))
}
