use crate::ui::ext::UiExt;

pub fn ui(ui: &mut egui::Ui, modules: &mut [Box<dyn crate::module::Module>]) {
	let mut config = crate::config_write();
	let mut changed = false;
	
	if ui.button("Set Theme (Open the settings menu in Warframe with the submenu set to keyboard/mouse). Requires you to restart WFBuddy").clicked()
		&& let Some(image) = crate::capture::capture_specific(&config.app_id)
	{
		config.theme = ie::Theme::from_options(image.as_image());
		changed = true;
		log::info!("new theme: {:?}", config.theme);
	}
	
	changed |= ui.combo_cached(&mut config.app_id, "Warframe Window ID", || {
		// Keep this non-fatal: if window enumeration fails, show an empty list.
		xcap::Window::all()
			.map(|wins| {
				let mut ids = Vec::new();
				for w in wins {
					if let Ok(name) = w.app_name() {
						ids.push(name);
					}
					if let Ok(title) = w.title() {
						ids.push(title);
					}
				}
				ids.sort();
				ids.dedup();
				ids
			})
			.unwrap_or_default()
	});
	
	// changed |= ui.text_edit_singleline(&mut config.log_path).changed();
	
	changed |= ui.num_edit_range(&mut config.pol_delay, "Screenshot polling delay", 0.5..=30.0).changed();
	
	for module in modules {
		ui.spacer();
		changed |= module.ui_settings(ui, &mut config);
	}
	
	if changed {
		config.save();
	}
}