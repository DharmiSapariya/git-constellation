//! Regenerates the review pictures in docs/preview.
//! Run: cargo test --test art_previews -- --ignored

mod common;

use std::path::PathBuf;

use common::{load_fixture, options_for, planned};
use git_constellation::art::labels::drawn_labels;
use git_constellation::art::{ArtOptions, background_svg, sky_svg};
use git_constellation::export::{contact_sheet, svg_to_png, svg_to_png_region};
use git_constellation::layout::labels::LabelKind;
use git_constellation::model::{LabelMode, Point, ThemeName};
use git_constellation::theme::{MonoVariant, Theme};
use git_constellation::{Options, build_sky};

#[test]
#[ignore = "writes files into docs/preview"]
fn regenerate_background_previews() {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("preview");
    std::fs::create_dir_all(&folder).unwrap();
    let model = build_sky(&load_fixture("medium"), &Options::default()).unwrap();
    let themes: [(&str, Theme); 7] = [
        ("midnight", Theme::for_name(ThemeName::Midnight)),
        ("dusk", Theme::for_name(ThemeName::Dusk)),
        ("aurora", Theme::for_name(ThemeName::Aurora)),
        ("sakura", Theme::for_name(ThemeName::Sakura)),
        ("paper", Theme::for_name(ThemeName::Paper)),
        ("mono-dark", Theme::mono(MonoVariant::Dark)),
        ("mono-light", Theme::mono(MonoVariant::Light)),
    ];
    let mut thumbs = Vec::new();
    for (name, theme) in &themes {
        let svg = background_svg(&model, theme, &ArtOptions::default(), &[]);
        std::fs::write(
            folder.join(format!("bg-{name}.png")),
            svg_to_png(&svg, 2.0).unwrap(),
        )
        .unwrap();
        if *name != "mono-light" {
            thumbs.push(svg_to_png(&svg, 0.5).unwrap());
        }
    }
    // A 3 x 2 grid of the six themes (mono shown in its dark variant).
    std::fs::write(
        folder.join("contact-m4.png"),
        contact_sheet(&thumbs, 3).unwrap(),
    )
    .unwrap();
}

fn folder() -> PathBuf {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("preview");
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// A `width` x `height` window centered on `center`, kept inside the 1200 x 600 canvas.
fn window(center: Point, width: f64, height: f64) -> (f64, f64, f64, f64) {
    let x = (center.0 - width / 2.0).clamp(0.0, 1200.0 - width);
    let y = (center.1 - height / 2.0).clamp(0.0, 600.0 - height);
    (x, y, width, height)
}

#[test]
#[ignore = "writes files into docs/preview"]
fn regenerate_sky_previews() {
    let folder = folder();
    let art = ArtOptions::default();

    // The whole sky, three sizes, on midnight, at scale 2.
    for fixture in ["small", "medium", "large"] {
        let options = options_for(ThemeName::Midnight);
        let theme = Theme::for_name(ThemeName::Midnight);
        let (model, layout) = planned(fixture, &options);
        let svg = sky_svg(&model, &layout, &theme, &art, LabelMode::Major);
        std::fs::write(
            folder.join(format!("stars-{fixture}.png")),
            svg_to_png(&svg, 2.0).unwrap(),
        )
        .unwrap();
    }

    // The paper theme.
    let options = options_for(ThemeName::Paper);
    let (model, layout) = planned("medium", &options);
    let svg = sky_svg(
        &model,
        &layout,
        &Theme::for_name(ThemeName::Paper),
        &art,
        LabelMode::Major,
    );
    std::fs::write(
        folder.join("stars-paper-medium.png"),
        svg_to_png(&svg, 2.0).unwrap(),
    )
    .unwrap();

    // The six themes side by side, medium sky, full size each.
    let mut thumbs = Vec::new();
    for name in [
        ThemeName::Midnight,
        ThemeName::Dusk,
        ThemeName::Aurora,
        ThemeName::Sakura,
        ThemeName::Paper,
        ThemeName::Mono,
    ] {
        let options = options_for(name);
        let (model, layout) = planned("medium", &options);
        let svg = sky_svg(
            &model,
            &layout,
            &Theme::for_name(name),
            &art,
            LabelMode::Major,
        );
        thumbs.push(svg_to_png(&svg, 1.0).unwrap());
    }
    std::fs::write(
        folder.join("contact-m5.png"),
        contact_sheet(&thumbs, 3).unwrap(),
    )
    .unwrap();

    // Three close-ups of the medium midnight sky at scale 4.
    let options = options_for(ThemeName::Midnight);
    let theme = Theme::for_name(ThemeName::Midnight);
    let (model, layout) = planned("medium", &options);
    let svg = sky_svg(&model, &layout, &theme, &art, LabelMode::Major);
    let crop = |name: &str, (x, y, w, h): (f64, f64, f64, f64)| {
        std::fs::write(
            folder.join(name),
            svg_to_png_region(&svg, x, y, w, h, 4.0).unwrap(),
        )
        .unwrap();
    };

    // The biggest star and its neighbours.
    let biggest = (0..model.stars.len())
        .max_by_key(|i| (model.stars[*i].tier, std::cmp::Reverse(*i)))
        .expect("the medium sky has stars");
    crop(
        "zoom-star.png",
        window(layout.positions[biggest], 300.0, 200.0),
    );

    // A stretch of sky with labels over the Milky Way; the "first light" note if there is one.
    let shown = drawn_labels(&layout.labels, LabelMode::Major);
    let note = shown
        .iter()
        .find(|l| matches!(l.kind, LabelKind::FirstLight(_)))
        .or_else(|| shown.first())
        .expect("the medium sky has labels");
    let middle = (
        (note.rect.x0 + note.rect.x1) / 2.0,
        (note.rect.y0 + note.rect.y1) / 2.0,
    );
    crop("zoom-labels.png", window(middle, 300.0, 150.0));

    // Where lines meet stars: the middle of the first line of the biggest constellation
    // (a founding star can be a wanderer with no lines at all).
    let meeting = layout
        .constellations
        .iter()
        .max_by_key(|c| c.edges.len())
        .and_then(|c| c.edges.first())
        .map(|&(a, b)| {
            let (pa, pb) = (layout.positions[a], layout.positions[b]);
            ((pa.0 + pb.0) / 2.0, (pa.1 + pb.1) / 2.0)
        })
        .expect("the medium sky has lines");
    crop("zoom-lines.png", window(meeting, 300.0, 200.0));
}
