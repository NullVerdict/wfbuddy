use std::collections::HashSet;

use anyhow::{Context, Result};

const URL: &str = "https://warframe.com/droptables";

/// Downloads the official drop tables page and extracts the names of currently-dropping relics.
///
/// We keep this intentionally simple (best-effort): if the page layout changes, we just won't
/// populate vaulted detection, but the rest of the app still works.
pub fn downloaded_relic_names() -> Result<HashSet<String>> {
	let resp = ureq::get(URL).call().context("GET droptables")?;
	let html = resp.into_string().context("Read droptables HTML")?;

	// This is the same basic approach as the original project: match the first <td> in a row.
	// Example match: <tr><td>Lith A1 Relic</td>
	let regex = regex::Regex::new(r"<tr><td>(?:</td><td>)?(?<name>[^<]+)</td>")
		.context("Compile droptables regex")?;

	let mut items = HashSet::new();
	for cap in regex.captures_iter(&html) {
		let Some(name) = cap.name("name") else { continue };
		let name = name.as_str().trim();
		if name.ends_with("Relic") {
			items.insert(name.to_string());
		}
	}

	Ok(items)
}
