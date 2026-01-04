use std::time::{Duration, Instant};

/// Runtime overlay controller.
///
/// When enabled, we:
/// - Keep the window always-on-top + transparent (configured in `main` via `NativeOptions`)
/// - Optionally make it "click-through" (Windows: `WS_EX_TRANSPARENT`)
/// - Follow the target application's window bounds using `xcap`
pub struct OverlayController {
	last_sync: Instant,
	click_through: bool,
	last_effective_click_through: Option<bool>,
}

impl OverlayController {
	pub fn new(click_through: bool) -> Self {
		Self {
			last_sync: Instant::now() - Duration::from_secs(10),
			click_through,
			last_effective_click_through: None,
		}
	}

	pub fn click_through(&self) -> bool {
		self.click_through
	}

	pub fn set_click_through(&mut self, click_through: bool) {
		if self.click_through != click_through {
			self.click_through = click_through;
			self.last_effective_click_through = None;
		}
	}

	pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame, target_app_id: &str) {
		self.apply_platform_styles(frame);

		// Don't spam window moves/resizes; 5 Hz is plenty.
		if self.last_sync.elapsed() < Duration::from_millis(200) {
			return;
		}
		self.last_sync = Instant::now();

		let Ok(windows) = xcap::Window::all() else { return };
		let Some(target) = windows.into_iter().find(|w| w.app_name().ok().as_deref() == Some(target_app_id)) else {
			return;
		};

		let (Ok(x), Ok(y), Ok(w), Ok(h)) = (target.x(), target.y(), target.width(), target.height()) else { return };

		// `ViewportCommand` coordinates are in logical points, not physical pixels.
		let native_ppp = ctx.native_pixels_per_point().unwrap_or(1.0);
		let pos = egui::pos2(x as f32 / native_ppp, y as f32 / native_ppp);
		let size = egui::vec2(w as f32 / native_ppp, h as f32 / native_ppp);

		ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
		ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
	}

	fn apply_platform_styles(&mut self, frame: &mut eframe::Frame) {
		#[cfg(windows)]
		{
			use raw_window_handle::{HasWindowHandle, RawWindowHandle};
			use windows::Win32::Foundation::{HWND, POINT, RECT};
			use windows::Win32::UI::WindowsAndMessaging::{
				GetCursorPos, GetWindowLongPtrW, GetWindowRect, SetLayeredWindowAttributes,
				SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED, WS_EX_TRANSPARENT,
			};

			let hwnd = match frame.window_handle().ok().map(|h| h.as_raw()) {
				Some(RawWindowHandle::Win32(h)) => HWND(h.hwnd.get() as _),
				_ => return,
			};

			// IMPORTANT: When `WS_EX_TRANSPARENT` is set, the window stops receiving mouse
			// events. To allow users to interact without hotkeys, we make the overlay
			// temporarily interactive when the cursor is over it.
			let effective_click_through = if self.click_through {
				unsafe {
					let mut pt = POINT::default();
					let mut rect = RECT::default();
					let cursor_ok = GetCursorPos(&mut pt).as_bool();
					let rect_ok = GetWindowRect(hwnd, &mut rect).as_bool();
					if cursor_ok && rect_ok {
						let inside = pt.x >= rect.left && pt.x <= rect.right && pt.y >= rect.top && pt.y <= rect.bottom;
						!inside
					} else {
						true
					}
				}
			} else {
				false
			};

			if self.last_effective_click_through == Some(effective_click_through) {
				return;
			}

			unsafe {
				let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
				ex_style |= WS_EX_LAYERED.0 as u32;

				if effective_click_through {
					ex_style |= WS_EX_TRANSPARENT.0 as u32;
				} else {
					ex_style &= !(WS_EX_TRANSPARENT.0 as u32);
				}

				_ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style as isize);

				// Ensure the layered window alpha is fully opaque (we rely on the window's transparent
				// background instead of per-window alpha).
				_ = SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0), 255, LWA_ALPHA);
			}

			self.last_effective_click_through = Some(effective_click_through);
		}

		#[cfg(not(windows))]
		{
			let _ = frame;
			self.last_effective_click_through = Some(false);
		}
	}
}
