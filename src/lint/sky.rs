//! Checks that need the drawn sky: SVG sanity, text contrast and how visible each star is.

use std::collections::BTreeSet;

use super::Check;
use crate::SkyError;
use crate::art::labels::{drawn_labels, label_style};
use crate::art::{ArtOptions, sky_svg_with_stats, sky_svg_without_text};
use crate::export::svg_to_png;
use crate::layout::{Layout, Rect, star_radius};
use crate::model::{LabelMode, SkyModel};
use crate::theme::{Rgb, Theme, contrast_ratio, luma, parse_hex};

/// Hard cap on the SVG's size, and the softer size that only warns.
pub const MAX_SVG_BYTES: usize = 300 * 1024;
pub const SOFT_SVG_BYTES: usize = 180 * 1024;
/// Every label's text must reach this WCAG contrast over what is behind it.
pub const MIN_TEXT_CONTRAST: f64 = 4.5;
/// A star's center is compared with a ring of pixels this far out (inner, outer radius).
pub const VISIBILITY_RING: (f64, f64) = (30.0, 40.0);
/// The center must differ from the ring's median luma by this share (brighter on dark skies,
/// darker on paper).
pub const VISIBILITY_MARGIN: f64 = 0.25;
/// "Bigger stars" for the second visibility number start at this tier.
pub const VISIBILITY_BIG_TIER: u8 = 3;

/// The hard and soft checks that come from the drawn sky.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawnChecks {
    pub hard: Vec<Check>,
    pub soft: Vec<Check>,
}

fn check(name: &'static str, passed: bool, detail: String) -> Check {
    Check {
        name,
        passed,
        detail,
    }
}

/// True when any attribute value holds a number that is NaN or infinite.
/// Hex colors (`#7e4569` would read as 7e4569, an enormous number) are skipped.
pub fn has_non_finite_number(svg: &str) -> bool {
    let mut rest = svg;
    while let Some(start) = rest.find("=\"") {
        rest = &rest[start + 2..];
        let end = rest.find('"').unwrap_or(rest.len());
        let value = &rest[..end];
        let separators = |c: char| c.is_whitespace() || matches!(c, ',' | '(' | ')');
        if value
            .split(separators)
            .filter(|token| !token.starts_with('#'))
            .any(|token| token.parse::<f64>().is_ok_and(|n| !n.is_finite()))
        {
            return true;
        }
        rest = &rest[end..];
    }
    false
}

/// Every value of an attribute written as ` name="value"`.
fn attribute_values<'a>(svg: &'a str, name: &str) -> Vec<&'a str> {
    let needle = format!(" {name}=\"");
    let mut found = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find(&needle) {
        rest = &rest[start + needle.len()..];
        let end = rest.find('"').unwrap_or(rest.len());
        found.push(&rest[..end]);
        rest = &rest[end..];
    }
    found
}

/// Every id that a `url(#id)` points at.
fn url_references(svg: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find("url(#") {
        rest = &rest[start + 5..];
        let end = rest.find(')').unwrap_or(rest.len());
        found.push(&rest[..end]);
        rest = &rest[end..];
    }
    found
}

/// The hard checks that need only the SVG text.
pub fn svg_hard_checks(svg: &str) -> Vec<Check> {
    let ids = attribute_values(svg, "id");
    let unique: BTreeSet<&str> = ids.iter().copied().collect();
    let broken: Vec<&str> = url_references(svg)
        .into_iter()
        .filter(|id| !unique.contains(id))
        .collect();
    vec![
        check(
            "SVG size",
            svg.len() < MAX_SVG_BYTES,
            format!("{} bytes (limit {MAX_SVG_BYTES})", svg.len()),
        ),
        check(
            "finite numbers",
            !has_non_finite_number(svg),
            "no NaN or infinity in any attribute".to_string(),
        ),
        check(
            "references resolve",
            broken.is_empty(),
            format!(
                "{} url(#id) references, {} broken",
                url_references(svg).len(),
                broken.len()
            ),
        ),
        check(
            "ids are unique",
            unique.len() == ids.len(),
            format!("{} ids, {} distinct", ids.len(), unique.len()),
        ),
    ]
}

