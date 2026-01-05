use serde::Deserialize;

/// Minimal item representation from https://api.warframestat.us/items
///
/// We request `only=name,vaulted,type,category,productCategory`.
#[derive(Debug, Clone, Deserialize)]
pub struct WarframeStatItem {
	pub name: Option<String>,
	pub vaulted: Option<bool>,
	#[serde(rename = "type")]
	pub item_type: Option<String>,
	pub category: Option<String>,
	#[serde(rename = "productCategory")]
	pub product_category: Option<String>,
}
