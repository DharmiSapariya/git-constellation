//! Regenerates the Milestone 4 review pictures in docs/preview.
//! Run: cargo test --test art_previews -- --ignored

mod common;

use std::path::PathBuf;

use common::load_fixture;
use git_constellation::art::{ArtOptions, background_svg};
use git_constellation::export::{contact_sheet, svg_to_png};
use git_constellation::model::ThemeName;
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
