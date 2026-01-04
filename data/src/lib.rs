use std::collections::{HashMap, HashSet};

mod structs;
pub use structs::*;
mod id;
pub use id::*;
mod droptable;
mod publicexport;
mod market;

// TODO: maybe function to get platinum value, which calls api if its old or only
// has value from ducanator, and updates it

pub struct Data {
	pub id_manager: IdManager,
	
	pub platinum_values: HashMap<Id, f32>,
	pub ducat_values: HashMap<Id, u32>,
	pub relic_items: HashSet<Id>,
	pub vaulted_items: HashSet<Id>,
}

impl Data {
	pub fn populated(lang: Language) -> Result<Self, anyhow::Error> {
		let mut idman = id::IdManager::new();
		
		let publicexport = publicexport::PublicExport::new(lang)?;
		
		// add all required locale
		let resources = get::<publicexport::resources::Resources>(&publicexport.resources_url)?;
		for v in &resources.resources {
			idman.add_locale((lang, &v.name), &v.unique_name);
		}
		
		let warframes = get::<publicexport::warframes::Warframes>(&publicexport.warframes_url)?;
		for v in &warframes.warframes {
			idman.add_locale((lang, &v.name), &v.unique_name);
		}
		
		let weapons = get::<publicexport::weapons::Weapons>(&publicexport.weapons_url)?;
		for v in &weapons.weapons {
			idman.add_locale((lang, &v.name), &v.unique_name);
		}
		
		let sentinels = get::<publicexport::sentinels::Sentinels>(&publicexport.sentinels_url)?;
		for v in &sentinels.sentinels {
			idman.add_locale((lang, &v.name), &v.unique_name);
		}
		
		// blueprint locale
		let recipes = get::<publicexport::recipes::Recipes>(&publicexport.recipes_url)?;
		for recipe in &recipes.recipes {
			let Some(result_locale) = idman.get_locale_from_gamename(lang, &recipe.result_type) else {println!("[BlueprintLocale] No id found for {}", recipe.result_type); continue};
			let locale = lang.blueprint_name(result_locale);
			println!("[BlueprintLocale] Register {} = {locale}", recipe.unique_name);
			idman.add_locale((lang, &locale), &recipe.unique_name);
		}
		
		//
		let relicarcane = get::<publicexport::relicarcane::RelicArcane>(&publicexport.relic_arcane_url)?;
		let mut relic_items = HashSet::new();
		
		for v in &relicarcane.items {
			let publicexport::relicarcane::Item::Relic(relic) = v else {continue};
			idman.add_locale((lang, &relic.name), &relic.unique_name);
			for reward in &relic.relic_rewards {
				let Some(id) = idman.get_id_from_gamename(&reward.reward_name) else {println!("[RelicItem] No id found for {}", reward.reward_name); continue};
				relic_items.insert(id);
			}
		}
		
		//
		let droptable = droptable::Droptable::downloaded(&mut idman)?;
		let mut vaulted_items = HashSet::new();
		let mut item_relics = HashMap::new();
		
		for v in relicarcane.items {
			let publicexport::relicarcane::Item::Relic(relic) = v else {continue};
			let relic_id = idman.get_id_from_gamename(&relic.unique_name).unwrap();
			if !droptable.contains_id(&relic_id) {
				vaulted_items.insert(relic_id);
			}
			
			for reward in &relic.relic_rewards {
				let Some(id) = idman.get_id_from_gamename(&reward.reward_name) else {continue};
				item_relics.entry(id).or_insert_with(|| Vec::new()).push(relic_id);
			}
		}
		
		'o: for (item, relics) in item_relics {
			for relic in relics {
				if !vaulted_items.contains(&relic) {
					continue 'o;
				}
			}
			
			vaulted_items.insert(item);
		}
		
		// droptable is english localized names, so we gotta add enlgish relic locales to translate it
		if lang != Language::English {
			let publicexport = publicexport::PublicExport::new(Language::English)?;
			let relicarcane = get::<publicexport::relicarcane::RelicArcane>(&publicexport.relic_arcane_url)?;
			for v in relicarcane.items {
				let publicexport::relicarcane::Item::Relic(relic) = v else {continue};
				idman.add_locale_en(relic.name, relic.unique_name);
			}
		}
		
		//
		let market_items = get::<market::items::Items>(market::items::URL)?;
		let market_ducats = get::<market::ducats::Ducats>(market::ducats::URL)?;
		let mut market_id_map = HashMap::new();
		for v in &market_items.data {
			let Some(id) = idman.get_id_from_gamename(&v.game_ref) else {println!("[WFMarket] No id found for {}", v.game_ref); continue};
			market_id_map.insert(v.id.clone(), id);
		}
		
		let mut s = Self {
			platinum_values: market_ducats.payload.previous_hour
				.iter()
				.filter_map(|v| market_id_map.get(&v.item).map(|id| (*id, v.wa_price)))
				.collect(),
			
			ducat_values: market_ducats.payload.previous_hour
				.iter()
				.filter_map(|v| market_id_map.get(&v.item).map(|id| (*id, v.ducats)))
				.collect(),
			
			relic_items,
			vaulted_items,
			id_manager: idman,
		};
		
		s.platinum_values.insert(s.id_manager.get_id_from_gamename("/Lotus/StoreItems/Types/Recipes/Components/FormaBlueprint").unwrap(), (350.0f32 / 3.0).floor() * 0.1);
		
		// println!("{:#?}", s.vaulted_items);
		// for id in &s.vaulted_items {
		// 	println!("vaulted: {}", s.id_manager.get_en_from_id(*id).unwrap());
		// }
		
		Ok(s)
	}
	
	/// Attempts to find the closest item name from a dirty ocr string
	pub fn find_item_name<'a, 'b>(&'a self, name: impl Into<Name<'b>>) -> &'a str {
		self.id_manager.get_closest_match(name)
	}
}

fn get<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, anyhow::Error> {
	Ok(ureq::get(url)
		.call()?
		.body_mut()
		.read_json::<T>()?)
}