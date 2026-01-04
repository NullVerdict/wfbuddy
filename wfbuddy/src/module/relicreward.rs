use std::{
	collections::BTreeMap,
	time::{Duration, Instant},
};

use crate::iepol::{EventReceiver, IePolWatchType};

use ie::screen::relicreward::Rarity;

// `UiExt` is re-exported at the crate root (`pub use ui::UiExt;`).
// Importing via `crate::ui::ext` fails because `ext` is intentionally private.
use crate::UiExt;
use crate::ui::OverlayPlacement;

pub struct RelicReward {
	uniform: crate::Uniform,

	rewards_rs: EventReceiver,

	current_rewards: Vec<Reward>,
	selected_rewards: BTreeMap<String, u32>,

	// True while the relic reward screen is detected.
	reward_screen_active: bool,

	// Overlay placement computed from the last detected reward screen.
	overlay_placement: Option<OverlayPlacement>,
	last_reward_seen: Option<Instant>,
	next_auto_check: Instant,
}

impl RelicReward {
	pub fn new(uniform: crate::Uniform) -> Self {
		let (tx, rewards_rs) = std::sync::mpsc::channel();
		// TODO: identifier + locale files or smth for multi-language support.
		uniform
			.iepol
			.watch_event(IePolWatchType::PartyHeaderText("void fissure/rewards".to_string()), tx);

		Self {
			uniform,
			rewards_rs,
			current_rewards: Vec::new(),
			selected_rewards: BTreeMap::new(),
			reward_screen_active: false,
			overlay_placement: None,
			last_reward_seen: None,
			next_auto_check: Instant::now(),
		}
	}

	fn check_rewards(&mut self, rewards: ie::screen::relicreward::Rewards) {
		let lang = crate::config().client_language;

		self.current_rewards = rewards
			.rewards
			.into_iter()
			.map(|reward| {
				let name = self.uniform.data.find_item_name((lang, &reward.name));
				let id = self
					.uniform
					.data
					.id_manager
					.get_id_from_locale((lang, name))
					.unwrap();

				Reward {
					name: name.to_owned(),
					rarity: reward.rarity,
					owned: reward.owned,
					vaulted: self.uniform.data.vaulted_items.contains(&id),
					platinum: self
						.uniform
						.data
						.platinum_values
						.get(&id)
						.copied()
						.unwrap_or_default(),
					ducats: self
						.uniform
						.data
						.ducat_values
						.get(&id)
						.copied()
						.unwrap_or_default(),
				}
			})
			.collect::<Vec<_>>();

		// Poll again near the end of the reward timer so we can catch the user's selection.
		let delay = rewards.timer.saturating_sub(2);
		if delay > 0 {
			self.uniform
				.iepol
				.delay_till(Instant::now() + Duration::from_secs(delay as u64));
		}
	}

	fn check_selected(&mut self, image: std::sync::Arc<ie::OwnedImage>) {
		let ui_scale = crate::config().wf_ui_scale;
		let selected = self
			.uniform
			.ie
			.relicreward_get_selected(image.as_image(), ui_scale);

		if let Some(reward) = self.current_rewards.get(selected as usize) {
			let mut name = reward.name.clone();
			let mut amount = 1;

			// Warframe sometimes prefixes stack size like: `2 X <item name>`.
			if name.starts_with("2 X ") {
				name = name.trim_start_matches("2 X ").to_owned();
				amount = 2;
			}

			*self.selected_rewards.entry(name).or_insert(0) += amount;
		}

		// Stop showing the choice overlay after selection.
		self.current_rewards.clear();

		// Give the game time to transition away from the rewards screen.
		self.uniform
			.iepol
			.delay_till(Instant::now() + Duration::from_secs(15));
	}

	fn update_overlay_placement(
		&mut self,
		app_id: &str,
		capture: ie::Image,
		rewards: &ie::screen::relicreward::Rewards,
	) {
		let Some(bounds) = crate::capture::window_bounds(app_id) else {
			self.overlay_placement = None;
			return;
		};

		let cap_w = capture.width() as f32;
		let cap_h = capture.height() as f32;

		// Convert capture pixel coordinates into egui logical coordinates.
		// xcap window bounds may be in logical units (common) or physical pixels depending on platform.
		// We detect which by comparing captured pixel dimensions against the reported window size.
		let scale = bounds.scale_factor.max(1.0);

		let bounds_are_logical = {
			let expect_physical_w = bounds.width * scale;
			let expect_physical_h = bounds.height * scale;
			(cap_w - expect_physical_w).abs() / cap_w.max(1.0) < 0.15
				&& (cap_h - expect_physical_h).abs() / cap_h.max(1.0) < 0.15
		};

		let base_x = if bounds_are_logical { bounds.x } else { bounds.x / scale };
		let base_y = if bounds_are_logical { bounds.y } else { bounds.y / scale };

		let px_to_logical = 1.0 / scale;

		let area = rewards.reward_area;
		let pos = egui::pos2(
			base_x + area.x as f32 * px_to_logical,
			base_y + area.y as f32 * px_to_logical,
		);

		let size = egui::vec2(
			area.w as f32 * px_to_logical,
			area.h as f32 * px_to_logical,
		);

		self.overlay_placement = Some(OverlayPlacement { pos, size });
	}
}

