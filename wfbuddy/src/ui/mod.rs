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

		// Locate OCR model assets at *runtime*.
		//
		// IMPORTANT: `env!("CARGO_MANIFEST_DIR")` is a compile-time path (it points to the build machine),
		// so it must not be used for runtime asset resolution in release builds.
		fn locate_ocr_dir() -> Option<PathBuf> {
			let mut candidates: Vec<PathBuf> = Vec::new();

			// 1) Beside the executable: <exe-dir>/ocr
			if let Ok(exe) = std::env::current_exe() {
				if let Some(dir) = exe.parent() {
					candidates.push(dir.join("ocr"));
					// 2) One directory above (common if exe is in ./bin/)
					if let Some(parent) = dir.parent() {
						candidates.push(parent.join("ocr"));
					}
				}
			}

			// 3) Current working directory: ./ocr
			if let Ok(cwd) = std::env::current_dir() {
				candidates.push(cwd.join("ocr"));
			}

			// 4) Config directory: <config>/WFBuddy/ocr
			if let Some(cfg) = dirs::config_dir() {
				candidates.push(cfg.join("WFBuddy").join("ocr"));
			}

			candidates.into_iter().find(|p| {
				p.join("detection.mnn").is_file()
					&& p.join("latin_recognition.mnn").is_file()
					&& p.join("latin_charset.txt").is_file()
			})
		}

		let ie = {
			let ocr_dir = locate_ocr_dir();
			let (det, rec, charset) = if let Some(dir) = &ocr_dir {
				(
					dir.join("detection.mnn"),
					dir.join("latin_recognition.mnn"),
					dir.join("latin_charset.txt"),
				)
			} else {
				// Pass obviously-invalid paths; the `ie` crate will gracefully disable OCR and surface the error.
				(
					PathBuf::from("missing-ocr-detection.mnn"),
					PathBuf::from("missing-ocr-recognition.mnn"),
					PathBuf::from("missing-ocr-charset.txt"),
				)
			};

			Arc::new(ie::Ie::new(config.theme, det, rec, charset))
		};

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
			if !self.uniform.ie.ocr_available() {
				ui.group(|ui| {
					ui.label(egui::RichText::new(crate::tr!("ocr-missing")).strong());
					if let Some(err) = self.uniform.ie.ocr_init_error() {
						ui.add_space(4.0);
						ui.small(err);
					}
				});
				ui.add_space(6.0);
			}
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
						.corner_radius(egui::CornerRadius::same(6))
						.inner_margin(egui::Margin::same(8))
						.show(ui, |ui| {
							ui.horizontal(|ui| {
								ui.label(crate::tr!("app-title"));
								ui.add_space(6.0);

								if ui.small_button("⚙").clicked() {
									self.overlay_show_settings = !self.overlay_show_settings;
								}

									let click_through = self
										.overlay
										.as_ref()
										.map_or(false, OverlayController::click_through);
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
