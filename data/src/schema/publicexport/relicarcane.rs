use serde::Deserialize;

#[derive(Deserialize)]
pub struct RelicArcane {
	#[serde(rename = "ExportRelicArcane")]
	pub items: Vec<Item>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Item {
	Relic(Relic),
	Arcane(Arcane),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relic {
	pub unique_name: String,
	pub name: String,
	pub relic_rewards: Vec<RelicReward>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelicReward {
	pub reward_name: String,
	pub item_count: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Arcane {
	pub unique_name: String,
	pub name: String,
}
