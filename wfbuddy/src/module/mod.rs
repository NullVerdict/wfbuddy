mod relicreward;
pub use relicreward::RelicReward;

mod debug;
pub use debug::Debug;

pub trait Module {
	fn name(&self) -> &'static str;
	
	fn ui(&mut self, ui: &mut egui::Ui);
	
	#[allow(unused_variables)]
	fn ui_settings(&mut self, ui: &mut egui::Ui, config: &mut crate::config::Config) -> bool {false}
	
	#[allow(unused_variables)]
	fn ui_important(&mut self, ui: &mut egui::Ui) -> bool {false}


	/// Optional snapshot for the in-game overlay viewport.
	///
	/// The overlay is rendered in a separate, borderless viewport. Since egui
	/// requires overlay callbacks to be `'static`, the overlay window uses a
	/// per-frame snapshot instead of borrowing module state directly.
	#[allow(unused_variables)]
	fn overlay_cards(&self) -> Vec<crate::overlay::OverlayCard> { Vec::new() }
	
	fn tick(&mut self) {}
}