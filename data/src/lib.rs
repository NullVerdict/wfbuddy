use std::{
	collections::{HashMap, HashSet},
	fs::File,
	io::{BufReader, BufWriter, Write},
	path::PathBuf,
};

use anyhow::{Context, Result};

mod schema;

/// A single tradable relic reward item with the values we care about.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ItemEntry {
	pub platinum: f32,
	pub ducats: u32,
	pub vaulted: bool,
}

/// Persistent dataset used for OCR matching + overlay enrichment.
///
/// Notes:
/// - We keep a `relic_items` set for fast membership checks + Levenshtein search.
/// - Values live in the `items` map to avoid syncing multiple parallel HashMaps.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Data {
	pub items: HashMap<String, ItemEntry>,
	pub relic_items: HashSet<String>,
}

/// Old cache representation (pre-typed `ItemEntry`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct DataV1 {
	platinum_values: HashMap<String, f32>,
	ducat_values: HashMap<String, u32>,
	relic_items: HashSet<String>,
	vaulted_items: HashSet<String>,
}

/// Current cache representation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct DataV2 {
	items: HashMap<String, ItemEntry>,
	relic_items: HashSet<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
enum DataRepr {
	V2(DataV2),
	V1(DataV1),
}

impl<'de> serde::Deserialize<'de> for Data {
	fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		match DataRepr::deserialize(deserializer)? {
			DataRepr::V2(v2) => Ok(Self {
				items: v2.items,
				relic_items: v2.relic_items,
			}),
			DataRepr::V1(v1) => {
				// Best-effort upgrade path for older caches.
				let mut items = HashMap::new();
				for name in v1.relic_items.iter() {
					let platinum = v1.platinum_values.get(name).copied().unwrap_or_default();
					let ducats = v1.ducat_values.get(name).copied().unwrap_or_default();
					let vaulted = v1.vaulted_items.contains(name);
					items.insert(
						name.clone(),
						ItemEntry {
							platinum,
							ducats,
							vaulted,
						},
					);
				}
				Ok(Self {
					items,
					relic_items: v1.relic_items,
				})
			}
		}
	}
}

impl Default for Data {
	fn default() -> Self {
		let mut s = Self {
			items: HashMap::new(),
			relic_items: HashSet::new(),
		};

		// Keep Forma in the dataset so the UI doesn’t special-case “missing data”.
		// (The value is just a rough ducat-to-plat approximation; adjust if you want.)
		s.items.insert(
			"Forma Blueprint".to_string(),
			ItemEntry {
				platinum: (350.0f32 / 3.0).floor() * 0.1,
				ducats: 0,
				vaulted: false,
			},
		);
		s.relic_items.insert("Forma Blueprint".to_string());

		s.items.insert(
			"2 X Forma Blueprint".to_string(),
			ItemEntry {
				platinum: (350.0f32 / 3.0).floor() * 0.2,
				ducats: 0,
				vaulted: false,
			},
		);
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
		let mut res = ureq::get(schema::items::URL).call().context("GET items")?;
		let items = res
			.body_mut()
			.read_json::<schema::items::Items>()
			.context("Decode items JSON")?;

		let mut res = ureq::get(schema::ducats::URL).call().context("GET ducats")?;
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
			items: HashMap::new(),
			relic_items: HashSet::new(),
		};

		// Populate vaulted status using WarframeStat's static processing dataset.
		// We intentionally keep this best-effort: if the endpoint is unavailable
		// we still want the app to work.
		let vaulted_items = fetch_vaulted_items().unwrap_or_default();

		for v in &ducats.payload.previous_hour {
			let name = name_map
				.get(&v.item)
				.with_context(|| format!("Missing name for item id {}", v.item))?
				.clone();

			// Warframe Market also lists tradable "Prime Set" items.
			// Void Relics, however, only reward individual Prime parts / blueprints
			// (not assembled sets), so we keep sets out of the OCR matching pool.
			// This also prevents OCR failures like "SET" being matched to the
			// shortest Prime Set name.
			if is_prime_set_name(&name) {
				continue;
			}

			s.items.insert(
				name.clone(),
				ItemEntry {
					platinum: v.wa_price,
					ducats: v.ducats,
					vaulted: vaulted_items.contains(&name),
				},
			);
			s.relic_items.insert(name);
		}

		// Ensure Forma entries exist even if the remote feed changes.
		let mut out = Self::default();
		out.items.extend(s.items);
		out.relic_items.extend(s.relic_items);
		Ok(out)
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

	pub fn platinum(&self, name: &str) -> f32 {
		self.items.get(name).map(|v| v.platinum).unwrap_or_default()
	}

	pub fn ducats(&self, name: &str) -> u32 {
		self.items.get(name).map(|v| v.ducats).unwrap_or_default()
	}

	pub fn is_vaulted(&self, name: &str) -> bool {
		self.items.get(name).map(|v| v.vaulted).unwrap_or(false)
	}

	/// Attempts to find the closest item name from a dirty ocr string.
	pub fn find_item_name(&self, name: &str) -> String {
		let name = name.trim_ascii();
		// If OCR completely fails, it sometimes returns just "SET".
		// Sets can't appear as relic rewards, so don't try to match this to any item.
		if name.eq_ignore_ascii_case("set") {
			return "(unreadable)".to_string();
		}
		// Also avoid matching strings that end with " SET".
		if name.to_ascii_lowercase().ends_with(" set") {
			return "(unreadable)".to_string();
		}
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

fn is_prime_set_name(name: &str) -> bool {
	// Warframe Market uses the English string "<Something> Prime Set".
	// We only filter the canonical suffix to avoid false positives.
	name.trim_end().ends_with(" Set")
}

/// Best-effort fetch of vaulted status from WarframeStat.
///
/// We use the processing dataset at `/items` and request only the fields we
/// need to keep the payload small.
fn fetch_vaulted_items() -> Result<HashSet<String>> {
	let mut res = ureq::get("https://api.warframestat.us/items")
		.query("only", "name,vaulted")
		.call()
		.context("GET https://api.warframestat.us/items?only=name,vaulted")?;

	let items = res
		.body_mut()
		.read_json::<Vec<schema::vaulted::WarframeStatItem>>()
		.context("Decode vaulted items JSON")?;

	let mut set = HashSet::new();
	for item in items {
		if item.vaulted.unwrap_or(false) && let Some(name) = item.name {
			// Keep vaulted sets out of the reward dataset as well.
			if !is_prime_set_name(&name) {
				set.insert(name);
			}
		}
	}
	Ok(set)
}
