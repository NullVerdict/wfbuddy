use serde::Deserialize;

/// Minimal item representation from https://api.warframestat.us/items
///
/// We request `only=name,vaulted` so these are the only fields we depend on.
#[derive(Debug, Clone, Deserialize)]
pub struct WarframeStatItem {
	pub name: Option<String>,
	pub vaulted: Option<bool>,
}
