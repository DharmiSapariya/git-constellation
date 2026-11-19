//! Labels: constellation names, repo names, "first light" and the caption, written
//! inside the boxes the layout reserved for them. Never decides where anything goes.

use crate::layout::labels::{Label, LabelKind};
use crate::model::LabelMode;
use crate::svg::{self, SvgBuilder, fmt};
use crate::theme::{Theme, sky_color_at};

/// Font stacks only: nothing is loaded from outside.
pub const SERIF_STACK: &str = "Georgia, 'Times New Roman', serif";
pub const ROUNDED_STACK: &str =
    "ui-rounded, 'SF Pro Rounded', Nunito, 'Segoe UI', system-ui, sans-serif";
/// Constellation names are spaced out by this share of their font size.
pub const CONSTELLATION_SPACING: f64 = 0.16;
/// Text opacities.
pub const CONSTELLATION_OPACITY: f64 = 0.9;
pub const STAR_LABEL_OPACITY: f64 = 0.95;
pub const DIM_LABEL_OPACITY: f64 = 0.95;
/// Weight of repo names.
pub const STAR_LABEL_WEIGHT: &str = "500";
/// The baseline sits this share of the font size below the middle of the box.
pub const BASELINE_SHIFT: f64 = 0.35;
/// The soft outline behind text, in the local sky color.
pub const HALO_STROKE: f64 = 3.0;
pub const HALO_OPACITY: f64 = 0.55;

/// How one piece of text looks.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub family: &'static str,
    pub size: f64,
    pub weight: Option<&'static str>,
    pub italic: bool,
    /// In pixels; 0 means none.
    pub letter_spacing: f64,
    /// A hex color from the theme.
    pub fill: &'static str,
    pub opacity: f64,
    /// `middle` or `start`.
    pub anchor: &'static str,
}

/// The style of one label (kinds differ in font, spacing, color and opacity).
pub fn label_style(theme: &Theme, label: &Label) -> TextStyle {
    match label.kind {
        LabelKind::Constellation(_) => TextStyle {
            family: SERIF_STACK,
            size: label.font_size,
            weight: None,
            italic: true,
            letter_spacing: CONSTELLATION_SPACING * label.font_size,
            fill: theme.text,
            opacity: CONSTELLATION_OPACITY,
            anchor: "middle",
        },
        LabelKind::Star(_) => TextStyle {
            family: ROUNDED_STACK,
            size: label.font_size,
            weight: Some(STAR_LABEL_WEIGHT),
            italic: false,
            letter_spacing: 0.0,
            fill: theme.text,
            opacity: STAR_LABEL_OPACITY,
            anchor: "middle",
        },
        // The layout measured these two boxes without spacing or capitals, so none here.
        LabelKind::FirstLight(_) | LabelKind::Caption => TextStyle {
            family: SERIF_STACK,
            size: label.font_size,
            weight: None,
            italic: true,
            letter_spacing: 0.0,
            fill: theme.text_dim,
            opacity: DIM_LABEL_OPACITY,
            anchor: "middle",
        },
    }
}

fn base_attributes(style: &TextStyle) -> Vec<(&'static str, String)> {
    let mut attrs = vec![
        ("font-family", style.family.to_string()),
        ("font-size", fmt(style.size)),
        ("text-anchor", style.anchor.to_string()),
    ];
    if let Some(weight) = style.weight {
        attrs.push(("font-weight", weight.to_string()));
    }
    if style.italic {
        attrs.push(("font-style", "italic".to_string()));
    }
    if style.letter_spacing > 0.0 {
        attrs.push(("letter-spacing", fmt(style.letter_spacing)));
    }
    attrs
}

/// Text with a soft halo behind it, as two `<text>` elements: first a copy in
/// the local sky color (hidden from screen readers), then the real text.
/// (`paint-order` alone is not trusted: some renderers ignore it.)
pub fn text_with_halo(x: f64, y: f64, content: &str, style: &TextStyle, halo_hex: &str) -> String {
    let mut halo = base_attributes(style);
    halo.push(("fill", halo_hex.to_string()));
    halo.push(("fill-opacity", fmt(HALO_OPACITY)));
    halo.push(("stroke", halo_hex.to_string()));
    halo.push(("stroke-width", fmt(HALO_STROKE)));
    halo.push(("stroke-opacity", fmt(HALO_OPACITY)));
    halo.push(("stroke-linejoin", "round".to_string()));
    halo.push(("aria-hidden", "true".to_string()));
    let mut real = base_attributes(style);
    real.push(("fill", style.fill.to_string()));
    real.push(("opacity", fmt(style.opacity)));
    let mut out = svg::text(x, y, content, &halo);
    out.push_str(&svg::text(x, y, content, &real));
    out
}

/// The labels that are drawn in a label mode. `none` draws no label at all,
/// not even "first light" or the caption, whatever the layout lists.
pub fn drawn_labels(labels: &[Label], mode: LabelMode) -> &[Label] {
    match mode {
        LabelMode::Off => &[],
        LabelMode::Major | LabelMode::All => labels,
    }
}

/// One label, centered in its reserved box.
pub fn label_markup(theme: &Theme, label: &Label, canvas_height: f64) -> String {
    let center_x = (label.rect.x0 + label.rect.x1) / 2.0;
    let center_y = (label.rect.y0 + label.rect.y1) / 2.0;
    let baseline = center_y + BASELINE_SHIFT * label.font_size;
    let halo = sky_color_at(theme, center_y, canvas_height).to_hex();
    text_with_halo(
        center_x,
        baseline,
        &label.text,
        &label_style(theme, label),
        &halo,
    )
}

/// Draw the `labels` layer (empty when no label is drawn, so the structure stays the same).
pub fn draw_labels(b: &mut SvgBuilder, theme: &Theme, labels: &[Label], mode: LabelMode) {
    let height = b.height();
    let shown = drawn_labels(labels, mode);
    b.group(&[("id", "labels".to_string())], |g| {
        for label in shown {
            g.add(&label_markup(theme, label, height));
        }
    });
}