fn pixmap_of(png: &[u8]) -> Result<tiny_skia::Pixmap, SkyError> {
    tiny_skia::Pixmap::decode_png(png)
        .map_err(|e| SkyError::Render(format!("the PNG could not be read for linting: {e}")))
}

/// The mean color of the pixels inside `rect` (clipped to the picture).
fn mean_color(pixmap: &tiny_skia::Pixmap, rect: &Rect) -> Option<(f64, f64, f64)> {
    let (w, h) = (i64::from(pixmap.width()), i64::from(pixmap.height()));
    let (x0, x1) = (
        (rect.x0.floor() as i64).max(0),
        (rect.x1.ceil() as i64).min(w),
    );
    let (y0, y1) = (
        (rect.y0.floor() as i64).max(0),
        (rect.y1.ceil() as i64).min(h),
    );
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let pixels = pixmap.pixels();
    let (mut r, mut g, mut b, mut n) = (0.0, 0.0, 0.0, 0.0);
    for y in y0..y1 {
        for x in x0..x1 {
            let p = pixels[(y * w + x) as usize];
            r += f64::from(p.red());
            g += f64::from(p.green());
            b += f64::from(p.blue());
            n += 1.0;
        }
    }
    Some((r / n, g / n, b / n))
}

fn to_rgb(color: (f64, f64, f64)) -> Rgb {
    let byte = |v: f64| v.round().clamp(0.0, 255.0) as u8;
    Rgb {
        r: byte(color.0),
        g: byte(color.1),
        b: byte(color.2),
    }
}

/// The hard text-contrast check: for every drawn label, the label's text color,
/// at its drawn opacity, over the mean color behind its box in the sky drawn
/// without any text, must reach `MIN_TEXT_CONTRAST`.
pub fn text_contrast_check(
    model: &SkyModel,
    layout: &Layout,
    theme: &Theme,
    art: &ArtOptions,
    mode: LabelMode,
) -> Result<Check, SkyError> {
    let svg = sky_svg_without_text(model, layout, theme, art, mode);
    let pixmap = pixmap_of(&svg_to_png(&svg, 1.0)?)?;
    let shown = drawn_labels(&layout.labels, mode);
    let mut worst: Option<(f64, String)> = None;
    for label in shown {
        let Some(behind) = mean_color(&pixmap, &label.rect) else {
            continue;
        };
        let style = label_style(theme, label);
        let ink = parse_hex(style.fill).unwrap_or(Rgb {
            r: 255,
            g: 255,
            b: 255,
        });
        let blend = |ink_value: u8, behind_value: f64| {
            f64::from(ink_value) * style.opacity + behind_value * (1.0 - style.opacity)
        };
        let text = to_rgb((
            blend(ink.r, behind.0),
            blend(ink.g, behind.1),
            blend(ink.b, behind.2),
        ));
        let ratio = contrast_ratio(text, to_rgb(behind));
        if worst.as_ref().is_none_or(|(w, _)| ratio < *w) {
            worst = Some((ratio, label.text.clone()));
        }
    }
    Ok(match worst {
        Some((ratio, text)) => check(
            "text contrast",
            ratio >= MIN_TEXT_CONTRAST,
            format!(
                "{} labels, worst {ratio:.2} on \"{text}\" (want at least {MIN_TEXT_CONTRAST})",
                shown.len()
            ),
        ),
        None => check("text contrast", true, "no labels are drawn".to_string()),
    })
}

/// How many stars stand out from their surroundings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Visibility {
    pub measured: usize,
    pub share: f64,
    pub big_measured: usize,
    pub big_share: f64,
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(values[values.len() / 2])
}

