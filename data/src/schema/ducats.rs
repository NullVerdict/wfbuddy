// i might be blind, but i didnt see a "get all item platinum values" api, so we use this
use serde::Deserialize;

pub const URL: &str = "https://api.warframe.market/v1/tools/ducats";

#[derive(Deserialize)]
pub struct Ducats {
	pub payload: Payload,
}

#[derive(Deserialize)]
pub struct Payload {
	pub previous_hour: Vec<Item>,
}

#[derive(Deserialize)]
pub struct Item {
	pub wa_price: f32,
	pub ducats: u32,
	pub item: String,
}