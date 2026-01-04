use serde::Deserialize;

pub const URL: &str = "https://api.warframe.market/v2/items";

#[derive(Deserialize)]
pub struct Items {
	pub data: Vec<Item>,
}

#[derive(Deserialize)]
pub struct Item {
	pub id: String,
	#[serde(rename = "gameRef", default)]
	pub game_ref: Option<String>,
	// pub ducats: Option<u32>,
	pub i18n: Locale,
}

#[derive(Deserialize)]
pub struct Locale {
	pub en: Info,
}

#[derive(Deserialize)]
pub struct Info {
	pub name: String,
}