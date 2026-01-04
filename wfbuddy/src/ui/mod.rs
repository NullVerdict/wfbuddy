use std::sync::Arc;
use std::{collections::{HashMap, HashSet}, path::PathBuf};

mod ext;
pub use ext::UiExt;

mod settings;

use crate::overlay::OverlayController;

pub struct WFBuddy {
	uniform: crate::Uniform,
	modules: Vec<Box<dyn crate::module::Module>>,

	tab: Tab,

	overlay: Option<OverlayController>,
	overlay_show_settings: bool,
}

impl WFBuddy {
	pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
		let config = crate::config();

		// Apply UI zoom (in addition to OS DPI scaling).
		cc.egui_ctx.set_zoom_factor(config.ui_zoom_factor);

		// Initialize overlay controller if requested.
		let overlay = matches!(config.ui_mode, crate::config::UiMode::Overlay)
			.then(|| OverlayController::new(config.overlay_click_through));

		// OCR model assets live in the workspace `ocr/` directory (sibling to the `wfbuddy/` crate).
		fn ocr_path(file: &str) -> PathBuf {
			let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
			p.pop(); // workspace root
			p.push("ocr");
			p.push(file);
			p
		}

		let ie = Arc::new(ie::Ie::new(
			config.theme,
			ocr_path("detection.mnn"),
			ocr_path("latin_recognition.mnn"),
			ocr_path("latin_charset.txt"),
		));

		let data = data::Data::populated(config.client_language).unwrap_or_else(|err| {
			eprintln!("[wfbuddy] Failed to load WF data: {err:#}");
			data::Data {
				id_manager: data::IdManager::new(),
				platinum_values: HashMap::new(),
				ducat_values: HashMap::new(),
				relic_items: HashSet::new(),
				vaulted_items: HashSet::new(),
			}
		});

		let uniform = Arc::new(crate::UniformData {
			iepol: crate::iepol::IePol::new(ie.clone()),
			data,
			ie,
		});

		let modules: Vec<Box<dyn crate::module::Module>> = vec![
			Box::new(crate::module::RelicReward::new(uniform.clone())),
			Box::new(crate::module::Debug::new(uniform.clone())),
		];

		drop(config);

		Self {
			uniform,
			modules,
			tab: Tab::Home,
			overlay,
			overlay_show_settings: false,
		}
	}

	fn ui_home(&mut self, ui: &mut egui::Ui) {
		for module in &mut self.modules {
			module.ui_important(ui);
		}
	}

	fn ui_settings(&mut self, ui: &mut egui::Ui) {
		settings::ui(ui, &mut self.modules);
	}

	fn is_overlay(&self) -> bool {
		self.overlay.is_some()
	}
}

impl eframe::App for WFBuddy {
	fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
		// Apply overlay behavior (follow + click-through toggle).
		if let Some(overlay) = &mut self.overlay {
			let config = crate::config();
			overlay.set_click_through(config.overlay_click_through);
			let app_id = config.app_id.clone();
			drop(config);

			overlay.update(ctx, frame, &app_id);
		}

		// Apply zoom changes if user updated config while running.
		let zoom = crate::config().ui_zoom_factor;
		if (ctx.zoom_factor() - zoom).abs() > f32::EPSILON {
			ctx.set_zoom_factor(zoom);
		}

		if self.is_overlay() {
			egui::Area::new(egui::Id::new("overlay_root"))
				.fixed_pos(egui::pos2(12.0, 12.0))
				.show(ctx, |ui| {
					egui::Frame::default()
						.fill(egui::Color32::from_black_alpha(128))
					.corner_radius(6)
						.inner_margin(egui::Margin::same(8))
						.show(ui, |ui| {
							ui.horizontal(|ui| {
								ui.label(crate::tr!("app-title"));
								ui.add_space(6.0);

								if ui.small_button("⚙").clicked() {
									self.overlay_show_settings = !self.overlay_show_settings;
								}

								let click_through = crate::config().overlay_click_through;
								ui.add_space(6.0);
								ui.label(if click_through {
									crate::tr!("overlay-status-clickthrough")
								} else {
									crate::tr!("overlay-status-interactive")
								});
							});

							ui.separator();
							self.ui_home(ui);
						});
				});

			if self.overlay_show_settings {
				egui::Window::new(crate::tr!("tab-settings"))
					.default_size([420.0, 520.0])
					.show(ctx, |ui| self.ui_settings(ui));
			}
		} else {
			egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
				ui.horizontal(|ui| {
					ui.selectable_value(&mut self.tab, Tab::Home, crate::tr!("tab-home"));
					ui.selectable_value(&mut self.tab, Tab::Settings, crate::tr!("tab-settings"));
				});
			});

			egui::CentralPanel::default().show(ctx, |ui| match self.tab {
				Tab::Home => self.ui_home(ui),
				Tab::Settings => self.ui_settings(ui),
			});
		}

		for module in &mut self.modules {
			module.tick();
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
	Home,
	Settings,
}
