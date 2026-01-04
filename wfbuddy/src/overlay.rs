/// Constants & shared types for the always-on-top overlay viewport.
///
/// The viewport is rendered as a separate, borderless native window (egui viewport).
/// Sizes are in egui "points" (logical pixels).
pub const OVERLAY_WIDTH: f32 = 940.0;
pub const OVERLAY_HEIGHT: f32 = 190.0;

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
