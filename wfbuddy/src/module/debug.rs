pub struct Debug {
	uniform: crate::Uniform,
}

impl Debug {
	pub fn new(uniform: crate::Uniform) -> Self {
		Self {
			uniform,
		}
	}
}

impl super::Module for Debug {
	fn name(&self) -> &'static str {
		"Debug"
	}
	
	fn ui(&mut self, ui: &mut egui::Ui) {
		if ui.button("Party Header Text").clicked() {
			let Some(image) = crate::capture::capture() else {return};
			println!("{}", self.uniform.ie.util_party_header_text(image.as_image()));
		}
	}
}