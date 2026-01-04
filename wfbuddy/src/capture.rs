use xcap::image::EncodableLayout;

pub fn capture_specific(window_id: &str) -> Option<ie::OwnedImage> {
	let windows = xcap::Window::all().ok()?;
	let needle = window_id.to_lowercase();

	// Match either app_name() (exact) or title() (substring), since app names can vary
	// between platforms / launchers (Steam, standalone, Wine, etc).
	let window = windows.iter().find(|window| {
		window
			.app_name()
			.ok()
			.is_some_and(|name| name.eq_ignore_ascii_case(window_id))
			|| window
				.title()
				.ok()
				.is_some_and(|title| title.to_lowercase().contains(&needle))
	})?;

	let img = window.capture_image().ok()?;
	let mut img = ie::OwnedImage::from_rgba(img.width() as usize, img.as_bytes());
	img.resize_h(1080);
	Some(img)
}

/// Reads the config and captures the selected window.
///
/// We clone the window id so we don't hold the config lock during capture.
pub fn capture() -> Option<ie::OwnedImage> {
	let window_id = crate::config_read().app_id.clone();
	capture_specific(&window_id)
}
