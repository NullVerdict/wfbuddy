use std::{fs::File, io::{BufReader, BufWriter, Write}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
	pub app_id: String,
	pub theme: ie::Theme,
	pub client_language: crate::Language,
	
	// not used anymore (for now?), the game buffering writing to log could take 10+ sec, making it nearly useless
	pub log_path: String,
	pub pol_delay: f32,
	
	pub relicreward_valuedforma: bool,

	/// Show a compact, always-on-top overlay with the currently detected relic rewards.
	///
	/// Rendered as a separate borderless viewport/window.
	pub overlay_relicreward_enabled: bool,

	/// If enabled, the overlay viewport follows the selected game window.
	pub overlay_follow_game_window: bool,

	/// Vertical anchor of the overlay within the game window (0.0 = top, 1.0 = bottom).
	///
	/// Default is slightly below the in-game reward cards.
	pub overlay_y_ratio: f32,

	/// Pixel margin used when clamping the overlay inside the game window.
	pub overlay_margin_px: f32,

	/// Make the overlay window ignore mouse input (click-through).
	///
	/// Note: if you set this to true, the overlay cannot be interacted with.
	pub overlay_mouse_passthrough: bool,

	/// Try to create the overlay viewport as a per-pixel transparent window.
	///
	/// This is a best-effort hint to the OS/graphics stack and may fail on some
	/// systems (e.g. certain OpenGL configs). If you see logs like
	/// "Cannot create transparent window", disable this.
	pub overlay_transparent_window: bool,
}

impl Config {
	pub fn load() -> Self {
		let Some(dir) = dirs::config_dir() else { return Default::default() };
		let path = dir.join("WFBuddy").join("config.json");
		let Ok(file) = File::open(path) else { return Default::default() };
		serde_json::from_reader(BufReader::new(file)).unwrap_or_default()
	}
	
	pub fn save(&self) {
	let Some(dir) = dirs::config_dir() else {
		eprintln!("Could not determine config_dir; config will not be saved");
		return;
	};

	let dir_path = dir.join("WFBuddy");
	if let Err(err) = std::fs::create_dir_all(&dir_path) {
		eprintln!("Failed to create config dir {}: {err}", dir_path.display());
		return;
	}

	let config_path = dir_path.join("config.json");
	let tmp_path = dir_path.join("config.json.tmp");

	let Ok(file) = File::create(&tmp_path) else {
		eprintln!("Failed to write config temp file {}", tmp_path.display());
		return;
	};

	let mut writer = BufWriter::new(file);
	if let Err(err) = serde_json::to_writer(&mut writer, self) {
		eprintln!("Failed to serialize config: {err}");
		return;
	}
	if let Err(err) = writer.flush() {
		eprintln!("Failed to flush config: {err}");
		return;
	}

	// Atomic-ish replace: on Windows rename fails if the destination exists.
	if std::fs::rename(&tmp_path, &config_path).is_err() {
		let _ = std::fs::remove_file(&config_path);
		if let Err(err) = std::fs::rename(&tmp_path, &config_path) {
			eprintln!("Failed to persist config file {}: {err}", config_path.display());
		}
	}
}

}

impl Default for Config {
	fn default() -> Self {
		Self {
			// TODO: check if same on windows
			app_id: "steam_app_230410".to_string(),
			theme: ie::Theme {
				primary: ie::Color::WHITE,
				secondary: ie::Color::WHITE,
			},
			client_language: crate::Language::English,
			
			#[cfg(unix)]
			log_path: dirs::home_dir()
				.map(|h| h.join(".steam/steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log").to_string_lossy().to_string())
				.unwrap_or_else(|| "EE.log".to_string()),
			#[cfg(windows)]
			log_path: dirs::cache_dir()
				.map(|c| c.join("Warframe/EE.log").to_string_lossy().to_string())
				.unwrap_or_else(|| "EE.log".to_string()),
			pol_delay: 3.0,
			
			relicreward_valuedforma: true,

			overlay_relicreward_enabled: true,
			overlay_follow_game_window: true,
			overlay_y_ratio: crate::overlay::OVERLAY_DEFAULT_Y_RATIO_BELOW_REWARDS,
			overlay_margin_px: 16.0,
			overlay_mouse_passthrough: true,
			overlay_transparent_window: false,
		}
	}
}