/// The star-visibility measurement on a rendered PNG (scale 1): the brightest pixel of the 3 by 3
/// block at each star on dark themes, or the darkest pixel of its body disc on light themes,
/// against the median luma of a ring 30 to 40 px out.
pub fn star_visibility(
    png: &[u8],
    model: &SkyModel,
    layout: &Layout,
    dark: bool,
) -> Result<Visibility, SkyError> {
    let pixmap = pixmap_of(png)?;
    let (w, h) = (i64::from(pixmap.width()), i64::from(pixmap.height()));
    let pixels = pixmap.pixels();
    let luma_at = |x: i64, y: i64| -> Option<f64> {
        if x < 0 || y < 0 || x >= w || y >= h {
            return None;
        }
        let p = pixels[(y * w + x) as usize];
        Some(luma(Rgb {
            r: p.red(),
            g: p.green(),
            b: p.blue(),
        }))
    };
    let reach = VISIBILITY_RING.1.ceil() as i64;
    let (mut measured, mut passed, mut big_measured, mut big_passed) =
        (0usize, 0usize, 0usize, 0usize);
    for (star, position) in model.stars.iter().zip(&layout.positions) {
        let (cx, cy) = (position.0.round() as i64, position.1.round() as i64);
        // Dark skies: the star's hot center is a few pixels wide, so its 3 x 3 block is
        // sampled. Paper: the center is a pale core, and what makes a star visible is its inked
        // body, so the whole body disc is sampled.
        let sample = if dark {
            1
        } else {
            (star_radius(star.tier).ceil() as i64).max(1)
        };
        let mut block = Vec::new();
        for dy in -sample..=sample {
            for dx in -sample..=sample {
                if dark || dx * dx + dy * dy <= sample * sample {
                    if let Some(v) = luma_at(cx + dx, cy + dy) {
                        block.push(v);
                    }
                }
            }
        }
        let center = if dark {
            block.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        } else {
            block.iter().copied().fold(f64::INFINITY, f64::min)
        };
        let mut ring = Vec::new();
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let d = ((dx * dx + dy * dy) as f64).sqrt();
                if (VISIBILITY_RING.0..=VISIBILITY_RING.1).contains(&d) {
                    if let Some(v) = luma_at(cx + dx, cy + dy) {
                        ring.push(v);
                    }
                }
            }
        }
        let (Some(around), true) = (median(&mut ring), center.is_finite()) else {
            continue;
        };
        let stands_out = if dark {
            center >= around * (1.0 + VISIBILITY_MARGIN)
        } else {
            center <= around * (1.0 - VISIBILITY_MARGIN)
        };
        measured += 1;
        passed += usize::from(stands_out);
        if star.tier.number() >= VISIBILITY_BIG_TIER {
            big_measured += 1;
            big_passed += usize::from(stands_out);
        }
    }
    let share = |part: usize, whole: usize| {
        if whole == 0 {
            1.0
        } else {
            part as f64 / whole as f64
        }
    };
    Ok(Visibility {
        measured,
        share: share(passed, measured),
        big_measured,
        big_share: share(big_passed, big_measured),
    })
}

/// All the checks that need the drawn sky, for one sky in one theme.
pub fn drawn_checks(
    model: &SkyModel,
    layout: &Layout,
    theme: &Theme,
    art: &ArtOptions,
    mode: LabelMode,
) -> Result<DrawnChecks, SkyError> {
    let (svg, skipped) = sky_svg_with_stats(model, layout, theme, art, mode);
    let mut hard = svg_hard_checks(&svg);
    hard.extend(super::ring_flare_checks(&svg, model, layout, mode));
    hard.push(text_contrast_check(model, layout, theme, art, mode)?);

    let png = svg_to_png(&svg, 1.0)?;
    let mut soft = vec![
        check(
            "SVG size (soft)",
            svg.len() <= SOFT_SVG_BYTES,
            format!("{} bytes (want at most {SOFT_SVG_BYTES})", svg.len()),
        ),
        check(
            "constellation lines too short to draw",
            skipped == 0,
            format!("{skipped} skipped"),
        ),
        check(
            "labels without a clean spot",
            layout.dropped_labels == 0,
            format!("{} dropped", layout.dropped_labels),
        ),
    ];
    soft.extend(super::pixel_checks(&png, theme.is_dark)?);
    let seen = star_visibility(&png, model, layout, theme.is_dark)?;
    soft.push(check(
        "stars stand out",
        true,
        format!(
            "{:.0}% of {} stars, {:.0}% of {} stars at tier {VISIBILITY_BIG_TIER}+ \
             (center {}% {} than the ring's median)",
            seen.share * 100.0,
            seen.measured,
            seen.big_share * 100.0,
            seen.big_measured,
            (VISIBILITY_MARGIN * 100.0) as u32,
            if theme.is_dark { "brighter" } else { "darker" },
        ),
    ));
    Ok(DrawnChecks { hard, soft })
}
