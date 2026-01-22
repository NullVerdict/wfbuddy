//! Screen/window capture utilities.

use anyhow::{anyhow, Context, Result};

/// Basic window descriptor for UI selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub id: u32,
    pub app_name: String,
    pub title: String,
}

impl std::fmt::Display for WindowInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Keep it short but informative.
        if self.title.is_empty() {
            write!(f, "{} (id:{})", self.app_name, self.id)
        } else {
            write!(f, "{} — {} (id:{})", self.app_name, self.title, self.id)
        }
    }
}

pub fn list_windows() -> Result<Vec<WindowInfo>> {
    let windows = xcap::Window::all().context("xcap::Window::all")?;

    let mut out = Vec::with_capacity(windows.len());
    for w in windows {
        // Some platforms may return empty strings; keep them but UI can filter.
        out.push(WindowInfo {
            id: w.id(),
            app_name: w.app_name(),
            title: w.title(),
        });
    }

    // Sort for more stable UX.
    out.sort_by(|a, b| a.app_name.cmp(&b.app_name).then(a.title.cmp(&b.title)));
    Ok(out)
}

/// Capture the first window whose `app_name` matches `target_app_name`.
///
/// If multiple windows share the same `app_name`, the first match is used.
///
/// If `max_height` is set, the capture will be downscaled to that height when
/// larger (preserving aspect ratio).
pub fn capture_by_app_name(target_app_name: &str, max_height: Option<u32>) -> Result<ie::OwnedImage> {
    let windows = xcap::Window::all().context("xcap::Window::all")?;

    let window = windows
        .into_iter()
        .find(|v| v.app_name() == target_app_name)
        .ok_or_else(|| anyhow!("window not found: app_name={target_app_name}"))?;

    let img = window.capture_image().context("xcap::Window::capture_image")?;

    let mut out = ie::OwnedImage::from_rgba(img.width() as usize, img.as_bytes());

    if let Some(max_h) = max_height {
        let h = out.as_image().height();
        if h > max_h {
            out.resize_h(max_h);
        }
    }

    Ok(out)
}
