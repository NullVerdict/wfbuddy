#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Language {
	English,
}

impl Language {
	pub fn ocr_code(&self) -> &'static str {
		match self {
			Self::English => "latin",
		}
	}
	
	pub fn blueprint_name(&self, name: &str) -> String {
		match self {
			Language::English => format!("{name} Blueprint"),
		}
	}
}

pub struct Name<'a> {
	pub lang: crate::Language,
	pub text: &'a str,
}

impl<'a> Name<'a> {
	pub fn new(lang: crate::Language, s: &'a str) -> Self {
		Self {
			lang,
			text: s,
		}
	}
}


impl<'a> From<(crate::Language, &'a String)> for Name<'a> {
	fn from(val: (crate::Language, &'a String)) -> Self {
		Name {
			lang: val.0,
			text: val.1.as_str(),
		}
	}
}

impl<'a> From<(crate::Language, &'a str)> for Name<'a> {
	fn from(val: (crate::Language, &'a str)) -> Self {
		Name {
			lang: val.0,
			text: val.1,
		}
	}
}

pub enum Vaulted {
	Unvaulted,
	Vaulted,
	Resurgence,
}