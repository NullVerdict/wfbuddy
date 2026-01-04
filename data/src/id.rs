use std::collections::HashMap;

// Index of the gamename string
pub type Id = lasso::Spur;

pub struct IdManager {
	strings: lasso::Rodeo,
	
	map_en_gamename: HashMap<lasso::Spur, lasso::Spur>,
	map_gamename_en: HashMap<lasso::Spur, lasso::Spur>,
}

impl Default for IdManager {
	fn default() -> Self {
		Self::new()
	}
}

impl IdManager {
	pub fn new() -> Self {
		Self {
			strings: lasso::Rodeo::new(),
			
			map_en_gamename: HashMap::new(),
			map_gamename_en: HashMap::new(),
		}
	}
	
	pub fn add_locale<'a>(&mut self, locale_name: impl Into<super::Name<'a>>, gamename: impl Into<String>) {
		let locale_name = locale_name.into();
		match locale_name.lang {
			crate::Language::English => self.add_locale_en(locale_name.text, gamename),
		}
	}
	
	pub fn add_locale_en(&mut self, locale_name: impl Into<String>, gamename: impl Into<String>) {
		let gamename = gamename.into();
		let gamename_key = self.strings.get_or_intern(convert_gamename(gamename));
		let locale_name_key = self.strings.get_or_intern(convert_en(locale_name));
		self.map_en_gamename.insert(locale_name_key, gamename_key);
		self.map_gamename_en.insert(gamename_key, locale_name_key);
	}
	
	pub fn get_id_from_gamename(&self, name: &str) -> Option<Id> {
		self.strings.get(convert_gamename(name))
	}
	
	pub fn get_id_from_locale<'a>(&self, locale_name: impl Into<super::Name<'a>>) -> Option<Id> {
		let locale_name = locale_name.into();
		match locale_name.lang {
			crate::Language::English => self.get_id_from_en(locale_name.text),
		}
	}
	
	pub fn get_id_from_en(&self, name: &str) -> Option<Id> {
		self
			.map_en_gamename
			.get(&self.strings.get(convert_en(name))?)
			.copied()
	}
	
	pub fn get_gamename_from_id(&self, id: Id) -> Option<&str> {
		self.strings.try_resolve(&id)
	}
	
	pub fn get_locale_from_gamename(&self, lang: crate::Language, name: &str) -> Option<&str> {
		let id = self.get_id_from_gamename(name)?;
		self.get_locale_from_id(lang, id)
	}
	
	pub fn get_locale_from_id(&self, lang: crate::Language, id: Id) -> Option<&str> {
		match lang {
			crate::Language::English => self.get_en_from_id(id),
		}
	}
	
	pub fn get_en_from_id(&self, id: Id) -> Option<&str> {
		self.strings.try_resolve(self.map_gamename_en.get(&id)?)
	}
	
	pub fn get_closest_match<'a>(&self, locale_name: impl Into<super::Name<'a>>) -> &str {
		let locale_name = locale_name.into();
		match locale_name.lang {
			crate::Language::English => self.get_closest_match_en(locale_name.text),
		}
	}
	
	pub fn get_closest_match_en<'a>(&'a self, name: &str) -> &'a str {
		let check_name = convert_en(name);
		if let Some(id) = self.get_id_from_en(&check_name) {
			return self.get_en_from_id(id).unwrap();
		}
		
		let mut min_name = "";
		let mut min = usize::MAX;
		for (id, _) in self.map_en_gamename.iter() {
			let item_name = self.strings.resolve(id);
			let lev = levenshtein::levenshtein(name, item_name);
			if lev < min {
				min_name = item_name;
				min = lev;
			}
		}
		
		min_name
	}
}

fn convert_gamename(s: impl Into<String>) -> String {
	let s = s.into();
	s.replace("/StoreItems/", "/")
}

// since we return the locale, we wont adjust it for now
// TODO: find solution
fn convert_en(s: impl Into<String>) -> String {
	s.into()
	// let mut s = s.into();
	// s.make_ascii_lowercase();
	// s.retain(|v| !v.is_ascii_whitespace());
	// s
}