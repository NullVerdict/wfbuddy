use std::sync::{LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

mod lang;
pub use lang::Language;
pub mod util;
pub mod capture;
// mod logwatcher;
mod iepol;
mod module;
mod overlay;
mod ui;
pub use ui::UiExt;
mod config;

pub type Uniform = std::sync::Arc<UniformData>;

pub struct UniformData {
	pub iepol: iepol::IePol,
	pub data: data::Data,
	pub ie: std::sync::Arc<ie::Ie>,
}

/// Global config.
///
/// We use an `RwLock` so background threads can read frequently while the UI
/// writes rarely (settings changes).
static CONFIG: LazyLock<RwLock<config::Config>> =
	LazyLock::new(|| RwLock::new(config::Config::load()));

pub fn config_read() -> RwLockReadGuard<'static, config::Config> {
	CONFIG.read().expect("config lock poisoned")
}

pub fn config_write() -> RwLockWriteGuard<'static, config::Config> {
	CONFIG.write().expect("config lock poisoned")
}

fn main() -> eframe::Result {
	// Logging is controlled via RUST_LOG (e.g. RUST_LOG=debug).
	env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

	// Transparent overlay windows are sometimes unsupported with certain OpenGL
	// configurations (you'll see logs like "Cannot create transparent window").
	// When the user asks for transparency, prefer the wgpu renderer.
	let cfg = config_read().clone();
	let mut native_options = eframe::NativeOptions::default();
	if cfg.overlay_transparent_window {
		native_options.renderer = eframe::Renderer::Wgpu;
	}
	eframe::run_native(
		"WFBuddy",
		native_options,
		Box::new(|cc| match ui::WFBuddy::try_new(cc) {
			Ok(app) => Ok(Box::new(app) as Box<dyn eframe::App>),
			Err(err) => Ok(Box::new(ui::ErrorApp::new(err)) as Box<dyn eframe::App>),
		}),
	)
}
