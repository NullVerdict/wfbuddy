mod relicreward;
pub use relicreward::RelicReward;

mod debug;
pub use debug::Debug;

pub trait Module {
	fn name(&self) -> &'static str;

	fn ui(&mut self, ui: &mut egui::Ui);

	#[allow(unused_variables)]
	fn ui_settings(&mut self, ui: &mut egui::Ui, config: &mut crate::config::Config) -> bool {
		false
	}

	#[allow(unused_variables)]
	fn ui_important(&mut self, ui: &mut egui::Ui) -> bool {
		false
	}

	/// Whether this module wants to render the overlay viewport.
	fn overlay_active(&self) -> bool {
		false
	}

	/// UI shown in the overlay viewport. By default this reuses `ui_important`.
	#[allow(unused_variables)]
	fn ui_overlay(&mut self, ui: &mut egui::Ui) -> bool {
		self.ui_important(ui)
	}

	fn tick(&mut self) {}
}
