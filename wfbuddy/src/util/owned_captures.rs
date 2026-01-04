use std::collections::HashMap;

pub struct OwnedCaptures {
	indexed: Vec<Option<String>>,
	named: HashMap<String, String>,
}

impl OwnedCaptures {
	pub fn get(&self, index: usize) -> Option<&str> {
		self.indexed.get(index).map(|v| v.as_ref().map(|v| v.as_str())).flatten()
	}
	
	pub fn name(&self, name: &str) -> Option<&str> {
		self.named.get(name).map(|v| v.as_str())
	}
}

impl<'a, 'b> From<(&'a regex::Regex, regex::Captures<'b>)> for OwnedCaptures {
	fn from((regex, cap): (&'a regex::Regex, regex::Captures<'b>)) -> Self {
		let indexed = cap
			.iter()
			.map(|v| v.map(|v| v.as_str().to_owned()))
			.collect::<Vec<_>>();
		
		let named = regex
			.capture_names()
			.flatten()
			.filter_map(|name| cap.name(name).map(|v| (name.to_string(), v.as_str().to_owned())))
			.collect::<HashMap<_, _>>();
		
		Self{indexed, named}
	}
}

impl core::ops::Index<usize> for OwnedCaptures {
	type Output = str;
	
	fn index(&self, index: usize) -> &Self::Output {
		self.indexed[index].as_ref().unwrap()
	}
}

impl core::ops::Index<&str> for OwnedCaptures {
	type Output = str;
	
	fn index(&self, name: &str) -> &Self::Output {
		&self.named[name]
	}
}