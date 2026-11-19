//! Milestone 5: labels, the HUD, the sky-color helper and the theme accent.

mod common;

use common::{THEME_NAMES, options_for, planned};
use git_constellation::Options;
use git_constellation::art::background::{DUST_LABEL_PAD, dust_dots};
use git_constellation::art::hud::{
    HUD_TITLE_MAX_X, HUD_USERNAME_MAX_CHARS, TITLE_POS, TITLE_SIZE, display_username,
    subtitle_text, title_text,
};
use git_constellation::art::labels::{drawn_labels, label_markup};
use git_constellation::art::milkyway::{BAND_STAR_CLEARANCE, band_dots};
use git_constellation::art::{ArtOptions, sky_svg};
use git_constellation::layout::labels::{Label, LabelKind};
use git_constellation::layout::text::text_width;
use git_constellation::layout::{HUD_RECT, Rect, dist};
use git_constellation::model::{LabelMode, ThemeName};
use git_constellation::svg::is_hex_color;
use git_constellation::theme::{MonoVariant, SKY_MID_STOP, Theme, parse_hex, sky_color_at};

fn label(kind: LabelKind, text: &str, font_size: f64) -> Label {
    Label {
        kind,
        text: text.to_string(),
        rect: Rect {
            x0: 100.0,
            y0: 200.0,
            x1: 180.0,
            y1: 214.0,
        },
        font_size,
    }
}

/// The number inside `name="..."` of the first element that has it after `from`.
fn number_after(svg: &str, from: &str, name: &str) -> f64 {
    let start = svg.find(from).expect("marker");
    let rest = &svg[start..];
    let needle = format!(" {name}=\"");
    let at = rest.find(&needle).expect("attribute") + needle.len();
    let end = rest[at..].find('"').expect("closing quote");
    rest[at..at + end].parse().expect("a number")
}

#[test]
fn the_sky_color_follows_the_gradient_and_paper_is_flat() {
    let midnight = Theme::for_name(ThemeName::Midnight);
    let at = |y: f64| sky_color_at(&midnight, y, 600.0);
    assert_eq!(at(0.0), parse_hex(midnight.sky_top).unwrap());
    assert_eq!(
        at(SKY_MID_STOP * 600.0),
        parse_hex(midnight.sky_mid).unwrap()
    );
    assert_eq!(at(600.0), parse_hex(midnight.sky_bottom).unwrap());
    assert_eq!(at(-50.0), at(0.0), "above the sky is the top");
    assert_eq!(at(900.0), at(600.0), "below the sky is the bottom");
    let paper = Theme::for_name(ThemeName::Paper);
    for y in [0.0, 300.0, 600.0] {
        assert_eq!(
            sky_color_at(&paper, y, 600.0),
            parse_hex(paper.background()).unwrap()
        );
    }
}

#[test]
fn every_theme_has_an_accent_and_paper_uses_its_ink() {
    let mut themes: Vec<Theme> = THEME_NAMES.iter().map(|n| Theme::for_name(*n)).collect();
    themes.push(Theme::mono(MonoVariant::Light));
    themes.push(Theme::mono(MonoVariant::Dark));
    for theme in &themes {
        assert!(is_hex_color(theme.accent), "{:?}", theme.name);
    }
    for name in [
        ThemeName::Midnight,
        ThemeName::Dusk,
        ThemeName::Aurora,
        ThemeName::Sakura,
    ] {
        assert_eq!(Theme::for_name(name).accent, "#ffe08a");
    }
    let paper = Theme::for_name(ThemeName::Paper);
    assert_eq!(paper.accent, paper.ink);
}

#[test]
fn a_label_is_centered_in_its_box_with_the_baseline_below_the_middle() {
    let theme = Theme::for_name(ThemeName::Midnight);
    let markup = label_markup(&theme, &label(LabelKind::Star(0), "my-repo", 10.5), 600.0);
    assert_eq!(
        markup.matches("<text").count(),
        2,
        "a halo copy and the real text"
    );
    assert_eq!(markup.matches("aria-hidden=\"true\"").count(), 1);
    assert!(markup.contains("text-anchor=\"middle\""));
    assert!((number_after(&markup, "<text", "x") - 140.0).abs() < 0.01);
    let baseline = number_after(&markup, "<text", "y");
    assert!(
        (baseline - (207.0 + 0.35 * 10.5)).abs() < 0.01,
        "{baseline}"
    );
}

#[test]
fn constellation_names_are_spaced_and_the_small_notes_are_not() {
    let theme = Theme::for_name(ThemeName::Midnight);
    let name = label_markup(
        &theme,
        &label(LabelKind::Constellation(0), "RUST", 11.0),
        600.0,
    );
    assert!(name.contains("letter-spacing=\"1.76\""), "{name}");
    assert!(name.contains("font-style=\"italic\""));
    for (kind, text) in [
        (LabelKind::FirstLight(0), "first light"),
        (LabelKind::Caption, "a young sky"),
    ] {
        let note = label_markup(&theme, &label(kind, text, 9.0), 600.0);
        assert!(!note.contains("letter-spacing"), "{text}");
        assert!(
            note.contains(&format!(">{text}</text>")),
            "{text} is not changed to capitals"
        );
        assert!(note.contains(&format!("fill=\"{}\"", theme.text_dim)));
    }
}

