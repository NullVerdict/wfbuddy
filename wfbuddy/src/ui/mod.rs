use crate::{
	iepol::IePol,
	module::{self, Module},
};

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
		let ie = std::sync::Arc::new(ie::Ie::new(
			crate::config().theme,
			"ocr/detection.mnn",
			format!("ocr/{ocr_code}_recognition.mnn"),
			format!("ocr/{ocr_code}_charset.txt"),
		));

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
		ui.label(format!(
			"Seconds till next poll: {}",
			self.uniform.iepol.secs_till_next_poll()
		));

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

	fn any_overlay_active(&mut self) -> bool {
		self.modules.iter_mut().any(|m| m.overlay_active())
	}

	fn ui_overlay_panel(&mut self, ui: &mut egui::Ui) {
		for module in &mut self.modules {
			if module.overlay_active() {
				module.ui_overlay(ui);
				ui.separator();
			}
		}
	}

	fn show_overlay(&mut self, ctx: &egui::Context) {
		let (enabled, passthrough, opacity, margin, app_id) = {
			let cfg = crate::config();
			(
				cfg.overlay_enabled,
				cfg.overlay_mouse_passthrough,
				cfg.overlay_opacity,
				cfg.overlay_margin,
				cfg.app_id.clone(),
			)
		};

		if !enabled || !self.any_overlay_active() {
			return;
		}

		let bounds = crate::capture::window_bounds(&app_id);

		let mut builder = egui::ViewportBuilder::default()
			.with_title("WFBuddy Overlay")
			.with_decorations(false)
			.with_always_on_top()
			.with_transparent(true)
			.with_resizable(false)
			.with_taskbar(false)
			.with_active(false)
			.with_mouse_passthrough(passthrough);

		// Position + size the overlay to match the game window.
		if let Some(b) = bounds {
			let pos = egui::pos2(b.x / b.scale_factor, b.y / b.scale_factor);
			let size = egui::vec2(b.width / b.scale_factor, b.height / b.scale_factor);
			builder = builder.with_position(pos).with_inner_size(size);
		}

		let overlay_id = egui::ViewportId::from_hash_of("wfbuddy_overlay");

		ctx.show_viewport_immediate(overlay_id, builder, |overlay_ctx, class| {
			// If multi-viewport isn't available for some reason, fall back to an embedded window.
			if matches!(class, egui::ViewportClass::Embedded) {
				egui::Window::new("WFBuddy Overlay")
					.collapsible(false)
					.resizable(true)
					.show(ctx, |ui| self.ui_overlay_panel(ui));
				return;
			}

			egui::Area::new("wfbuddy_overlay_area")
				.fixed_pos(egui::pos2(margin, margin))
				.order(egui::Order::Foreground)
				.show(overlay_ctx, |ui| {
					let alpha = (opacity.clamp(0.0, 1.0) * 255.0) as u8;
					let frame = egui::Frame::none()
						.fill(egui::Color32::from_black_alpha(alpha))
						.rounding(egui::Rounding::same(10.0))
						.stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.fg_stroke.color))
						.inner_margin(egui::Margin::same(10.0));

					frame.show(ui, |ui| self.ui_overlay_panel(ui));
				});
		});
	}
}

impl eframe::App for WFBuddy {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		for module in &mut self.modules {
			module.tick();
		}

		egui::CentralPanel::default().show(ctx, |ui| self.ui(ui));

		// Drive the overlay after the main UI has updated.
		self.show_overlay(ctx);

		// https://github.com/emilk/egui/issues/5113
		// https://github.com/emilk/egui/pull/7775
		ctx.request_repaint();
	}

	fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
		// Keep the root window opaque by drawing a normal CentralPanel background,
		// but allow overlay viewports to be transparent.
		egui::Rgba::from_rgba_premultiplied(0.0, 0.0, 0.0, 0.0).to_array()
	}
}
