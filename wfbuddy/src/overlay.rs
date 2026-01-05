/// Constants & shared types for the always-on-top overlay viewport.
///
/// The viewport is rendered as a separate, borderless native window (egui viewport).
/// Sizes are in egui "points" (logical pixels).
/// Estimated card width used to size the viewport (in egui points).
///
/// The actual card uses `ui.set_min_width(210)` plus padding/margins.
pub const OVERLAY_CARD_WIDTH: f32 = 235.0;
pub const OVERLAY_CARD_SPACING_X: f32 = 10.0;

/// Estimated overlay height (in egui points).
///
/// This is used for default positioning and viewport sizing when we can't
/// measure content precisely (egui doesn't provide stable pre-layout sizing).
pub const OVERLAY_HEIGHT_EST: f32 = 190.0;

/// Default vertical anchor (center of the overlay) positioned just below the in-game relic reward cards.
///
/// The relic reward cards row is detected on a 1080p capture where:
///   - REWARD_Y = 225
///   - REWARD_SIZE = 235
///
/// so the row bottom is at y = 460.
/// We place the overlay top a small gap below that, then convert to a center anchor ratio.
pub const OVERLAY_DEFAULT_Y_RATIO_BELOW_REWARDS: f32 =
	(460.0 + 18.0 + (OVERLAY_HEIGHT_EST / 2.0)) / 1080.0;

/// Small, self-contained data used by the always-on-top overlay viewport.
#[derive(Debug, Clone)]
pub struct OverlayCard {
	pub name: String,
	/// Best-effort type label (e.g. "Blueprint", "PrimePart").
	pub item_type: Option<String>,
	/// Best-effort Russian label for `item_type`.
	pub item_type_ru: Option<&'static str>,
	pub vaulted: bool,
	pub platinum: f32,
	pub ducats: u32,
	pub owned: u32,
}
