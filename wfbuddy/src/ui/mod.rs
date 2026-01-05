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
	overlay_viewport_open: bool,
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
			overlay_viewport_open: false,
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
			// Explicitly close the viewport when toggled off.
			if self.overlay_viewport_open {
				let viewport_id = egui::ViewportId::from_hash_of("wfbuddy.relicreward_overlay");
				let builder = egui::ViewportBuilder::default();
				parent_ctx.show_viewport_deferred(viewport_id, builder, move |ctx, _class| {
					ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
				});
				self.overlay_viewport_open = false;
			}
			return;
		}

		self.update_overlay_game_rect();

		let cards: Vec<crate::overlay::OverlayCard> = self
			.modules
			.iter()
			.flat_map(|m| m.overlay_cards())
			.collect();
		if cards.is_empty() {
			// In newer egui versions the child viewport might stay alive even if we
			// stop calling show_viewport_*. Explicitly close it.
			if self.overlay_viewport_open {
				let viewport_id = egui::ViewportId::from_hash_of("wfbuddy.relicreward_overlay");
				let builder = egui::ViewportBuilder::default();
				parent_ctx.show_viewport_deferred(viewport_id, builder, move |ctx, _class| {
					ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
				});
				self.overlay_viewport_open = false;
			}
			return;
		}
		self.overlay_viewport_open = true;

		let game_rect = self.overlay_game_rect;

		// Compute a compact overlay size that hugs the content.
		let n = cards.len().max(1).min(4);
		let overlay_w = (
			crate::overlay::OVERLAY_PADDING * 2.0
				+ crate::overlay::OVERLAY_CARD_WIDTH * n as f32
				+ crate::overlay::OVERLAY_SPACING * (n.saturating_sub(1) as f32)
		)
		.clamp(
			crate::overlay::OVERLAY_MIN_WIDTH,
			crate::overlay::OVERLAY_MAX_WIDTH,
		);
		let overlay_h = crate::overlay::OVERLAY_HEIGHT;

		let viewport_id = egui::ViewportId::from_hash_of("wfbuddy.relicreward_overlay");
		let builder = egui::ViewportBuilder::default()
			.with_title("WFBuddy Overlay")
			.with_decorations(false)
			.with_resizable(false)
			.with_transparent(cfg.overlay_transparent_window)
			.with_window_level(egui::viewport::WindowLevel::AlwaysOnTop)
			.with_mouse_passthrough(cfg.overlay_mouse_passthrough)
			.with_inner_size(egui::vec2(overlay_w, overlay_h));

		parent_ctx.show_viewport_deferred(viewport_id, builder, move |ctx, _class| {
			// IMPORTANT: for per-pixel transparency we need to clear with alpha=0.
			// In eframe this is tied to visuals.window_fill for the viewport.
			// See egui docs for ViewportBuilder.
			let mut style = (*ctx.style()).clone();
			if cfg.overlay_transparent_window {
				style.visuals.window_fill = egui::Color32::TRANSPARENT;
				style.visuals.panel_fill = egui::Color32::TRANSPARENT;
			} else {
				// If the system/GL config doesn't support transparent windows, keep it
				// opaque but compact.
				style.visuals.window_fill = egui::Color32::from_black_alpha(230);
				style.visuals.panel_fill = egui::Color32::from_black_alpha(230);
			}
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
				let margin = cfg.overlay_margin_px / ppp;
				let mut px = x + (w - overlay_w) * 0.5;
				let mut py = y + h * cfg.overlay_y_ratio - overlay_h * 0.5;
				// Clamp inside the game window.
				px = px.clamp(x + margin, x + w - overlay_w - margin);
				py = py.clamp(y + margin, y + h - overlay_h - margin);
				ctx.send_viewport_cmd(egui::viewport::ViewportCommand::OuterPosition(
					egui::pos2(px, py),
				));
			}

			let bg_alpha = if cfg.overlay_transparent_window { 140 } else { 230 };
			let card_alpha = if cfg.overlay_transparent_window { 110 } else { 210 };
			let total_plat: f32 = cards.iter().map(|c| c.platinum).sum();
			let total_ducats: u32 = cards.iter().map(|c| c.ducats).sum();

			egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
				ui.spacing_mut().item_spacing = egui::vec2(crate::overlay::OVERLAY_SPACING, 10.0);

				let outer = egui::Frame::NONE
					.fill(egui::Color32::from_black_alpha(bg_alpha))
					.stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(40)))
					.corner_radius(egui::CornerRadius::same(18))
					.inner_margin(egui::Margin::same(crate::overlay::OVERLAY_PADDING));

				outer.show(ui, |ui| {
					ui.vertical(|ui| {
						ui.horizontal(|ui| {
							for card in cards.iter() {
								let frame = egui::Frame::NONE
									.fill(egui::Color32::from_black_alpha(card_alpha))
									.stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(30)))
									.corner_radius(egui::CornerRadius::same(14))
									.inner_margin(egui::Margin::same(12.0));

								frame.show(ui, |ui| {
									ui.set_min_size(egui::vec2(
										crate::overlay::OVERLAY_CARD_WIDTH,
										crate::overlay::OVERLAY_CARD_MIN_HEIGHT,
									));

									ui.horizontal(|ui| {
										ui.label(
											egui::RichText::new(card.name.as_str())
											.strong()
											.size(16.0),
										);
										ui.with_layout(
											egui::Layout::right_to_left(egui::Align::Center),
											|ui| {
												if card.vaulted {
													ui.label(
														egui::RichText::new("VAULTED")
														.strong()
														.size(12.0),
													);
												}
											},
										);
									});

									ui.add_space(6.0);
									ui.horizontal(|ui| {
										ui.label(egui::RichText::new(format!("{:.1}p", card.platinum)).size(14.0));
										ui.label(egui::RichText::new("•").weak());
										ui.label(egui::RichText::new(format!("{}d", card.ducats)).size(14.0));
									});
									ui.add_space(6.0);
									ui.label(
										egui::RichText::new(format!("Owned: {}", card.owned)).size(13.0),
									);
								});
							}
						});

						ui.add_space(6.0);
						ui.horizontal(|ui| {
							ui.label(
								egui::RichText::new("Use Ctrl+Tab to interact with the overlay")
								.weak()
								.size(12.0),
							);
							ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
								ui.label(
									egui::RichText::new(format!("Σ {:.1}p • {}d", total_plat, total_ducats))
									.weak()
									.size(12.0),
								);
							});
						});
					});
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