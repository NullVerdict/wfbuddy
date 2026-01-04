#![allow(dead_code)]

#[derive(serde::Deserialize)]
pub struct RelicArcane {
	#[serde(rename = "ExportRelicArcane")]
	pub items: Vec<Item>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum Item {
	Relic(Relic),
	Arcane(Arcane),
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relic {
	pub unique_name: String,
	pub name: String,
	pub relic_rewards: Vec<RelicReward>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelicReward {
	pub reward_name: String,
	pub item_count: i32,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Arcane {
	pub unique_name: String,
	pub name: String,
}

// {
// 	"uniqueName": "/Lotus/Types/Game/Projections/T2VoidProjectionWukongPrimeAPlatinum",
// 	"name": "Meso A2 Relic",
// 	"codexSecret": false,
// 	"description": "An artifact containing Orokin secrets. It can only be opened through the power of the Void.",
// 	"relicRewards": [
// 		{
// 			"rewardName": "/Lotus/StoreItems/Types/Recipes/Weapons/WeaponParts/AkboltoPrimeReceiver",
// 			"rarity": "RARE",
// 			"tier": 0,
// 			"itemCount": 1
// 		},
// 		{
// 			"rewardName": "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/EquinoxPrimeBlueprint",
// 			"rarity": "UNCOMMON",
// 			"tier": 0,
// 			"itemCount": 1
// 		},
// 		{
// 			"rewardName": "/Lotus/StoreItems/Types/Recipes/WarframeRecipes/ZephyrPrimeChassisBlueprint",
// 			"rarity": "UNCOMMON",
// 			"tier": 0,
// 			"itemCount": 1
// 		},
// 		{
// 			"rewardName": "/Lotus/StoreItems/Types/Recipes/Weapons/WeaponParts/ZhugePrimeStock",
// 			"rarity": "COMMON",
// 			"tier": 0,
// 			"itemCount": 1
// 		},
// 		{
// 			"rewardName": "/Lotus/StoreItems/Types/Recipes/Weapons/TipedoPrimeBlueprint",
// 			"rarity": "COMMON",
// 			"tier": 0,
// 			"itemCount": 1
// 		},
// 		{
// 			"rewardName": "/Lotus/StoreItems/Types/Recipes/Components/FormaBlueprint",
// 			"rarity": "COMMON",
// 			"tier": 0,
// 			"itemCount": 1
// 		}
// 	]
// },