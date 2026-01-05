use std::{collections::BTreeMap, time::{Duration, Instant}};
use crate::{UiExt, iepol::{EventReceiver, IePolWatchType}};

pub struct RelicReward {
	uniform: crate::Uniform,
	
	rewards_rs: EventReceiver,
	
	current_rewards: Vec<Reward>,
	selected_rewards: BTreeMap<String, u32>,
	/// Safety net: if we don't observe the screen transition away from the
	/// reward picker (OCR miss, alt-tab, etc.), clear the overlay after this.
	overlay_expires_at: Option<Instant>,
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
			overlay_expires_at: None,
		}
	}
	
	fn check_rewards(&mut self, rewards: ie::screen::relicreward::Rewards) {
		self.current_rewards = rewards.rewards
			.into_iter()
			.map(|reward| {
				let name = self.uniform.data.find_item_name(&reward.name);
				Reward {
					vaulted: self.uniform.data.vaulted_items.contains(&name),
					platinum: self.uniform.data.platinum_values.get(&name).copied().unwrap_or_default(),
					ducats: self.uniform.data.ducat_values.get(&name).copied().unwrap_or_default(),
					owned: reward.owned,
					name,
				}
			})
			.collect::<Vec<_>>();

		// The relic reward screen has a visible countdown. In practice, OCR can
		// miss the final "picked" screen (or the user can leave the screen early),
		// which previously caused the overlay to stick forever.
		//
		// We use the in-game timer as a best-effort expiration for the overlay.
		self.overlay_expires_at = Some(Instant::now() + Duration::from_secs(rewards.timer as u64 + 3));

		// Poll again shortly before the timer hits 0, but never underflow.
		let delay_secs = rewards.timer.saturating_sub(1) as u64;
		self.uniform.iepol.delay_till(Instant::now() + Duration::from_secs(delay_secs));
	}
	
	fn check_selected(&mut self, image: std::sync::Arc<ie::OwnedImage>) {
		let selected = self.uniform.ie.relicreward_get_selected(image.as_image());
		if let Some(reward) = self.current_rewards.get(selected as usize) {
			let amount = if reward.name.starts_with("2 X ") { 2 } else { 1 };
			log::debug!("incrementing {} by {amount} as the picked index was {selected}", reward.name);
			*self.selected_rewards.entry(reward.name.clone()).or_insert(0) += amount;
		}
		
		self.current_rewards.clear();
		self.overlay_expires_at = None;
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
				let plat = if !reward.name.contains("Forma Blueprint") || valued_forma {
					reward.platinum
				} else {
					0.0
				};
				ui.label(format!("Platinum: {}", plat));
				ui.label(format!("Ducats: {}", reward.ducats));
				
				if reward.owned > 0 {
					ui.label(format!("Owned: {}", reward.owned + self.selected_rewards.get(&reward.name).map_or(0, |v| *v)));
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



	fn overlay_cards(&self) -> Vec<crate::overlay::OverlayCard> {
		let valued_forma = crate::config_read().relicreward_valuedforma;
		self.current_rewards
			.iter()
			.map(|reward| {
				let plat = if !reward.name.contains("Forma Blueprint") || valued_forma {
					reward.platinum
				} else {
					0.0
				};
				let picked = self.selected_rewards.get(&reward.name).copied().unwrap_or(0);
				crate::overlay::OverlayCard {
					name: reward.name.clone(),
					vaulted: reward.vaulted,
					platinum: plat,
					ducats: reward.ducats,
					owned: reward.owned + picked,
				}
			})
			.collect()
	}
	
	fn tick(&mut self) {
		// Expire the overlay even if OCR misses the screen transition.
		if let Some(expires_at) = self.overlay_expires_at {
			if Instant::now() >= expires_at {
				log::debug!("relic reward overlay expired; clearing cards");
				self.current_rewards.clear();
				self.overlay_expires_at = None;
			}
		}

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