#[test]
fn label_text_is_escaped_and_never_cut_again() {
    let theme = Theme::for_name(ThemeName::Midnight);
    let odd = "<a&\"b'>_a-very-long-name-indeed";
    let markup = label_markup(&theme, &label(LabelKind::Star(0), odd, 10.5), 600.0);
    assert!(markup.contains("&lt;a&amp;&quot;b&#39;&gt;_a-very-long-name-indeed"));
    let wrapped =
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 300 300\">{markup}</svg>");
    usvg::Tree::from_str(&wrapped, &usvg::Options::default()).expect("valid XML");
}

#[test]
fn none_draws_no_label_at_all_but_the_hud_stays() {
    let options = options_for(ThemeName::Midnight);
    let theme = Theme::for_name(ThemeName::Midnight);
    let (model, layout) = planned("medium", &options);
    assert!(!layout.labels.is_empty(), "the layout does list labels");
    assert!(drawn_labels(&layout.labels, LabelMode::Off).is_empty());
    let off = sky_svg(
        &model,
        &layout,
        &theme,
        &ArtOptions::default(),
        LabelMode::Off,
    );
    let major = sky_svg(
        &model,
        &layout,
        &theme,
        &ArtOptions::default(),
        LabelMode::Major,
    );
    let all = sky_svg(
        &model,
        &layout,
        &theme,
        &ArtOptions::default(),
        LabelMode::All,
    );
    // The title is two text elements (halo and real) and the stats line is one.
    assert_eq!(off.matches("<text").count(), 3, "only the HUD text");
    assert!(off.contains("<g id=\"labels\">") && off.contains("<g id=\"hud\">"));
    assert_eq!(
        major.matches("<text").count(),
        2 * layout.labels.len() + 3,
        "every placed label, plus the HUD"
    );
    assert!(all.matches("<text").count() >= major.matches("<text").count());
}

#[test]
fn dust_and_band_dots_keep_out_of_label_boxes_and_away_from_stars() {
    let boxes = [Rect {
        x0: 400.0,
        y0: 150.0,
        x1: 800.0,
        y1: 450.0,
    }];
    let free = dust_dots(7, 1200.0, 600.0, 260, &[], &[]);
    let kept = dust_dots(7, 1200.0, 600.0, 260, &[], &boxes);
    assert!(
        free.iter().any(|d| boxes[0].contains((d.x, d.y))),
        "the box is in the way"
    );
    let padded = boxes[0].inflate(DUST_LABEL_PAD);
    assert!(kept.iter().all(|d| !padded.contains((d.x, d.y))));

    let weeks = vec![1.0; 53];
    let loose = band_dots(3, 1200.0, 600.0, &weeks, &[], &[]);
    assert!(
        loose.iter().any(|d| boxes[0].contains((d.x, d.y))),
        "the band crosses the box"
    );
    let boxed = band_dots(3, 1200.0, 600.0, &weeks, &[], &boxes);
    assert!(boxed.iter().all(|d| !padded.contains((d.x, d.y))));
    let star = (loose[0].x, loose[0].y, 3.0);
    let guarded = band_dots(3, 1200.0, 600.0, &weeks, &[star], &[]);
    assert!(
        guarded
            .iter()
            .all(|d| dist((d.x, d.y), (star.0, star.1)) >= BAND_STAR_CLEARANCE),
        "no band dot within {BAND_STAR_CLEARANCE} px of a star"
    );
    assert!(guarded.len() < loose.len());
}

#[test]
fn long_usernames_are_cut_by_characters_and_stay_inside_the_hud() {
    assert_eq!(display_username("octocat"), "octocat");
    let long = "a".repeat(39);
    let wide = "W".repeat(39);
    let accents = "é".repeat(39);
    let emoji = "🌟".repeat(39);
    for name in [long, wide, accents, emoji] {
        let shown = display_username(&name);
        assert!(shown.chars().count() <= HUD_USERNAME_MAX_CHARS, "{shown}");
        assert!(shown.ends_with('\u{2026}'), "{shown} is marked as cut");
        let reach = TITLE_POS.0 + text_width(&title_text(&shown), TITLE_SIZE, false);
        assert!(reach <= HUD_TITLE_MAX_X, "{shown} reaches {reach}");
    }
    assert!(
        std::hint::black_box(HUD_TITLE_MAX_X) <= HUD_RECT.x1,
        "the title stays inside the reserved HUD box"
    );
}

#[test]
fn the_stats_line_uses_singular_and_plural_forms() {
    let (mut model, _) = planned("medium", &options_for(ThemeName::Midnight));
    assert!(!model.constellations.is_empty());
    model.stars.truncate(1);
    model.constellations.truncate(1);
    model.current_streak = 1;
    assert_eq!(
        subtitle_text(&model),
        "1 star  \u{b7}  1 constellation  \u{b7}  1-day streak"
    );
    model.current_streak = 0;
    assert!(subtitle_text(&model).ends_with("no streak yet"));
    model.current_streak = 12;
    assert!(subtitle_text(&model).ends_with("12-day streak"));
    model.constellations.clear();
    assert!(subtitle_text(&model).contains("0 constellations"));
}

#[test]
fn the_hud_shows_the_username_and_uses_the_accent() {
    for name in THEME_NAMES {
        let options = Options {
            theme: name,
            ..Options::default()
        };
        let theme = Theme::for_name(name);
        let (model, layout) = planned("small", &options);
        let svg = sky_svg(
            &model,
            &layout,
            &theme,
            &ArtOptions::default(),
            LabelMode::Major,
        );
        let hud = &svg[svg.find("<g id=\"hud\">").expect("hud")..];
        assert!(
            hud.contains(&format!("fill=\"{}\"", theme.accent)),
            "{name:?}"
        );
        assert!(hud.contains(&display_username(&model.username)), "{name:?}");
    }
}
