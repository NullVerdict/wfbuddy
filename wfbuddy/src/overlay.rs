/// Constants & shared types for the always-on-top overlay viewport.
///
/// The overlay is rendered as a small, borderless native window (an egui viewport)
/// that sits just below the in-game relic reward cards (AlecaFrame-style).
///
/// Note: sizes are in egui "points" (logical pixels).

/// Layout constants for the AlecaFrame-style bar.
///
/// NOTE: `egui::Margin` stores its values as `i8` (to keep it small), so we keep
/// padding as `i8` and expose an `f32` view for sizing math.
pub const BAR_PADDING: i8 = 12;
pub const BAR_PADDING_F32: f32 = BAR_PADDING as f32;
pub const CARD_W: f32 = 220.0;
pub const CARD_SPACING: f32 = 10.0;
pub const CARD_H: f32 = 130.0;

/// Approximate bar height (used for the default vertical anchor ratio).
pub const BAR_H: f32 = CARD_H + BAR_PADDING_F32 * 2.0;

/// Default vertical anchor (center of the overlay) positioned just below the
/// in-game relic reward cards.
///
/// The relic reward cards row is detected on a 1080p capture where:
///   - REWARD_Y = 225
///   - REWARD_SIZE = 235
///
/// so the row bottom is at y = 460.
/// We place the overlay top a small gap below that, then convert to a center anchor ratio.
pub const OVERLAY_DEFAULT_Y_RATIO_BELOW_REWARDS: f32 = (460.0 + 18.0 + (BAR_H / 2.0)) / 1080.0;

/// Small, self-contained data used by the overlay viewport.
#[derive(Debug, Clone)]
pub struct OverlayCard {
	pub name: String,
	/// Optional extra label shown under the item name (e.g. "Чертёж", "Системы").
	pub kind: Option<String>,
	pub vaulted: bool,
	pub platinum: f32,
	pub ducats: u32,
	pub owned: u32,
}
