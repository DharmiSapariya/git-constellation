//! The HUD: the sparkle, the "@name's sky" title and the one-line stats.

use crate::layout::text::{text_width, truncate_chars};
use crate::model::{Point, SkyModel};
use crate::svg::{self, SvgBuilder, fmt};
use crate::theme::{Theme, sky_color_at};

use super::labels::{ROUNDED_STACK, TextStyle, text_with_halo};
use super::stars::sparkle_path;

/// The sparkle icon: where it sits and the star radius it is drawn with.
pub const ICON_CENTER: Point = (54.0, 44.0);
pub const ICON_RADIUS: f64 = 4.0;
/// The title: where it starts (left edge, baseline), size and weight.
pub const TITLE_POS: Point = (76.0, 50.0);
pub const TITLE_SIZE: f64 = 15.0;
pub const TITLE_WEIGHT: &str = "600";
/// The usernames shown are at most this long, and the title never reaches past this x.
pub const HUD_USERNAME_MAX_CHARS: usize = 24;
pub const HUD_TITLE_MAX_X: f64 = 326.0;
/// The stats line: where it starts, size and opacity.
pub const SUBTITLE_POS: Point = (40.0, 68.0);
pub const SUBTITLE_SIZE: f64 = 11.0;
pub const SUBTITLE_OPACITY: f64 = 0.85;
/// Between the three stats.
pub const SUBTITLE_SEPARATOR: &str = "  \u{b7}  ";

/// The username as shown in the title: at most 24 characters, and short enough
/// that the conservative width estimate stays left of `HUD_TITLE_MAX_X`.
pub fn display_username(username: &str) -> String {
    let mut limit = HUD_USERNAME_MAX_CHARS;
    loop {
        let shown = truncate_chars(username, limit);
        let title = title_text(&shown);
        let fits = TITLE_POS.0 + text_width(&title, TITLE_SIZE, false) <= HUD_TITLE_MAX_X;
        if fits || limit <= 1 {
            return shown;
        }
        limit -= 1;
    }
}

/// `@name's sky`.
pub fn title_text(shown_username: &str) -> String {
    format!("@{shown_username}'s sky")
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// `{N} stars  ·  {C} constellations  ·  {S}-day streak` (with singular forms).
pub fn subtitle_text(model: &SkyModel) -> String {
    let streak = if model.current_streak == 0 {
        "no streak yet".to_string()
    } else {
        format!("{}-day streak", model.current_streak)
    };
    [
        plural(model.stars.len(), "star", "stars"),
        plural(
            model.constellations.len(),
            "constellation",
            "constellations",
        ),
        streak,
    ]
    .join(SUBTITLE_SEPARATOR)
}

/// Draw the `hud` layer.
pub fn draw_hud(b: &mut SvgBuilder, theme: &Theme, model: &SkyModel) {
    let height = b.height();
    let halo = sky_color_at(theme, TITLE_POS.1, height).to_hex();
    let title_style = TextStyle {
        family: ROUNDED_STACK,
        size: TITLE_SIZE,
        weight: Some(TITLE_WEIGHT),
        italic: false,
        letter_spacing: 0.0,
        fill: theme.text,
        opacity: 1.0,
        anchor: "start",
    };
    b.group(&[("id", "hud".to_string())], |g| {
        g.add(&svg::path(
            &sparkle_path(ICON_RADIUS),
            &[
                ("fill", theme.accent.to_string()),
                (
                    "transform",
                    format!("translate({} {})", fmt(ICON_CENTER.0), fmt(ICON_CENTER.1)),
                ),
            ],
        ));
        let title = title_text(&display_username(&model.username));
        g.add(&text_with_halo(
            TITLE_POS.0,
            TITLE_POS.1,
            &title,
            &title_style,
            &halo,
        ));
        g.add(&svg::text(
            SUBTITLE_POS.0,
            SUBTITLE_POS.1,
            &subtitle_text(model),
            &[
                ("font-family", ROUNDED_STACK.to_string()),
                ("font-size", fmt(SUBTITLE_SIZE)),
                ("fill", theme.text_dim.to_string()),
                ("opacity", fmt(SUBTITLE_OPACITY)),
            ],
        ));
    });
}
