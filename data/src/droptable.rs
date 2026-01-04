use std::collections::HashSet;

const URL: &str = "https://warframe.com/droptables";

pub struct Droptable {
	items: HashSet<crate::Id>,
}

impl Droptable {
	pub fn downloaded(idman: &mut crate::IdManager) -> Result<Self, anyhow::Error> {
		let html = ureq::get(URL)
			.call()?
			.body_mut()
			.read_to_string()?;
		
		let regex = regex::Regex::new(r"<tr><td>(?:</td><td>)?(?<name>[^<]+)</td>")?;
		let caps = regex.captures_iter(&html);
		let items = caps
			.filter_map(|cap| {
				cap.name("name")
					.filter(|name| name.as_str().ends_with("Relic"))
					.and_then(|name| idman.get_id_from_en(name.as_str()))
			})
			.collect::<HashSet<_>>();
		
			Ok(Self{items})
	}
	
	pub fn contains_id(&self, id: &crate::Id) -> bool {
		self.items.contains(id)
	}
}