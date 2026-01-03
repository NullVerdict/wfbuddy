use crate::{iepol::IePol, module::{self, Module}};

mod ext;
pub use ext::UiExt;
mod settings;

pub struct WFBuddy {
	modules: Vec<Box<dyn Module>>,
	uniform: crate::Uniform,
	tab: &'static str,
}

impl WFBuddy {
	pub fn new(_cc: &eframe::CreationContext) -> Self {
		let lang = crate::config().client_language;
		let ocr_code = lang.ocr_code();
		let ie = std::sync::Arc::new(ie::Ie::new(crate::config().theme, "ocr/detection.mnn", format!("ocr/{ocr_code}_recognition.mnn"), format!("ocr/{ocr_code}_charset.txt")));
		let uniform = std::sync::Arc::new(crate::UniformData {
			iepol: IePol::new(ie.clone()),
			data: data::Data::populated(lang).unwrap(),
			ie,
		});
		
		Self {
			modules: vec![
				Box::new(module::RelicReward::new(uniform.clone())),
				Box::new(module::Debug::new(uniform.clone())),
			],
			uniform,
			tab: "Home",
		}
	}
	
	fn ui(&mut self, ui: &mut egui::Ui) {
		ui.label(format!("Seconds till next poll: {}", self.uniform.iepol.secs_till_next_poll()));
		
		ui.horizontal(|ui| {
			if ui.selectable_label(self.tab == "Home", "Home").clicked() {
				self.tab = "Home";
			}
			
			if ui.selectable_label(self.tab == "Settings", "Settings").clicked() {
				self.tab = "Settings";
			}
			
			for module in &mut self.modules {
				if ui.selectable_label(self.tab == module.name(), module.name()).clicked() {
					self.tab = module.name();
				}
			}
		});
		
		ui.separator();
		
		match self.tab {
			"Home" => {
				for module in &mut self.modules {
					if module.ui_important(ui) {
						ui.separator();
					}
				}
			}
			
			"Settings" => {
				settings::ui(ui, &mut self.modules);
			}
			
			tab => {
				for module in &mut self.modules {
					if module.name() == tab {
						module.ui(ui);
						break;
					}
				}
			}
		}
	}
}

impl eframe::App for WFBuddy {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		// println!("tick");
		for module in &mut self.modules {
			module.tick();
		}
		
		egui::CentralPanel::default().show(&ctx, |ui| self.ui(ui));
		
		// https://github.com/emilk/egui/issues/5113
		// https://github.com/emilk/egui/pull/7775
		ctx.request_repaint();
	}
}