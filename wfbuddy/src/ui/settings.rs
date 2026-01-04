use crate::ui::ext::UiExt;

pub fn ui(ui: &mut egui::Ui, modules: &mut [Box<dyn crate::module::Module>]) {
	let mut config = crate::config();
	let mut changed = false;

	if ui
		.button(
			"Set Theme (Open the settings menu in Warframe with the submenu set to keyboard/mouse). Requires you to restart WFBuddy",
		)
		.clicked()
	{
		if let Some(image) = crate::capture::capture_specific(&config.app_id) {
			config.theme = ie::Theme::from_options(image.as_image());
			changed = true;
			println!("new theme: {:?}", config.theme);
		}
	}

	changed |= ui.combo_cached(&mut config.app_id, "Warframe Window ID", || {
		xcap::Window::all()
			.unwrap()
			.into_iter()
			.filter_map(|v| v.app_name().ok())
			.collect()
	});

	changed |= ui
		.num_edit_range(
			&mut config.pol_delay,
			"Screenshot polling delay (seconds)",
			0.5..=30.0,
		)
		.changed();

	ui.separator();
	ui.label("Scaling");
	changed |= ui
		.num_edit_range(
			&mut config.wf_ui_scale,
			"Warframe UI scale (1.0 = 100%)",
			0.5..=2.0,
		)
		.changed();

	ui.separator();
	ui.label("Overlay");
	changed |= ui
		.checkbox(&mut config.overlay_enabled, "Enable overlay on top of Warframe")
		.changed();
	changed |= ui
		.checkbox(
			&mut config.overlay_mouse_passthrough,
			"Overlay mouse passthrough (click-through)",
		)
		.changed();
	changed |= ui
		.num_edit_range(&mut config.overlay_opacity, "Overlay panel opacity", 0.0..=1.0)
		.changed();
	changed |= ui
		.num_edit_range(&mut config.overlay_margin, "Overlay margin", 0.0..=200.0)
		.changed();

	for module in modules {
		ui.spacer();
		changed |= module.ui_settings(ui, &mut config);
	}

	if changed {
		config.save();
	}
}
