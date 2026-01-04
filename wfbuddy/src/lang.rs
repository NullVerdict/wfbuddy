#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Language {
	English,
}

impl Language {
	pub fn ocr_code(&self) -> &'static str {
		match self {
			Self::English => "latin",
		}
	}
}