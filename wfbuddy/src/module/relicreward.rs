use std::{collections::BTreeMap, time::{Duration, Instant}};
use crate::{UiExt, iepol::{EventReceiver, IePolWatchType}};

pub struct RelicReward {
	uniform: crate::Uniform,
	
	rewards_rs: EventReceiver,
	
	current_rewards: Vec<Reward>,
	selected_rewards: BTreeMap<String, u32>,
	// When to auto-hide the overlay if we miss a "selected" event.
	overlay_expires_at: Option<Instant>,
	last_screen_seen_at: Option<Instant>,
}

impl RelicReward {
	pub fn new(uniform: crate::Uniform) -> Self {
		let (tx, rewards_rs) = std::sync::mpsc::channel();
		// Cheap screen detection (no OCR) so we don't peg the CPU.
		uniform.iepol.watch_event(IePolWatchType::RelicRewardScreen, tx);
		
		Self {
			uniform,
			
			rewards_rs,
			
			current_rewards: Vec::new(),
			selected_rewards: BTreeMap::new(),
			overlay_expires_at: None,
			last_screen_seen_at: None,
		}
	}
	
	fn check_rewards(&mut self, rewards: ie::screen::relicreward::Rewards) {
		let now = Instant::now();
		self.last_screen_seen_at = Some(now);
		// The timer comes from OCR, so use it as a best-effort TTL.
		self.overlay_expires_at = Some(now + Duration::from_secs(rewards.timer.saturating_add(3) as u64));
		self.current_rewards = rewards.rewards
			.into_iter()
			.map(|reward| {
				let name = self.uniform.data.find_item_name(&reward.name);
				let kind = kind_label_ru(&self.uniform.data, &name);
				Reward {
					vaulted: self.uniform.data.vaulted_items.contains(&name),
					platinum: self.uniform.data.platinum_values.get(&name).copied().unwrap_or_default(),
					ducats: self.uniform.data.ducat_values.get(&name).copied().unwrap_or_default(),
					owned: reward.owned,
					name,
					kind,
				}
			})
			.collect::<Vec<_>>();
		// Poll again shortly before the timer hits 0, but never underflow.
		let delay_secs = rewards.timer.saturating_sub(1) as u64;
		self.uniform.iepol.delay_till(Instant::now() + Duration::from_secs(delay_secs));
	}
	
	fn check_selected(&mut self, image: std::sync::Arc<ie::OwnedImage>) {
		self.last_screen_seen_at = Some(Instant::now());
		let selected = self.uniform.ie.relicreward_get_selected(image.as_image());
		if let Some(reward) = self.current_rewards.get(selected as usize) {
			log::debug!("incrementing {} as the picked index was {selected}", reward.name);
			*self.selected_rewards.entry(reward.name.clone()).or_insert(0) += 1;
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
					kind: reward.kind.clone(),
					vaulted: reward.vaulted,
					platinum: plat,
					ducats: reward.ducats,
					owned: reward.owned + picked,
				}
			})
			.collect()
	}
	
	fn tick(&mut self) {
		// Drain the channel: if OCR runs faster than the UI tick, we still handle all events.
		while let Ok(image) = self.rewards_rs.try_recv() {
			self.last_screen_seen_at = Some(Instant::now());
			let rewards = self.uniform.ie.relicreward_get_rewards(image.as_image());
			if rewards.timer >= 3 {
				self.check_rewards(rewards);
			} else {
				self.check_selected(image);
			}
		}

		// Auto-hide overlay if we haven't received updates for a while.
		let now = Instant::now();
		if let Some(exp) = self.overlay_expires_at {
			if now >= exp {
				self.current_rewards.clear();
				self.overlay_expires_at = None;
			}
		}
		if let Some(last) = self.last_screen_seen_at {
			if !self.current_rewards.is_empty() && now.duration_since(last) > Duration::from_secs(10) {
				self.current_rewards.clear();
				self.overlay_expires_at = None;
			}
		}
	}
}

struct Reward {
	name: String,
	kind: Option<String>,
	vaulted: bool,
	platinum: f32,
	ducats: u32,
	owned: u32,
}

/// Best-effort mapping of a reward name to a short Russian "kind" label.
///
/// This is the missing information you can't see on the relic reward screen
/// (e.g. "Receiver", "Blade", "Systems", etc.).
fn kind_label_ru(data: &data::Data, name: &str) -> Option<String> {
	// Prefer explicit part names (most useful for relic choices).
	let n = name;
	let suffix = |s: &str| n.ends_with(s);

	let part = if suffix(" Neuroptics") || suffix(" Neuroptics Blueprint") { Some("Нейрооптика") }
	else if suffix(" Chassis") || suffix(" Chassis Blueprint") { Some("Шасси") }
	else if suffix(" Systems") || suffix(" Systems Blueprint") { Some("Системы") }
	else if suffix(" Receiver") { Some("Казённик") }
	else if suffix(" Barrel") { Some("Ствол") }
	else if suffix(" Stock") { Some("Ложа") }
	else if suffix(" Blade") { Some("Клинок") }
	else if suffix(" Handle") || suffix(" Grip") { Some("Рукоять") }
	else if suffix(" Blueprint") { Some("Чертёж") }
	else { None };
	if let Some(p) = part { return Some(p.to_string()); }

	// Fall back to WarframeStat "type" for anything that doesn't match a known part.
	let meta = data.item_meta.get(name)?;
	let t = meta.item_type.as_deref()?;
	let t_norm = t.to_ascii_lowercase().replace([' ', '_'], "");
	let ru = match t_norm.as_str() {
		"blueprint" => "Чертёж",
		"primepart" => "Прайм-деталь",
		"primeset" => "Прайм-набор",
		"relic" => "Реликвия",
		"mod" => "Мод",
		_ => {
			// As a last resort, show the raw type (still better than nothing).
			return Some(t.to_string());
		}
	};
	Some(ru.to_string())
}