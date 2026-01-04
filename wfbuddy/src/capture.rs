use xcap::image::EncodableLayout;

pub fn capture_specific(window_id: &str) -> Option<ie::OwnedImage> {
	let windows = xcap::Window::all().ok()?;
	let window = windows
		.iter()
		.filter_map(|window| window.app_name().ok().map(|name| (name, window)))
		.find_map(|(name, window)| if name == window_id {Some(window)} else {None})?;
	
	let img = window.capture_image().ok()?;
	let mut img = ie::OwnedImage::from_rgba(img.width() as usize, img.as_bytes());
	img.resize_h(1080);
	Some(img)
}

/// Reads the config and captures the selected window, will deadlock if a handle to the config already exists
pub fn capture() -> Option<ie::OwnedImage> {
	// capture_specific("steam_app_230410")
	// capture_specific("gwenview")
	capture_specific(&crate::config().app_id)
}