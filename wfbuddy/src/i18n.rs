use std::sync::LazyLock;

use i18n_embed::{
	fluent::{fluent_language_loader, FluentLanguageLoader},
	DesktopLanguageRequester,
};
use rust_embed::RustEmbed;
use unic_langid::LanguageIdentifier;

#[derive(RustEmbed)]
#[folder = "i18n"]
struct Localizations;

static LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| fluent_language_loader!());

/// Access the global language loader (used by `tr!()`).
pub fn loader() -> &'static FluentLanguageLoader {
	&*LOADER
}

/// Initialize localization. If `forced_locale` is provided, it is preferred over the system locale.
pub fn init(forced_locale: Option<&str>) {
	let requested = if let Some(tag) = forced_locale {
		tag.parse::<LanguageIdentifier>()
			.ok()
			.into_iter()
			.collect::<Vec<_>>()
	} else {
		DesktopLanguageRequester::requested_languages()
	};

	// We don't crash if locale loading fails; we fall back to the built-in fallback locale.
	let _ = i18n_embed::select(loader(), &Localizations, &requested);
}

/// (Re)select a locale at runtime.
pub fn set_locale(tag: &str) {
	init(Some(tag));
}

#[macro_export]
macro_rules! tr {
	($id:literal $(, $args:expr )* $(,)?) => {
		i18n_embed_fl::fl!($crate::i18n::loader(), $id $(, $args )* )
	};
}
