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

	last_overlay_follow_check: std::time::Instant,
	overlay_game_rect: Option<(i32, i32, u32, u32)>,
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

			last_overlay_follow_check: std::time::Instant::now() - Duration::from_secs(10),
			overlay_game_rect: None,
		})
	}

	fn update_overlay_game_rect(&mut self) {
		let cfg = crate::config_read();
		if !cfg.overlay_follow_game_window {
			return;
		}
		// Avoid enumerating windows every frame.
		if self.last_overlay_follow_check.elapsed() < Duration::from_millis(500) {
			return;
		}
		self.last_overlay_follow_check = std::time::Instant::now();
		self.overlay_game_rect = crate::capture::window_rect_specific(&cfg.app_id);
	}

	fn show_overlay_viewport(&mut self, parent_ctx: &egui::Context) {
		let cfg = crate::config_read().clone();
		if !cfg.overlay_relicreward_enabled {
			return;
		}

		self.update_overlay_game_rect();

		let cards: Vec<crate::overlay::OverlayCard> = self
			.modules
			.iter()
			.flat_map(|m| m.overlay_cards())
			.collect();
		if cards.is_empty() {
			// If we stop calling show_viewport_* the child window will be closed.
			return;
		}

		let game_rect = self.overlay_game_rect;

		let viewport_id = egui::ViewportId::from_hash_of("wfbuddy.relicreward_overlay");
		let builder = egui::ViewportBuilder::default()
			.with_title("WFBuddy Overlay")
			.with_decorations(false)
			.with_resizable(false)
			.with_transparent(true)
			.with_window_level(egui::viewport::WindowLevel::AlwaysOnTop)
			.with_mouse_passthrough(cfg.overlay_mouse_passthrough)
			.with_inner_size(egui::vec2(crate::overlay::OVERLAY_WIDTH, crate::overlay::OVERLAY_HEIGHT));

		parent_ctx.show_viewport_deferred(viewport_id, builder, move |ctx, _class| {
			// Semi-transparent background, borderless.
			let mut style = (*ctx.style()).clone();
			style.visuals.window_fill = egui::Color32::from_black_alpha(160);
			ctx.set_style(style);

			// Follow the game window (coordinates are in logical points).
			if cfg.overlay_follow_game_window && let Some((x, y, w, h)) = game_rect {
					let ppp = ctx.pixels_per_point();
					let (x, y, w, h) = (
						x as f32 / ppp,
						y as f32 / ppp,
						w as f32 / ppp,
						h as f32 / ppp,
					);
					let overlay_w = crate::overlay::OVERLAY_WIDTH;
					let overlay_h = crate::overlay::OVERLAY_HEIGHT;
					let margin = cfg.overlay_margin_px / ppp;
					let mut px = x + (w - overlay_w) * 0.5;
					let mut py = y + h * cfg.overlay_y_ratio - overlay_h * 0.5;
					// Clamp inside the game window.
					px = px.clamp(x + margin, x + w - overlay_w - margin);
					py = py.clamp(y + margin, y + h - overlay_h - margin);
					ctx.send_viewport_cmd(egui::viewport::ViewportCommand::OuterPosition(egui::pos2(px, py)));
				}

			egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
				ui.horizontal(|ui| {
					ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
						for card in cards.iter() {
						let frame = egui::Frame::NONE
							.fill(egui::Color32::from_black_alpha(120))
							.stroke(ui.visuals().window_stroke)
							.corner_radius(egui::CornerRadius::same(10))
							.inner_margin(egui::Margin::same(10));
						frame.show(ui, |ui| {
							ui.set_min_width(210.0);
							ui.label(egui::RichText::new(card.name.as_str()).strong());
							ui.add_space(2.0);
							ui.horizontal(|ui| {
								ui.label(format!("{}p", card.platinum));
								ui.label("•");
								ui.label(format!("{}d", card.ducats));
							});
							if card.owned > 0 {
								ui.label(format!("Owned: {}", card.owned));
							} else {
								ui.label("");
							}
							ui.add_space(2.0);
							if card.vaulted {
								ui.label(egui::RichText::new("Vaulted").strong());
							} else {
								ui.label(egui::RichText::new("Not vaulted").weak());
							}
						});
					}
				});
			});
		});
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
		self.show_overlay_viewport(ctx);
		
		// Avoid pegging a CPU core by repainting every frame.
		// We repaint often enough to keep timers (poll countdown) responsive.
		let secs = self.uniform.iepol.secs_till_next_poll();
		let after = Duration::from_secs_f32(secs.clamp(0.05, 0.5));
		ctx.request_repaint_after(after);
	}
}