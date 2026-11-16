//! PNG and GIF export.
//! (PNG from SVG exists so far; GIF export arrives in a later milestone.)

use crate::SkyError;

/// Rasterize SVG text to a PNG at the given scale (1.0 keeps the SVG's size).
/// Text is not drawn unless fonts are available, so callers that need text
/// must not rely on this alone.
pub fn svg_to_png(svg: &str, scale: f32) -> Result<Vec<u8>, SkyError> {
    let render_error = |what: String| SkyError::Render(what);
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .map_err(|error| render_error(format!("the SVG could not be read: {error}")))?;
    let size = tree.size();
    let (width, height) = (
        (size.width() * scale).ceil() as u32,
        (size.height() * scale).ceil() as u32,
    );
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| render_error(format!("cannot make a {width}x{height} image")))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|error| SkyError::Export(format!("PNG encoding failed: {error}")))
}
