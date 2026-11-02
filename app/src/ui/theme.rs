//! Spacing and sizes; colours come from egui's light/dark visuals, which
//! follow the Windows theme.

use eframe::egui::{Context, vec2};

pub const SPACE_XS: f32 = 4.0;
pub const SPACE_S: f32 = 8.0;
pub const SPACE_M: f32 = 16.0;
pub const SPACE_L: f32 = 24.0;
/// Form width: long lines are hard to read.
pub const FORM_MAX_WIDTH: f32 = 600.0;
pub const SIDE_PANEL_WIDTH: f32 = 240.0;
/// Corners of controls and of a highlighted card.
pub const RADIUS: u8 = 4;

pub fn apply(ctx: &Context) {
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = vec2(SPACE_S, SPACE_S);
        s.spacing.button_padding = vec2(12.0, 4.0);
        s.spacing.interact_size.y = 24.0;
    });
}
