use crate::ui::ext::UiExt;

pub fn ui(ui: &mut egui::Ui, modules: &mut [Box<dyn crate::module::Module>]) {
	let mut config = crate::config().clone();
	let mut changed = false;

	// Theme sampling
	if ui.button(crate::tr!("btn-set-theme")).clicked() {
		if let Some(image) = crate::capture::capture_specific(&config.app_id) {
			config.theme = ie::Theme::from_options(image.as_image());
			changed = true;
		}
	}

	ui.separator();

	// Target window / polling
	changed |= ui
		.combo_cached(&mut config.app_id, crate::tr!("label-warframe-window"), || {
			xcap::Window::all()
				.unwrap_or_default()
				.into_iter()
				.filter_map(|w| w.app_name().ok())
				.collect()
		});

	changed |= ui
		.num_edit_range(&mut config.pol_delay, crate::tr!("label-poll-delay"), 0.5..=30.0)
		.changed();

	ui.separator();

	// Localization + scaling
	ui.horizontal(|ui| {
		ui.label(crate::tr!("label-ui-language"));
		let before = config.ui_locale.clone();

		egui::ComboBox::from_id_source("ui_locale")
			.selected_text(&config.ui_locale)
			.show_ui(ui, |ui| {
				ui.selectable_value(&mut config.ui_locale, "en-US".to_string(), "en-US");
				ui.selectable_value(&mut config.ui_locale, "es-ES".to_string(), "es-ES");
			});

		if config.ui_locale != before {
			crate::i18n::set_locale(&config.ui_locale);
			changed = true;
		}
	});

	changed |= ui
		.num_edit_range(&mut config.ui_zoom_factor, crate::tr!("label-ui-scale"), 0.5..=2.5)
		.changed();

	ui.horizontal(|ui| {
		ui.label(crate::tr!("label-window-mode"));

		let before = config.ui_mode;
		egui::ComboBox::from_id_source("ui_mode")
			.selected_text(match config.ui_mode {
				crate::config::UiMode::Window => crate::tr!("mode-window"),
				crate::config::UiMode::Overlay => crate::tr!("mode-overlay"),
			})
			.show_ui(ui, |ui| {
				ui.selectable_value(
					&mut config.ui_mode,
					crate::config::UiMode::Window,
					crate::tr!("mode-window"),
				);
				ui.selectable_value(
					&mut config.ui_mode,
					crate::config::UiMode::Overlay,
					crate::tr!("mode-overlay"),
				);
			});

		if config.ui_mode != before {
			changed = true;
			ui.small(crate::tr!("note-restart-required"));
		}
	});

	if matches!(config.ui_mode, crate::config::UiMode::Overlay) {
		changed |= ui
			.checkbox(&mut config.overlay_click_through, crate::tr!("label-overlay-clickthrough"))
			.changed();
		ui.small(crate::tr!("hint-overlay-hotkey"));
	}

	// Module settings
	for module in modules {
		ui.spacer();
		changed |= module.ui_settings(ui, &mut config);
	}

	if changed {
		// Persist changes
		let mut live = crate::config();
		*live = config;
		live.save();
	}
}
