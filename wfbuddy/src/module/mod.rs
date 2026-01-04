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
	
	fn tick(&mut self) {}
}