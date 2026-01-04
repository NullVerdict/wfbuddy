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

impl<'a> Into<Name<'a>> for (crate::Language, &'a String) {
	fn into(self) -> Name<'a> {
		Name {
			lang: self.0,
			text: self.1,
		}
	}
}

impl<'a> Into<Name<'a>> for (crate::Language, &'a str) {
	fn into(self) -> Name<'a> {
		Name {
			lang: self.0,
			text: self.1,
		}
	}
}

pub enum Vaulted {
	Unvaulted,
	Vaulted,
	Resurgence,
}