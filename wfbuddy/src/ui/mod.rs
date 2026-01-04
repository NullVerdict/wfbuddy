use crate::{iepol::IePol, module::{self, Module}};
use std::time::Duration;

mod ext;
pub use ext::UiExt;
mod settings;

/// A tiny fallback UI shown when we fail to initialize the real app (e.g. OCR models missing).
///
/// This avoids crashing or silently exiting when the binary is launched from Explorer.
pub struct ErrorApp {
	message: String,
}

impl ErrorApp {
	pub fn new(err: anyhow::Error) -> Self {
		Self {
			message: format!(
				"WFBuddy failed to start.\n\n{:#}\n\n\
Fix:\n  • Ensure the 'ocr/' folder is next to the executable\n  • or set WFBUDDY_ASSETS_DIR to the assets directory",
				err
			),
		}
	}
}

impl eframe::App for ErrorApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui| {
			ui.heading("Startup error");
			ui.separator();
			egui::ScrollArea::vertical().show(ui, |ui| {
				ui.add(
					egui::TextEdit::multiline(&mut self.message)
						.font(egui::TextStyle::Monospace)
						.desired_rows(20)
						.lock_focus(true)
						.cursor_at_end(true),
				);
			});
		});
	}
}

pub struct WFBuddy {
	modules: Vec<Box<dyn Module>>,
	uniform: crate::Uniform,
	tab: Tab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
	Home,
	Settings,
	Module(usize),
}

impl WFBuddy {
	pub fn try_new(_cc: &eframe::CreationContext) -> anyhow::Result<Self> {
		// Lock the config once.
		let cfg = crate::config_read().clone();
		let lang = cfg.client_language.ocr_code();
		let assets = crate::util::resolve_ocr_assets(lang)?;
		let ie = std::sync::Arc::new(ie::Ie::try_new(
			cfg.theme,
			assets.detection,
			assets.recognition,
			assets.charset,
		)?);
		let uniform = std::sync::Arc::new(crate::UniformData {
			iepol: IePol::new(ie.clone()),
			data: data::Data::try_populated().unwrap_or_else(|err| {
				log::warn!("Failed to load market data: {err:#}");
				data::Data::default()
			}),
			ie,
		});
		
		Ok(Self {
			modules: vec![
				Box::new(module::RelicReward::new(uniform.clone())),
				Box::new(module::Debug::new(uniform.clone())),
			],
			uniform,
			tab: Tab::Home,
		})
	}
	
	fn ui(&mut self, ui: &mut egui::Ui) {
		ui.label(format!("Seconds till next poll: {}", self.uniform.iepol.secs_till_next_poll()));
		
		ui.horizontal(|ui| {
			if ui.selectable_label(self.tab == Tab::Home, "Home").clicked() {
				self.tab = Tab::Home;
			}
			if ui.selectable_label(self.tab == Tab::Settings, "Settings").clicked() {
				self.tab = Tab::Settings;
			}
			for (i, module) in self.modules.iter_mut().enumerate() {
				if ui.selectable_label(self.tab == Tab::Module(i), module.name()).clicked() {
					self.tab = Tab::Module(i);
				}
			}
		});
		
		ui.separator();
		
		match self.tab {
			Tab::Home => {
				for module in &mut self.modules {
					if module.ui_important(ui) {
						ui.separator();
					}
				}
			}
			Tab::Settings => settings::ui(ui, &mut self.modules),
			Tab::Module(i) => {
				if let Some(module) = self.modules.get_mut(i) {
					module.ui(ui);
				}
			}
		}
	}
}

impl eframe::App for WFBuddy {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		// Drive background processing.
		for module in &mut self.modules {
			module.tick();
		}
		
		egui::CentralPanel::default().show(ctx, |ui| self.ui(ui));
		
		// Avoid pegging a CPU core by repainting every frame.
		// We repaint often enough to keep timers (poll countdown) responsive.
		let secs = self.uniform.iepol.secs_till_next_poll();
		let after = Duration::from_secs_f32(secs.clamp(0.05, 0.5));
		ctx.request_repaint_after(after);
	}
}