//! PNG and SVG export, and the `render` command's engine.
//! (GIF export arrives in a later milestone.)

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, OnceLock};

use crate::art::{ArtOptions, sky_svg};
use crate::model::Activity;
use crate::theme::Theme;
use crate::{Options, SkyError, plan_sky};

/// No output side may be larger than this many pixels.
pub const MAX_SIDE: u32 = 8000;

/// Why an image could not be made.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExportError {
    #[error("the SVG could not be read: {0}")]
    Parse(String),
    #[error(
        "the image would be {width} x {height} pixels; each side must be between 1 and {MAX_SIDE}"
    )]
    BadSize { width: u64, height: u64 },
    #[error("the scale must be a positive number, not {0}")]
    BadScale(String),
    #[error("the PNG could not be written: {0}")]
    Encode(String),
}

impl From<ExportError> for SkyError {
    fn from(error: ExportError) -> Self {
        SkyError::Export(error.to_string())
    }
}

/// The system fonts, loaded once (loading them is slow). Without them text would not be drawn.
static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();

fn font_database() -> Arc<usvg::fontdb::Database> {
    FONTS
        .get_or_init(|| {
            let mut database = usvg::fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

fn svg_options() -> usvg::Options<'static> {
    usvg::Options {
        fontdb: font_database(),
        ..usvg::Options::default()
    }
}

/// Draw `region` (`x, y, width, height` of the SVG's own coordinates; `None` for all of it)
/// at `scale`, and encode it as a PNG.
fn rasterize(
    svg: &str,
    region: Option<(f64, f64, f64, f64)>,
    scale: f32,
) -> Result<Vec<u8>, ExportError> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(ExportError::BadScale(scale.to_string()));
    }
    let tree = usvg::Tree::from_str(svg, &svg_options())
        .map_err(|error| ExportError::Parse(error.to_string()))?;
    let size = tree.size();
    let (x, y, width, height) =
        region.unwrap_or((0.0, 0.0, f64::from(size.width()), f64::from(size.height())));
    let (w, h) = (
        (width * f64::from(scale)).round(),
        (height * f64::from(scale)).round(),
    );
    let max = f64::from(MAX_SIDE);
    if !(1.0..=max).contains(&w) || !(1.0..=max).contains(&h) {
        return Err(ExportError::BadSize {
            width: w as u64,
            height: h as u64,
        });
    }
    let mut pixmap = tiny_skia::Pixmap::new(w as u32, h as u32).ok_or(ExportError::BadSize {
        width: w as u64,
        height: h as u64,
    })?;
    let transform =
        tiny_skia::Transform::from_scale(scale, scale).pre_translate(-(x as f32), -(y as f32));
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap
        .encode_png()
        .map_err(|error| ExportError::Encode(error.to_string()))
}

/// Rasterize SVG text to a PNG at the given scale (1.0 keeps the SVG's size).
/// Text is drawn with the system fonts.
pub fn svg_to_png(svg: &str, scale: f32) -> Result<Vec<u8>, ExportError> {
    rasterize(svg, None, scale)
}

/// Rasterize only a rectangle of the SVG (`x`, `y`, `width`, `height` in its own
/// coordinates), zoomed by `scale`. For close-up review pictures.
pub fn svg_to_png_region(
    svg: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale: f32,
) -> Result<Vec<u8>, ExportError> {
    rasterize(svg, Some((x, y, width, height)), scale)
}

/// The output file type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Svg,
    Png,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Svg => "svg",
            Format::Png => "png",
        }
    }
}

impl FromStr for Format {
    type Err = SkyError;

    fn from_str(text: &str) -> Result<Self, SkyError> {
        match text.trim().to_lowercase().as_str() {
            "svg" => Ok(Format::Svg),
            "png" => Ok(Format::Png),
            other => Err(SkyError::InvalidOption(format!(
                "unknown format `{other}`. Choose svg or png."
            ))),
        }
    }
}

/// Everything the `render` command needs.
#[derive(Debug, Clone)]
pub struct RenderRequest<'a> {
    pub from_file: &'a Path,
    pub out: Option<&'a Path>,
    pub format: Format,
    pub scale: f32,
    pub grain: bool,
    pub options: &'a Options,
}

/// The finished sky as SVG text.
pub fn render_svg(activity: &Activity, options: &Options, grain: bool) -> Result<String, SkyError> {
    let (model, layout) = plan_sky(activity, options)?;
    let theme = Theme::for_name(options.theme);
    let art = ArtOptions {
        grain: grain || options.paper_grain,
        ..ArtOptions::default()
    };
    Ok(sky_svg(&model, &layout, &theme, &art, options.labels))
}

/// Make the picture in memory: SVG text, or a PNG of it.
pub fn render_bytes(
    activity: &Activity,
    options: &Options,
    format: Format,
    scale: f32,
    grain: bool,
) -> Result<Vec<u8>, SkyError> {
    let svg = render_svg(activity, options, grain)?;
    match format {
        Format::Svg => Ok(svg.into_bytes()),
        Format::Png => Ok(svg_to_png(&svg, scale)?),
    }
}

/// Read the activity file, render, write the result. Returns where it went.
pub fn render_file(request: &RenderRequest) -> Result<PathBuf, SkyError> {
    let text = std::fs::read_to_string(request.from_file).map_err(|e| {
        SkyError::Export(format!(
            "could not read {}: {e}",
            request.from_file.display()
        ))
    })?;
    let activity = Activity::from_json(&text)?;
    let bytes = render_bytes(
        &activity,
        request.options,
        request.format,
        request.scale,
        request.grain,
    )?;
    let out = request.out.map_or_else(
        || PathBuf::from(format!("constellation.{}", request.format.extension())),
        Path::to_path_buf,
    );
    std::fs::write(&out, bytes)
        .map_err(|e| SkyError::Export(format!("could not write {}: {e}", out.display())))?;
    Ok(out)
}

/// Lay several same-sized PNGs out in a grid (left to right, top to bottom),
/// for side-by-side review pictures.
pub fn contact_sheet(images: &[Vec<u8>], columns: usize) -> Result<Vec<u8>, ExportError> {
    let bad = |w: u64, h: u64| ExportError::BadSize {
        width: w,
        height: h,
    };
    if images.is_empty() || columns == 0 {
        return Err(bad(0, 0));
    }
    let pixmaps = images
        .iter()
        .map(|bytes| {
            tiny_skia::Pixmap::decode_png(bytes).map_err(|e| ExportError::Parse(e.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (cell_w, cell_h) = (pixmaps[0].width(), pixmaps[0].height());
    let rows = images.len().div_ceil(columns);
    let (width, height) = (
        u64::from(cell_w) * columns as u64,
        u64::from(cell_h) * rows as u64,
    );
    if width > u64::from(MAX_SIDE) || height > u64::from(MAX_SIDE) {
        return Err(bad(width, height));
    }
    let mut sheet =
        tiny_skia::Pixmap::new(width as u32, height as u32).ok_or_else(|| bad(width, height))?;
    for (i, pixmap) in pixmaps.iter().enumerate() {
        let (x, y) = ((i % columns) as u32 * cell_w, (i / columns) as u32 * cell_h);
        sheet.draw_pixmap(
            x as i32,
            y as i32,
            pixmap.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );
    }
    sheet
        .encode_png()
        .map_err(|e| ExportError::Encode(e.to_string()))
}
