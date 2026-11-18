//! PNG export: sizes, limits, errors and the contact sheet.

use git_constellation::SkyError;
use git_constellation::export::{ExportError, MAX_SIDE, contact_sheet, svg_to_png};

fn svg(w: u32, h: u32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\"><rect width=\"{w}\" height=\"{h}\" fill=\"#336699\"/></svg>"
    )
}

fn size(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "PNG signature");
    let read = |at: usize| u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]);
    (read(16), read(20))
}

#[test]
fn a_png_has_the_signature_and_the_right_size_at_every_scale() {
    for (scale, want) in [
        (1.0, (120, 60)),
        (2.0, (240, 120)),
        (0.5, (60, 30)),
        (1.5, (180, 90)),
    ] {
        let png = svg_to_png(&svg(120, 60), scale).unwrap();
        assert_eq!(size(&png), want, "scale {scale}");
    }
    assert_eq!(
        size(&svg_to_png(&svg(1200, 600), 1.0).unwrap()),
        (1200, 600)
    );
    assert_eq!(
        size(&svg_to_png(&svg(1200, 600), 2.0).unwrap()),
        (2400, 1200)
    );
}

#[test]
fn the_png_really_contains_the_picture() {
    let png = svg_to_png(&svg(20, 20), 1.0).unwrap();
    let pixmap = tiny_skia::Pixmap::decode_png(&png).unwrap();
    let p = pixmap.pixel(10, 10).unwrap();
    assert_eq!(
        (p.red(), p.green(), p.blue(), p.alpha()),
        (0x33, 0x66, 0x99, 255)
    );
}

#[test]
fn rendering_is_deterministic() {
    assert_eq!(
        svg_to_png(&svg(50, 50), 1.0).unwrap(),
        svg_to_png(&svg(50, 50), 1.0).unwrap()
    );
}

#[test]
fn bad_scales_are_rejected_with_the_value_in_the_message() {
    for bad in [0.0f32, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let error = svg_to_png(&svg(10, 10), bad).unwrap_err();
        assert!(matches!(error, ExportError::BadScale(_)), "{bad}: {error}");
        assert!(error.to_string().contains("positive"), "{error}");
    }
}

#[test]
fn oversized_images_are_refused_up_front() {
    let error = svg_to_png(&svg(1200, 600), 7.0).unwrap_err();
    assert_eq!(
        error,
        ExportError::BadSize {
            width: 8400,
            height: 4200
        }
    );
    assert!(error.to_string().contains("8000"), "{error}");
    assert!(svg_to_png(&svg(9000, 10), 1.0).is_err());
    assert!(svg_to_png(&svg(10, 9000), 1.0).is_err());
    // A huge SVG at a tiny scale is a small, legal image.
    assert_eq!(size(&svg_to_png(&svg(8000, 8000), 0.001).unwrap()), (8, 8));
}

#[test]
fn exactly_the_limit_is_allowed() {
    // 8000 x 1 is the largest legal shape; it must work.
    let png = svg_to_png(&svg(MAX_SIDE, 1), 1.0).unwrap();
    assert_eq!(size(&png), (MAX_SIDE, 1));
    assert!(svg_to_png(&svg(MAX_SIDE + 1, 1), 1.0).is_err());
}

#[test]
fn garbage_and_empty_svgs_are_friendly_errors_not_panics() {
    for text in [
        "",
        "this is not svg",
        "<svg",
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"0\" height=\"0\"/>",
        "<html></html>",
    ] {
        let error = svg_to_png(text, 1.0).unwrap_err();
        assert!(matches!(error, ExportError::Parse(_)), "{text:?}: {error}");
        assert!(error.to_string().contains("SVG"), "{error}");
    }
}

#[test]
fn export_errors_convert_into_the_library_error() {
    let converted: SkyError = ExportError::BadScale("0".to_string()).into();
    assert!(matches!(converted, SkyError::Export(_)));
    assert!(
        converted.to_string().contains("export failed"),
        "{converted}"
    );
}

#[test]
fn a_contact_sheet_lays_images_out_in_a_grid() {
    let a = svg_to_png(&svg(40, 20), 1.0).unwrap();
    let sheet = contact_sheet(
        &[a.clone(), a.clone(), a.clone(), a.clone(), a.clone(), a],
        3,
    )
    .unwrap();
    assert_eq!(size(&sheet), (120, 40), "3 columns x 2 rows");
    let pixmap = tiny_skia::Pixmap::decode_png(&sheet).unwrap();
    for (x, y) in [(5, 5), (65, 5), (115, 5), (5, 25), (65, 25), (115, 25)] {
        assert_eq!(
            pixmap.pixel(x, y).unwrap().red(),
            0x33,
            "cell at ({x},{y}) is filled"
        );
    }
}

#[test]
fn a_contact_sheet_with_a_short_last_row_leaves_it_empty() {
    let a = svg_to_png(&svg(10, 10), 1.0).unwrap();
    let sheet = contact_sheet(&[a.clone(), a.clone(), a], 2).unwrap();
    assert_eq!(size(&sheet), (20, 20));
    let pixmap = tiny_skia::Pixmap::decode_png(&sheet).unwrap();
    assert_eq!(
        pixmap.pixel(15, 15).unwrap().alpha(),
        0,
        "the missing fourth cell is transparent"
    );
}

#[test]
fn a_contact_sheet_rejects_nonsense() {
    let a = svg_to_png(&svg(10, 10), 1.0).unwrap();
    assert!(contact_sheet(&[], 3).is_err());
    assert!(contact_sheet(std::slice::from_ref(&a), 0).is_err());
    assert!(contact_sheet(&[b"not a png".to_vec()], 1).is_err());
    let wide = svg_to_png(&svg(5000, 10), 1.0).unwrap();
    assert!(
        contact_sheet(&[wide.clone(), wide], 2).is_err(),
        "10000 px wide is over the limit"
    );
}
