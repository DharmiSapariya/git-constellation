//! Review pictures (written to `target/previews`, or `PREVIEW_DIR`) and the two committed hero
//! images plus the linter report (written to `docs/preview`).
//! Run: cargo test --test art_previews -- --ignored

mod common;

use common::{
    SYNTHETIC, lint_report_text, load_fixture, medium_with_languages, options_for, planned,
    review_dir,
};
use git_constellation::art::labels::drawn_labels;
use git_constellation::art::milkyway::centerline;
use git_constellation::art::{ArtOptions, background_svg, sky_svg};
use git_constellation::export::{contact_sheet, svg_to_png, svg_to_png_region};
use git_constellation::layout::labels::LabelKind;
use git_constellation::model::{LabelMode, Point, ThemeName};
use git_constellation::theme::{MonoVariant, Theme};
use git_constellation::{Options, build_sky, plan_sky};

#[test]
#[ignore = "writes review images"]
fn regenerate_background_previews() {
    let folder = review_dir();
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

/// A `width` x `height` window centered on `center`, kept inside the 1200 x 600 canvas.
fn window(center: Point, width: f64, height: f64) -> (f64, f64, f64, f64) {
    let x = (center.0 - width / 2.0).clamp(0.0, 1200.0 - width);
    let y = (center.1 - height / 2.0).clamp(0.0, 600.0 - height);
    (x, y, width, height)
}

#[test]
#[ignore = "writes review images"]
fn regenerate_sky_previews() {
    let folder = review_dir();
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

#[test]
#[ignore = "writes review images"]
fn regenerate_ring_and_flare_review_images() {
    let folder = review_dir();
    let art = ArtOptions::default();
    let render = |fixture: &str, name: ThemeName| {
        let options = options_for(name);
        let (model, layout) = planned(fixture, &options);
        let svg = sky_svg(
            &model,
            &layout,
            &Theme::for_name(name),
            &art,
            LabelMode::Major,
        );
        (model, layout, svg)
    };

    // The large sky at scale 1 and the paper sky at scale 2.
    let (large, large_layout, large_svg) = render("large", ThemeName::Midnight);
    std::fs::write(
        folder.join("large-after.png"),
        svg_to_png(&large_svg, 1.0).unwrap(),
    )
    .unwrap();
    let (_, _, paper_svg) = render("medium", ThemeName::Paper);
    std::fs::write(
        folder.join("stars-paper-medium-after.png"),
        svg_to_png(&paper_svg, 2.0).unwrap(),
    )
    .unwrap();

    // A founding star and a fresh star side by side, scale 4.
    let founding = large
        .stars
        .iter()
        .position(|s| s.founding)
        .expect("a founding star");
    let fresh = large
        .stars
        .iter()
        .position(|s| s.fresh && !s.founding && !s.dormant)
        .expect("a fresh star");
    let crop = |center: Point| {
        let (x, y, w, h) = window(center, 200.0, 150.0);
        svg_to_png_region(&large_svg, x, y, w, h, 4.0).unwrap()
    };
    let pair = [
        crop(large_layout.positions[founding]),
        crop(large_layout.positions[fresh]),
    ];
    std::fs::write(
        folder.join("zoom-rings.png"),
        contact_sheet(&pair, 2).unwrap(),
    )
    .unwrap();

    // A calm tier 5 star with its flare, from the medium sky.
    let (medium, medium_layout, medium_svg) = render("medium", ThemeName::Midnight);
    let flared = medium
        .stars
        .iter()
        .position(|s| s.tier.number() == 5 && !s.dormant && !s.borrowed)
        .expect("a calm tier 5 star");
    let (x, y, w, h) = window(medium_layout.positions[flared], 220.0, 150.0);
    std::fs::write(
        folder.join("zoom-flare.png"),
        svg_to_png_region(&medium_svg, x, y, w, h, 4.0).unwrap(),
    )
    .unwrap();
}

/// The committed hero image: the biggest scale (1.5, then 1.25, then 1.0) that stays under
/// 400 KB. Returns the PNG and the scale used.
fn hero_png(svg: &str) -> (Vec<u8>, f32) {
    const LIMIT: usize = 400 * 1024;
    for scale in [1.5_f32, 1.25, 1.0] {
        let png = svg_to_png(svg, scale).unwrap();
        if png.len() < LIMIT {
            return (png, scale);
        }
    }
    panic!("a hero image is over 400 KB even at scale 1");
}

/// Writes the two hero images and the linter report into docs/preview.
#[test]
#[ignore = "writes files into docs/preview"]
fn regenerate_hero_images_and_lint_report() {
    let folder = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("preview");
    std::fs::create_dir_all(&folder).unwrap();
    for (name, theme_name) in [
        ("sky-medium.png", ThemeName::Midnight),
        ("sky-paper.png", ThemeName::Paper),
    ] {
        let options = options_for(theme_name);
        let (model, layout) = planned("medium", &options);
        let svg = sky_svg(
            &model,
            &layout,
            &Theme::for_name(theme_name),
            &ArtOptions::default(),
            LabelMode::Major,
        );
        let (png, scale) = hero_png(&svg);
        println!("{name}: {} bytes at scale {scale}", png.len());
        std::fs::write(folder.join(name), png).unwrap();
    }
    std::fs::write(folder.join("lint-report.txt"), lint_report_text()).unwrap();
}

/// The report must be complete: every fixture, the drawn-sky checks, nothing "not checkable yet".
#[test]
fn the_lint_report_covers_everything() {
    let report = lint_report_text();
    for fixture in common::FIXTURES {
        assert!(report.contains(&format!("=== {fixture}:")), "{fixture}");
    }
    for wanted in [
        "HARD (layout)",
        "SOFT (drawn sky)",
        "text contrast",
        "labels clear of rings",
        "flares clear of labels",
        "stars stand out",
        "sparse sky",
    ] {
        assert!(report.contains(wanted), "missing: {wanted}");
    }
    assert!(!report.contains("Not checkable yet"));
    assert!(
        !report.contains("[warn]") || report.contains("SOFT"),
        "hard checks never warn"
    );
}

/// The pictures for the Part 1 review: composition of small skies, paper stars, label crops.
#[test]
#[ignore = "writes review images"]
fn regenerate_part1_review_images() {
    let folder = review_dir();
    let art = ArtOptions::default();
    let write = |name: &str, png: Vec<u8>| std::fs::write(folder.join(name), png).unwrap();
    let sky = |model: &git_constellation::SkyModel,
               layout: &git_constellation::Layout,
               name: ThemeName| {
        sky_svg(
            model,
            layout,
            &Theme::for_name(name),
            &art,
            LabelMode::Major,
        )
    };

    // Whole skies at scale 1 (midnight): the fixtures and the synthetic sparse skies.
    for fixture in ["small", "tiny", "edge", "medium"] {
        let (model, layout) = planned(fixture, &Options::default());
        let svg = sky(&model, &layout, ThemeName::Midnight);
        write(
            &format!("compose-{fixture}-after.png"),
            svg_to_png(&svg, 1.0).unwrap(),
        );
    }
    for (name, languages) in SYNTHETIC {
        let (model, layout) =
            plan_sky(&medium_with_languages(languages), &Options::default()).unwrap();
        let svg = sky(&model, &layout, ThemeName::Midnight);
        write(
            &format!("compose-{name}-after.png"),
            svg_to_png(&svg, 1.0).unwrap(),
        );
    }

    // Paper: small and medium, scale 2.
    for fixture in ["small", "medium"] {
        let options = options_for(ThemeName::Paper);
        let (model, layout) = planned(fixture, &options);
        let svg = sky(&model, &layout, ThemeName::Paper);
        write(
            &format!("stars-paper-{fixture}-after.png"),
            svg_to_png(&svg, 2.0).unwrap(),
        );
    }

    // The six themes side by side (medium sky).
    let mut thumbs = Vec::new();
    for name in [
        ThemeName::Midnight,
        ThemeName::Dusk,
        ThemeName::Aurora,
        ThemeName::Sakura,
        ThemeName::Paper,
        ThemeName::Mono,
    ] {
        let (model, layout) = planned("medium", &options_for(name));
        thumbs.push(svg_to_png(&sky(&model, &layout, name), 1.0).unwrap());
    }
    write("contact-part1.png", contact_sheet(&thumbs, 3).unwrap());

    // Close-ups of the medium midnight sky at scale 4.
    let (model, layout) = planned("medium", &Options::default());
    let svg = sky(&model, &layout, ThemeName::Midnight);
    let crop = |name: &str, center: Point, w: f64, h: f64| {
        let (x, y, w, h) = window(center, w, h);
        write(name, svg_to_png_region(&svg, x, y, w, h, 4.0).unwrap());
    };
    let shown = drawn_labels(&layout.labels, LabelMode::Major);
    let middle = |l: &git_constellation::layout::labels::Label| {
        ((l.rect.x0 + l.rect.x1) / 2.0, (l.rect.y0 + l.rect.y1) / 2.0)
    };
    if let Some(note) = shown
        .iter()
        .find(|l| matches!(l.kind, LabelKind::FirstLight(_)))
    {
        crop("zoom-first-light.png", middle(note), 200.0, 120.0);
    }
    // Two repo names over the Milky Way (from the large sky, which labels many stars): the two
    // star labels nearest its centerline that sit close enough to share one crop.
    let (big_model, big_layout) = planned("large", &Options::default());
    let big_svg = sky(&big_model, &big_layout, ThemeName::Midnight);
    let big_shown = drawn_labels(&big_layout.labels, LabelMode::Major);
    let band = centerline(1200.0, 600.0, 200);
    let from_band = |p: Point| {
        band.iter()
            .map(|b| ((b.0 - p.0).powi(2) + (b.1 - p.1).powi(2)).sqrt())
            .fold(f64::INFINITY, f64::min)
    };
    let mut names: Vec<Point> = big_shown
        .iter()
        .filter(|l| matches!(l.kind, LabelKind::Star(_)))
        .map(middle)
        .collect();
    names.sort_by(|a, b| from_band(*a).total_cmp(&from_band(*b)));
    if let Some(&first) = names.first() {
        let partner = names[1..]
            .iter()
            .copied()
            .find(|p| ((p.0 - first.0).powi(2) + (p.1 - first.1).powi(2)).sqrt() < 200.0)
            .unwrap_or(first);
        let center = ((first.0 + partner.0) / 2.0, (first.1 + partner.1) / 2.0);
        let (x, y, w, h) = window(center, 320.0, 170.0);
        write(
            "zoom-labels-band.png",
            svg_to_png_region(&big_svg, x, y, w, h, 4.0).unwrap(),
        );
    }
}
