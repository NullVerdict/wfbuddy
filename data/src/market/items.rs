pub const URL: &str = "https://api.warframe.market/v2/items";

#[derive(serde::Deserialize)]
pub struct Items {
	pub data: Vec<Item>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
	pub id: String,
	pub game_ref: String,
	// pub ducats: Option<u32>,
	// pub i18n: Locale,
}

// #[derive(serde::Deserialize)]
// pub struct Locale {
// 	pub en: Info,
// }
// 
// #[derive(serde::Deserialize)]
// pub struct Info {
// 	pub name: String,
// }