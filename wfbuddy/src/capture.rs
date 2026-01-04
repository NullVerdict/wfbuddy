use xcap::image::EncodableLayout;

/// Geometry information for the captured application window.
#[derive(Debug, Clone, Copy)]
pub struct WindowBounds {
	pub x: f32,
	pub y: f32,
	pub width: f32,
	pub height: f32,
	/// Monitor scale factor (physical pixels per logical point).
	pub scale_factor: f32,
}

pub fn find_window(window_id: &str) -> Option<xcap::Window> {
	let windows = xcap::Window::all().ok()?;
	windows
		.into_iter()
		.find(|window| window.app_name().ok().as_deref() == Some(window_id))
}

pub fn window_bounds(window_id: &str) -> Option<WindowBounds> {
	let window = find_window(window_id)?;
	let scale_factor = window
		.current_monitor()
		.ok()
		.and_then(|m| m.scale_factor().ok())
		.unwrap_or(1.0);

	Some(WindowBounds {
		x: window.x().ok()? as f32,
		y: window.y().ok()? as f32,
		width: window.width().ok()? as f32,
		height: window.height().ok()? as f32,
		scale_factor,
	})
}

pub fn capture_specific(window_id: &str) -> Option<ie::OwnedImage> {
	let window = find_window(window_id)?;
	let img = window.capture_image().ok()?;
	Some(ie::OwnedImage::from_rgba(img.width() as usize, img.as_bytes()))
}

/// Reads the config and captures the selected window.
///
/// Note: this will deadlock if a handle to the config already exists.
pub fn capture() -> Option<ie::OwnedImage> {
	let app_id = { crate::config().app_id.clone() };
	capture_specific(&app_id)
}
