use std::{fs::File, io::{BufReader, BufWriter}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
	pub app_id: String,
	pub theme: ie::Theme,
	pub client_language: data::Language,
	
	// not used anymore (for now?), the game buffering writing to log could take 10+ sec, making it nearly useless
	pub log_path: String,
	pub pol_delay: f32,
	
	/// Multiplier for Warframe's in-game UI scale (1.0 = 100%).
	pub wf_ui_scale: f32,
	
	/// Enable the in-game overlay window.
	pub overlay_enabled: bool,
	/// If true, the overlay won't intercept mouse input.
	pub overlay_mouse_passthrough: bool,
	/// Overlay background opacity for the info panel (0.0 - 1.0).
	pub overlay_opacity: f32,
	/// Overlay margin from the top-left of the game window (in points).
	pub overlay_margin: f32,
	
	pub relicreward_valuedforma: bool,
}

impl Config {
	pub fn load() -> Self {
		let Ok(file) = File::open(dirs::config_dir().unwrap().join("WFBuddy").join("config.json")) else {return Default::default()};
		serde_json::from_reader(BufReader::new(file)).unwrap_or_default()
	}
	
	pub fn save(&self) {
		let dir_path = dirs::config_dir().unwrap().join("WFBuddy");
		_ = std::fs::create_dir_all(&dir_path);
		let config_path = dir_path.join("config.json");
		let writer = BufWriter::new(File::create(config_path).unwrap());
		serde_json::to_writer(writer, self).unwrap()
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
			client_language: data::Language::English,
			
			#[cfg(unix)]
			log_path: dirs::home_dir().unwrap().join(".steam/steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log").to_string_lossy().to_string(),
			#[cfg(windows)]
			log_path: dirs::cache_dir().unwrap().join("Warframe/EE.log").to_string_lossy().to_string(),
			pol_delay: 3.0,
			
			wf_ui_scale: 1.0,
			overlay_enabled: true,
			overlay_mouse_passthrough: true,
			overlay_opacity: 0.65,
			overlay_margin: 16.0,
			
			relicreward_valuedforma: true,
		}
	}
}