/// Constants & shared types for the always-on-top overlay viewport.
///
/// The viewport is rendered as a separate, borderless native window (egui viewport).
/// Sizes are in egui "points" (logical pixels).
///
/// The overlay size is computed dynamically based on the number of cards,
/// but we keep a couple sane bounds here.
pub const OVERLAY_MAX_WIDTH: f32 = 1260.0;

/// We show up to 4 reward cards (same as the in-game row without scrolling).
pub const OVERLAY_MAX_CARDS: usize = 4;

/// Card dimensions in egui points.
pub const OVERLAY_CARD_WIDTH: f32 = 255.0;
pub const OVERLAY_CARD_HEIGHT: f32 = 118.0;

/// egui stores `Margin` values as `i8` (for compactness), so keep a pixel value for that,
/// and a `f32` version for layout calculations.
pub const OVERLAY_PADDING_PX: i8 = 14;
pub const OVERLAY_PADDING_F32: f32 = OVERLAY_PADDING_PX as f32;
pub const OVERLAY_SPACING: f32 = 12.0;

/// Height of the footer/status bar under the cards.
pub const OVERLAY_FOOTER_HEIGHT: f32 = 22.0;

// Includes card row + a small hint/status bar.
pub const OVERLAY_HEIGHT: f32 = OVERLAY_CARD_HEIGHT + (OVERLAY_PADDING_F32 * 2.0) + OVERLAY_FOOTER_HEIGHT;

/// Default vertical anchor (center of the overlay) positioned just below the in-game relic reward cards.
///
/// The relic reward cards row is detected on a 1080p capture where:
///   - REWARD_Y = 225
///   - REWARD_SIZE = 235
///
/// so the row bottom is at y = 460.
/// We place the overlay top a small gap below that, then convert to a center anchor ratio.
pub const OVERLAY_DEFAULT_Y_RATIO_BELOW_REWARDS: f32 = (460.0 + 18.0 + (OVERLAY_HEIGHT / 2.0)) / 1080.0;

/// Small, self-contained data used by the always-on-top overlay viewport.
#[derive(Debug, Clone)]
pub struct OverlayCard {
	pub name: String,
	pub vaulted: bool,
	pub platinum: f32,
	pub ducats: u32,
	pub owned: u32,
}
