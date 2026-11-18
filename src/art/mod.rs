//! Art: how each part of the sky looks (never decides data).

pub mod background;
pub mod hud;
pub mod lines;
pub mod milkyway;
pub mod moon;
pub mod shooting;
pub mod sprites;
pub mod stars;

use crate::layout::anchors::SeededRng;
use crate::model::SkyModel;
use crate::rules::MAX_DUST_COUNT;
use crate::svg::SvgBuilder;
use crate::theme::Theme;

/// Random-stream layer names. Each layer has its own stream, so adding randomness
/// to one layer never changes another.
pub const NEBULA: &str = "nebula";
pub const DUST: &str = "dust";
pub const MILKYWAY: &str = "milkyway";
pub const GRAIN: &str = "grain";

/// The seeded random stream for one art layer: the sky's seed, mixed with the
/// layer's name by the hand-written FNV-1a hash.
pub fn layer_rng(seed: u64, layer: &str) -> SeededRng {
    SeededRng::for_layer(seed, layer, 0, 0)
}

/// Settings for drawing the sky.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArtOptions {
    pub width: f64,
    pub height: f64,
    /// Film grain (paper theme and others, off by default).
    pub grain: bool,
}

impl Default for ArtOptions {
    fn default() -> Self {
        ArtOptions {
            width: 1200.0,
            height: 600.0,
            grain: false,
        }
    }
}

/// Only the background layers: gradient and glow, nebula, dust, the Milky Way,
/// and optional grain. `avoid` lists `(x, y, radius)` of things dust must keep
/// away from (stars, later).
pub fn background_svg(
    model: &SkyModel,
    theme: &Theme,
    options: &ArtOptions,
    avoid: &[(f64, f64, f64)],
) -> String {
    let mut b = SvgBuilder::new(
        options.width,
        options.height,
        &format!("{}'s constellation sky", model.username),
        "A night sky: gradient, nebula, dust and a Milky Way band of contributions.",
    );
    background::draw_sky(&mut b, theme);
    background::draw_nebula(&mut b, theme, model.seed);
    background::draw_dust(
        &mut b,
        theme,
        model.seed,
        model.dust_count.min(MAX_DUST_COUNT),
        avoid,
    );
    milkyway::draw_milkyway(&mut b, theme, &model.milkyway, model.seed);
    if options.grain {
        background::draw_grain(&mut b, model.seed);
    }
    b.finish()
}
