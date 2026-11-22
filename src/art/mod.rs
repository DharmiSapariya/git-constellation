//! Art: how each part of the sky looks (never decides data).

pub mod background;
pub mod hud;
pub mod labels;
pub mod lines;
pub mod milkyway;
pub mod moon;
pub mod nebula;
pub mod shooting;
pub mod sprites;
pub mod star_parts;
pub mod stars;

use crate::layout::anchors::SeededRng;
use crate::layout::geometry::star_radius;
use crate::layout::{Layout, Rect};
use crate::model::{LabelMode, SkyModel};
use crate::rules::MAX_DUST_COUNT;
use crate::svg::SvgBuilder;
use crate::theme::Theme;

/// Random-stream layer names. Each layer has its own stream, so adding randomness
/// to one layer never changes another.
pub const NEBULA: &str = "nebula";
pub const DUST: &str = "dust";
pub const MILKYWAY: &str = "milkyway";
pub const GRAIN: &str = "grain";
pub const FIBERS: &str = "paper-fibers";
pub const STARS: &str = "stars";

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
        &[],
    );
    milkyway::draw_milkyway(&mut b, theme, &model.milkyway, model.seed, avoid, &[]);
    if options.grain {
        background::draw_grain(&mut b, model.seed);
    }
    b.finish()
}

/// The whole sky: the background, then lines, stars, labels and the HUD.
/// `labels` is the label mode (`none` draws no label at all).
pub fn sky_svg(
    model: &SkyModel,
    layout: &Layout,
    theme: &Theme,
    options: &ArtOptions,
    labels: LabelMode,
) -> String {
    compose(model, layout, theme, options, labels, true).0
}

/// The same sky with no text at all (no labels, no HUD). The linter renders this
/// to measure what is behind each label. Dust and Milky Way dots still keep clear
/// of the labels that would be there, so the picture matches the real one.
pub fn sky_svg_without_text(
    model: &SkyModel,
    layout: &Layout,
    theme: &Theme,
    options: &ArtOptions,
    labels: LabelMode,
) -> String {
    compose(model, layout, theme, options, labels, false).0
}

/// Like `sky_svg`, and also says how many constellation lines were too short to draw.
pub fn sky_svg_with_stats(
    model: &SkyModel,
    layout: &Layout,
    theme: &Theme,
    options: &ArtOptions,
    labels: LabelMode,
) -> (String, usize) {
    compose(model, layout, theme, options, labels, true)
}

fn compose(
    model: &SkyModel,
    layout: &Layout,
    theme: &Theme,
    options: &ArtOptions,
    labels: LabelMode,
    with_text: bool,
) -> (String, usize) {
    let mut b = SvgBuilder::new(
        options.width,
        options.height,
        &format!("{}'s constellation sky", model.username),
        "A night sky of stars, constellation lines and a Milky Way of contributions.",
    );
    let circles: Vec<(f64, f64, f64)> = model
        .stars
        .iter()
        .zip(&layout.positions)
        .map(|(star, position)| (position.0, position.1, star_radius(star.tier)))
        .collect();
    let shown = labels::drawn_labels(&layout.labels, labels);
    let rects: Vec<Rect> = shown.iter().map(|label| label.rect).collect();
    background::draw_sky(&mut b, theme);
    background::draw_nebula(&mut b, theme, model.seed);
    background::draw_dust(
        &mut b,
        theme,
        model.seed,
        model.dust_count.min(MAX_DUST_COUNT),
        &circles,
        &rects,
    );
    milkyway::draw_milkyway(&mut b, theme, &model.milkyway, model.seed, &circles, &rects);
    if options.grain {
        background::draw_grain(&mut b, model.seed);
    }
    let skipped = lines::draw_lines(&mut b, theme, model, layout);
    stars::draw_stars(&mut b, theme, model, &layout.positions, &rects);
    if with_text {
        labels::draw_labels(&mut b, theme, &layout.labels, labels);
        hud::draw_hud(&mut b, theme, model);
    }
    (b.finish(), skipped)
}
