use std::{
	collections::BTreeMap,
	sync::Arc,
	time::{Duration, Instant},
};

use crate::iepol::{EventReceiver, IePolWatchType};

pub struct RelicReward {
	uniform: crate::Uniform,

	rewards_rs: EventReceiver,

	current_rewards: Vec<Reward>,
	/// Aggregated picks across runs, keyed by canonical item name.
	selected_rewards: BTreeMap<String, u32>,
}

impl RelicReward {
	pub fn new(uniform: crate::Uniform) -> Self {
		let (tx, rewards_rs) = std::sync::mpsc::channel();

		// TODO(i18n): map per in-game locale (see `client_language`) once we add locale packs.
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

	fn normalize_name_and_amount(name: &str) -> (String, u32) {
		// Warframe shows some items as "2 X <Name>".
		if let Some(rest) = name.strip_prefix("2 X ") {
			(rest.to_owned(), 2)
		} else {
			(name.to_owned(), 1)
		}
	}

	fn handle_rewards(&mut self, rewards: ie::screen::relicreward::Rewards) {
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
					.expect("missing item id for localized name");

				Reward {
					name: name.to_owned(),
					vaulted: self.uniform.data.vaulted_items.contains(&id),
					platinum: *self.uniform.data.platinum_values.get(&id).unwrap_or(&0.0),
					ducats: *self.uniform.data.ducat_values.get(&id).unwrap_or(&0),
					owned: reward.owned,
				}
			})
			.collect();

		// We poll again shortly before the timer expires to read the user's selection.
		self.uniform.iepol.delay_till(
			Instant::now() + Duration::from_secs(rewards.timer.saturating_sub(2) as u64),
		);
	}

	fn handle_selection(&mut self, image: Arc<ie::OwnedImage>) {
		let selected_idx = self.uniform.ie.relicreward_get_selected(image.as_image()) as usize;

		if let Some(reward) = self.current_rewards.get(selected_idx) {
			let (name, amount) = Self::normalize_name_and_amount(&reward.name);
			*self.selected_rewards.entry(name).or_insert(0) += amount;
		}

		self.current_rewards.clear();

		// Give the game time to transition back to gameplay.
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
			if ui.button(crate::tr!("btn-check")).clicked() {
				let Some(image) = crate::capture::capture() else { return };
				let rewards = self.uniform.ie.relicreward_get_rewards(image.as_image());
				self.handle_rewards(rewards);
			}

			if ui.button(crate::tr!("btn-selected")).clicked() {
				let Some(image) = crate::capture::capture() else { return };
				self.handle_selection(Arc::new(image));
			}
		});

		self.ui_important(ui);
	}

	fn ui_settings(&mut self, ui: &mut egui::Ui, config: &mut crate::config::Config) -> bool {
		ui.label(crate::tr!("label-relic-rewards"));
		ui.checkbox(&mut config.relicreward_valuedforma, crate::tr!("label-forma-value"))
			.clicked()
	}

	fn ui_important(&mut self, ui: &mut egui::Ui) -> bool {
		let reward_count = self.current_rewards.len();
		let selected_count = self.selected_rewards.len();
		if reward_count == 0 && selected_count == 0 {
			return false;
		}

		ui.columns(reward_count, |uis| {
			for (i, ui) in uis.iter_mut().enumerate() {
				let reward = &self.current_rewards[i];

				ui.label(&reward.name);

				let plat = if !reward.name.contains("Forma Blueprint")
					|| crate::config().relicreward_valuedforma
				{
					reward.platinum
				} else {
					0.0
				};

				ui.label(format!("Platinum: {}", plat));
				ui.label(format!("Ducats: {}", reward.ducats));

				let owned_total = reward.owned + self.selected_rewards.get(&reward.name).copied().unwrap_or(0);
				if owned_total > 0 {
					ui.label(format!("Owned: {}", owned_total));
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
				ui.add_space(8.0);
			}

			ui.label(crate::tr!("label-selected-rewards"));
			ui.indent("selected", |ui| {
				for (item, amount) in &self.selected_rewards {
					ui.label(format!("{item} x{amount}"));
				}
			});

			ui.add_space(8.0);
			if ui.button(crate::tr!("btn-clear-selected")).clicked() {
				self.selected_rewards.clear();
			}
		}

		true
	}

	fn tick(&mut self) {
		let Ok(image) = self.rewards_rs.try_recv() else { return };

		let rewards = self.uniform.ie.relicreward_get_rewards(image.as_image());
		if rewards.timer >= 3 {
			self.handle_rewards(rewards);
		} else {
			self.handle_selection(image);
		}
	}
}

struct Reward {
	name: String,
	vaulted: bool,
	platinum: f32,
	ducats: u32,
	owned: u32,
}
