use std::sync::{Arc, LazyLock, Mutex};

pub mod util;
pub mod capture;
// mod logwatcher;
mod iepol;
mod module;
mod ui;
mod i18n;
mod overlay;
pub use ui::UiExt;
mod config;

pub type Uniform = Arc<UniformData>;
pub struct UniformData {
	pub iepol: iepol::IePol,
	pub data: data::Data,
	pub ie: Arc<ie::Ie>,
}

static CONFIG: LazyLock<Arc<Mutex<config::Config>>> = LazyLock::new(|| Arc::new(Mutex::new(config::Config::load())));
pub fn config() -> std::sync::MutexGuard<'static, config::Config> {
	CONFIG.lock().unwrap()
}

fn main() {
	// Load config early so we can configure localization and window options.
	let cfg = config().clone();
	i18n::init(Some(&cfg.ui_locale));

	let title = tr!("app-title").to_string();

	let mut native_options = eframe::NativeOptions::default();

	// Configure overlay window behavior at startup (some options require restart).
	if matches!(cfg.ui_mode, config::UiMode::Overlay) {
		native_options.viewport = native_options
			.viewport
			.with_decorations(false)
			.with_transparent(true)
			.with_always_on_top(true);
	}

	let _ = eframe::run_native(
		&title,
		native_options,
		Box::new(|cc| Ok(Box::new(ui::WFBuddy::new(cc)))),
	);
}
