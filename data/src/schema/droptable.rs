use std::collections::HashSet;

use anyhow::{Context, Result};

const URL: &str = "https://warframe.com/droptables";

/// Scrape the official droptables page to get the list of relics that are currently available
/// as drops.
///
/// The page is HTML, so we use a lightweight regex (same general approach as the original
/// WFBuddy codebase).
pub fn fetch_relic_names() -> Result<HashSet<String>> {
	let html = ureq::get(URL)
		.call()
		.context("GET droptables")?
		.body_mut()
		.read_to_string()
		.context("Read droptables HTML")?;

	let regex = regex::Regex::new(r"<tr><td>(?:</td><td>)?(?<name>[^<]+)</td>")
		.context("Compile droptables regex")?;
	let items = regex
		.captures_iter(&html)
		.filter_map(|cap| cap.name("name"))
		.map(|m| m.as_str().trim().to_string())
		.filter(|name| name.ends_with("Relic"))
		.collect::<HashSet<_>>();

	Ok(items)
}
