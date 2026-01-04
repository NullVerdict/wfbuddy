use std::{fs::File, io::{BufReader, BufWriter, Write}};
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
	
	pub relicreward_valuedforma: bool,
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
			client_language: data::Language::English,
			
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
		}
	}
}