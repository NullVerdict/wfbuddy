use std::{
	collections::BTreeMap,
	time::{Duration, Instant},
};

use crate::iepol::{EventReceiver, IePolWatchType};

use ie::screen::relicreward::Rarity;

use crate::UiExt;

pub struct RelicReward {
	uniform: crate::Uniform,

	rewards_rs: EventReceiver,

	current_rewards: Vec<Reward>,
	selected_rewards: BTreeMap<String, u32>,
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
		!self.current_rewards.is_empty()
	}

	fn ui_overlay(&mut self, ui: &mut egui::Ui) -> bool {
		// Reuse the important UI, but keep it compact.
		self.ui_important(ui)
	}

	fn ui_important(&mut self, ui: &mut egui::Ui) -> bool {
		let reward_count = self.current_rewards.len();
		let selected_count = self.selected_rewards.len();
		if reward_count == 0 && selected_count == 0 {
			return false;
		}

		ui.columns(reward_count.max(1), |uis| {
			for (i, ui) in uis.into_iter().enumerate().take(reward_count) {
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
		let Ok(image) = self.rewards_rs.try_recv() else { return };

		let ui_scale = crate::config().wf_ui_scale;
		let rewards = self.uniform.ie.relicreward_get_rewards(image.as_image(), ui_scale);

		// When the reward timer is almost over, the name list area changes (and is less reliable),
		// so we switch to detecting the selected reward instead.
		if rewards.timer >= 3 {
			self.check_rewards(rewards);
		} else {
			self.check_selected(image);
		}
	}
}

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