impl super::Module for RelicReward {
	fn name(&self) -> &'static str {
		"Relic Rewards"
	}

	fn ui(&mut self, ui: &mut egui::Ui) {
		ui.horizontal(|ui| {
			if ui.button("Check").clicked() {
				let Some(image) = crate::capture::capture() else { return };
				let ui_scale = crate::config().wf_ui_scale;
				let rewards = self.uniform.ie.relicreward_get_rewards(image.as_image(), ui_scale);
				self.check_rewards(rewards);
			}

			if ui.button("Selected").clicked() {
				let Some(image) = crate::capture::capture() else { return };
				self.check_selected(std::sync::Arc::new(image));
			}
		});

		self.ui_important(ui);
	}

	fn ui_settings(&mut self, ui: &mut egui::Ui, config: &mut crate::config::Config) -> bool {
		ui.label("Relic Rewards");
		ui.checkbox(&mut config.relicreward_valuedforma, "Forma has value")
			.clicked()
	}

	fn overlay_active(&self) -> bool {
		self.reward_screen_active
	}

	fn overlay_placement(&self) -> Option<crate::ui::OverlayPlacement> {
		self.overlay_placement
	}

	fn ui_overlay(&mut self, ui: &mut egui::Ui) -> bool {
		if self.current_rewards.is_empty() {
			ui.label("Detecting relic rewards…");
			return true;
		}
		self.ui_important(ui)
	}

	fn ui_important(&mut self, ui: &mut egui::Ui) -> bool {
		let reward_count = self.current_rewards.len();
		let selected_count = self.selected_rewards.len();
		if reward_count == 0 && selected_count == 0 {
			return false;
		}

		ui.columns(reward_count.max(1), |uis| {
			for (i, ui) in uis.iter_mut().enumerate().take(reward_count) {
				let reward = &self.current_rewards[i];

				ui.label(&reward.name);
				ui.label(format!("Rarity: {}", reward.rarity_label()));

				let plat = if !reward.name.contains("Forma Blueprint")
					|| crate::config().relicreward_valuedforma
				{
					reward.platinum
				} else {
					0.0
				};

				ui.label(format!("Platinum: {plat}"));
				ui.label(format!("Ducats: {}", reward.ducats));

				let owned = reward.owned
					+ self
						.selected_rewards
						.get(&reward.name)
						.copied()
						.unwrap_or_default();

				if owned > 0 {
					ui.label(format!("Owned: {owned}"));
				} else {
					ui.label("");
				}

				if reward.vaulted {
					ui.label("Vaulted");
				}
			}
		});

		if selected_count > 0 {
			if reward_count > 0 {
				ui.spacer();
			}

			ui.label("Selected Rewards");
			ui.indent("selected", |ui| {
				for (item, amount) in &self.selected_rewards {
					ui.label(format!("{item} x{amount}"));
				}
			});

			ui.spacer();
			if ui.button("Clear Selected Rewards").clicked() {
				self.selected_rewards.clear();
			}
		}

		true
	}

	fn tick(&mut self) {
		let now = Instant::now();

		// 1) Event-driven path (party header watcher), if it fires.
		if let Ok(image) = self.rewards_rs.try_recv() {
			let ui_scale = crate::config().wf_ui_scale;
			let rewards = self.uniform.ie.relicreward_get_rewards(image.as_image(), ui_scale);

			let reward_screen = rewards.present || rewards.timer > 0 || !rewards.rewards.is_empty();
			if reward_screen {
				let app_id = { crate::config().app_id.clone() };
				self.reward_screen_active = true;
				self.update_overlay_placement(&app_id, image.as_image(), &rewards);
				self.last_reward_seen = Some(now);
			}

			if rewards.timer >= 3 {
				self.check_rewards(rewards);
			} else if !self.current_rewards.is_empty() {
				self.check_selected(image);
			}
		}

		// 2) Automatic detection path (no button-click required).
		let (overlay_enabled, app_id, ui_scale) = {
			let cfg = crate::config();
			(cfg.overlay_enabled, cfg.app_id.clone(), cfg.wf_ui_scale)
		};

		if !overlay_enabled {
			return;
		}

		// Throttle captures to avoid burning CPU/GPU.
		if now < self.next_auto_check {
			return;
		}
		self.next_auto_check = now + Duration::from_millis(250);

		let Some(image) = crate::capture::capture_specific(&app_id) else { return };
		let rewards = self.uniform.ie.relicreward_get_rewards(image.as_image(), ui_scale);

		let reward_screen = rewards.present || rewards.timer > 0 || !rewards.rewards.is_empty();

		// If we're not on the reward screen, clear the overlay after a short grace period.
		if !reward_screen {
			if let Some(last) = self.last_reward_seen {
				if now.duration_since(last) > Duration::from_secs(2) {
					self.current_rewards.clear();
					self.reward_screen_active = false;
					self.overlay_placement = None;
					self.last_reward_seen = None;
				}
			}
			return;
		}

		self.reward_screen_active = true;
		self.last_reward_seen = Some(now);
		self.update_overlay_placement(&app_id, image.as_image(), &rewards);

		// When the reward timer is almost over, the name list area changes (and is less reliable),
		// so we switch to detecting the selected reward instead.
		if rewards.timer >= 3 || self.current_rewards.is_empty() {
			self.check_rewards(rewards);
		} else {
			self.check_selected(std::sync::Arc::new(image));
		}
	}}

#[derive(Clone)]
struct Reward {
	name: String,
	rarity: Rarity,
	vaulted: bool,
	platinum: f32,
	ducats: u32,
	owned: u32,
}

impl Reward {
	fn rarity_label(&self) -> &'static str {
		self.rarity.label()
	}
}
