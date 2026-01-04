use std::{collections::BTreeMap, time::{Duration, Instant}};
use crate::{UiExt, iepol::{EventReceiver, IePolWatchType}};

pub struct RelicReward {
	uniform: crate::Uniform,
	
	rewards_rs: EventReceiver,
	
	current_rewards: Vec<Reward>,
	selected_rewards: BTreeMap<String, u32>,
}

impl RelicReward {
	pub fn new(uniform: crate::Uniform) -> Self {
		let (tx, rewards_rs) = std::sync::mpsc::channel();
		// TODO: identifier + locale files or smth for multi language support
		uniform.iepol.watch_event(IePolWatchType::PartyHeaderText("void fissure/rewards".to_string()), tx);
		
		Self {
			uniform,
			
			rewards_rs,
			
			current_rewards: Vec::new(),
			selected_rewards: BTreeMap::new(),
		}
	}
	
	fn check_rewards(&mut self, rewards: ie::screen::relicreward::Rewards) {
		self.current_rewards = rewards.rewards
			.into_iter()
			.map(|reward| {
				let lang = crate::config_read().client_language;
				let name = self.uniform.data.find_item_name((lang, &reward.name));
				let id = self.uniform.data.id_manager.get_id_from_locale((lang, name)).unwrap();
				Reward {
					vaulted: self.uniform.data.vaulted_items.contains(&id),
					platinum: self.uniform.data.platinum_values.get(&id).copied().unwrap_or_default(),
					ducats: self.uniform.data.ducat_values.get(&id).copied().unwrap_or_default(),
					owned: reward.owned,
					name: name.to_owned(),
				}
			})
			.collect::<Vec<_>>();
		log::debug!("timer is {}", rewards.timer);
		self.uniform.iepol.delay_till(Instant::now() + { let delay_secs = rewards.timer.saturating_sub(1) as u64; Duration::from_secs(delay_secs) });
	}
	
	fn check_selected(&mut self, image: std::sync::Arc<ie::OwnedImage>) {
		let selected = self.uniform.ie.relicreward_get_selected(image.as_image());
		if let Some(reward) = self.current_rewards.get(selected as usize) {
			let mut name = reward.name.clone();
			let mut amount = 1;
			if name.starts_with("2 X ") {
				name = name.trim_start_matches("2 X ").to_owned();
				amount = 2;
			}
			
			log::debug!("incrementing {name} by {amount} as the picked index was {selected}");
			*self.selected_rewards.entry(name).or_insert(0) += amount;
		}
		
		self.current_rewards.clear();
		self.uniform.iepol.delay_till(Instant::now() + Duration::from_secs(15));
	}
}

impl super::Module for RelicReward {
	fn name(&self) -> &'static str {
		"Relic Rewards"
	}
	
	fn ui(&mut self, ui: &mut egui::Ui) {
		ui.horizontal(|ui| {
			if ui.button("Check").clicked() {
				let Some(image) = crate::capture::capture() else {return};
				let rewards = self.uniform.ie.relicreward_get_rewards(image.as_image());
				self.check_rewards(rewards);
			}
			
			if ui.button("Selected").clicked() {
				let Some(image) = crate::capture::capture() else {return};
				self.check_selected(std::sync::Arc::new(image));
			}
		});
		
		self.ui_important(ui);
	}
	
	fn ui_settings(&mut self, ui: &mut egui::Ui, config: &mut crate::config::Config) -> bool {
		ui.label("Relic Rewards");
		ui.checkbox(&mut config.relicreward_valuedforma, "Forma has value").clicked()
	}
	
	fn ui_important(&mut self, ui: &mut egui::Ui) -> bool {
		let reward_count = self.current_rewards.len();
		let selected_count = self.selected_rewards.len();
		if reward_count == 0 && selected_count == 0 {return false}
		let valued_forma = crate::config_read().relicreward_valuedforma;
		
		ui.columns(reward_count, |uis| {
			for (i, ui) in uis.iter_mut().enumerate() {
				let reward = &self.current_rewards[i];
				ui.label(&reward.name);
				let plat = if !reward.name.contains("Forma Blueprint") || valued_forma {reward.platinum} else {0.0};
				ui.label(format!("Platinum: {}", plat));
				ui.label(format!("Ducats: {}", reward.ducats));
				
				let owned = reward.owned + self.selected_rewards.get(&reward.name).map_or(0, |v| *v);
				if owned > 0 {
					ui.label(format!("Owned: {}", owned));
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
		// Drain the channel: if OCR runs faster than the UI tick, we still handle all events.
		while let Ok(image) = self.rewards_rs.try_recv() {
			let rewards = self.uniform.ie.relicreward_get_rewards(image.as_image());
			if rewards.timer >= 3 {
				self.check_rewards(rewards);
			} else {
				self.check_selected(image);
			}
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