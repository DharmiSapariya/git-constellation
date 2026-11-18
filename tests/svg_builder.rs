//! SvgBuilder: formatting, escaping, ids, safety rules and smooth paths.

use std::collections::BTreeSet;

use git_constellation::svg::{
    self, Stop, SvgBuilder, Units, escape, fmt, is_hex_color, smooth_path,
};

fn sample() -> String {
    let mut b = SvgBuilder::new(1200.0, 600.0, "A <sky> & more", "It's \"nice\"");
    let sky = b.new_id("sky");
    b.linear_gradient(
        &sky,
        (0.0, 0.0),
        (0.0, 1.0),
        Units::ObjectBoundingBox,
        &[
            Stop::new(0.0, "#0b1026", 1.0),
            Stop::new(0.55, "#151b3d", 1.0),
            Stop::new(1.0, "#2a2250", 1.0),
        ],
    );
    let glow = b.new_id("glow");
    b.radial_gradient(
        &glow,
        (600.0, 640.0),
        520.0,
        Units::UserSpaceOnUse,
        &[
            Stop::new(0.0, "#4b315f", 0.55),
            Stop::new(1.0, "#4b315f", 0.0),
        ],
    );
    b.group(&[("id", "bg".to_string())], |g| {
        g.add(&svg::rect(
            0.0,
            0.0,
            1200.0,
            600.0,
            &[("fill", format!("url(#{sky})"))],
        ));
        g.add(&svg::rect(
            0.0,
            0.0,
            1200.0,
            600.0,
            &[("fill", format!("url(#{glow})"))],
        ));
        g.add(&svg::circle(
            10.0,
            20.5,
            1.25,
            &[("fill", "#e6e9ff".to_string())],
        ));
        g.add(&svg::ellipse(
            300.0,
            200.0,
            190.0,
            100.0,
            &[("transform", "rotate(-12 300 200)".to_string())],
        ));
        g.add(&svg::path(
            &smooth_path(&[(0.0, 0.0), (50.0, 20.0), (100.0, 0.0)], false),
            &[("fill", "none".to_string())],
        ));
    });
    b.finish()
}

#[test]
fn numbers_are_short_clean_and_never_negative_zero() {
    assert_eq!(fmt(0.1 + 0.2), "0.3");
    assert_eq!(fmt(1.0), "1");
    assert_eq!(fmt(1.005), "1");
    assert_eq!(fmt(1.239), "1.24");
    assert_eq!(fmt(-0.001), "0");
    assert_eq!(fmt(-0.0), "0");
    assert_eq!(fmt(-1.5), "-1.5");
    assert_eq!(fmt(100.0), "100");
    assert_eq!(fmt(1234567.891), "1234567.89");
    assert_eq!(fmt(1e-9), "0");
    assert!(
        !fmt(1e15).contains('e') && !fmt(1e-15).contains('e'),
        "never scientific notation"
    );
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(fmt(bad), "0");
    }
    for i in -3000..3000 {
        let text = fmt(f64::from(i) * 0.0037);
        assert!(
            text != "-0" && !text.contains("e") && !text.ends_with('.'),
            "{text}"
        );
    }
}

#[test]
fn all_five_special_characters_are_escaped() {
    assert_eq!(escape("&"), "&amp;");
    assert_eq!(escape("<"), "&lt;");
    assert_eq!(escape(">"), "&gt;");
    assert_eq!(escape("\""), "&quot;");
    assert_eq!(escape("'"), "&#39;");
    assert_eq!(
        escape("a & b < c > d \" e ' f"),
        "a &amp; b &lt; c &gt; d &quot; e &#39; f"
    );
    assert_eq!(
        escape("&lt;"),
        "&amp;lt;",
        "already-escaped text is escaped again, never trusted"
    );
    assert_eq!(escape("plain 日本語"), "plain 日本語");
}

#[test]
fn titles_and_attribute_values_are_escaped_in_the_document() {
    let svg = sample();
    assert!(svg.contains("<title>A &lt;sky&gt; &amp; more</title>"));
    assert!(svg.contains("<desc>It&#39;s &quot;nice&quot;</desc>"));
    let tricky = svg::rect(
        0.0,
        0.0,
        1.0,
        1.0,
        &[("id", "x\" onload=\"evil".to_string())],
    );
    assert!(
        tricky.contains("id=\"x&quot; onload=&quot;evil\""),
        "{tricky}"
    );
    assert!(!tricky.contains("onload=\"evil"));
    assert!(svg::text(1.0, 2.0, "<b>&", &[]).contains("&lt;b&gt;&amp;"));
    assert!(svg::style("a > b { }").contains("a &gt; b"));
}

#[test]
fn the_root_element_and_accessibility_parts_are_right() {
    let svg = sample();
    assert!(svg.starts_with(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1200 600\" width=\"1200\" height=\"600\" role=\"img\">"
    ));
    let title = svg.find("<title>").expect("title");
    let desc = svg.find("<desc>").expect("desc");
    let defs = svg.find("<defs>").expect("defs");
    assert!(title < desc && desc < defs, "title, desc, then defs");
    assert_eq!(svg.matches("<defs>").count(), 1, "one defs, at the top");
    assert!(svg.ends_with("</svg>\n"));
}

#[test]
fn ids_are_unique_and_deterministic() {
    let mut a = SvgBuilder::new(10.0, 10.0, "t", "d");
    let ids: Vec<String> = ["sky", "sky", "neb", "sky", "mw"]
        .iter()
        .map(|p| a.new_id(p))
        .collect();
    assert_eq!(ids, ["sky1", "sky2", "neb3", "sky4", "mw5"]);
    let unique: BTreeSet<&String> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len());
    let mut b = SvgBuilder::new(99.0, 99.0, "other", "other");
    assert_eq!(b.new_id("sky"), "sky1", "a new document starts again at 1");
    assert_eq!(
        sample(),
        sample(),
        "identical calls give identical documents"
    );
}

fn ids_in(svg: &str) -> Vec<String> {
    svg.split(" id=\"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap_or("").to_string())
        .collect()
}

#[test]
fn every_url_reference_points_at_a_defined_id() {
    let svg = sample();
    let defined: BTreeSet<String> = ids_in(&svg).into_iter().collect();
    let refs: Vec<&str> = svg
        .split("url(#")
        .skip(1)
        .map(|r| r.split(')').next().unwrap_or(""))
        .collect();
    assert_eq!(refs.len(), 2);
    for r in refs {
        assert!(
            defined.contains(r),
            "url(#{r}) points at nothing; defined: {defined:?}"
        );
    }
    let all = ids_in(&svg);
    assert_eq!(all.len(), defined.len(), "no id is defined twice");
}

#[test]
fn the_output_parses_and_renders_with_usvg() {
    let svg = sample();
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("valid SVG");
    assert_eq!((tree.size().width(), tree.size().height()), (1200.0, 600.0));
}

#[test]
fn nothing_unsafe_or_external_is_ever_written() {
    let svg = sample();
    for forbidden in [
        "<script",
        "<image",
        "<foreignObject",
        "xlink:href",
        "href=",
        "rgba(",
        "hsl(",
        "hsla(",
        "rgb(",
        "mix-blend-mode",
        "<filter",
        "javascript:",
    ] {
        assert!(!svg.contains(forbidden), "found {forbidden}");
    }
    let without_namespace = svg.replace("http://www.w3.org/2000/svg", "");
    assert!(!without_namespace.contains("http://") && !without_namespace.contains("https://"));
    assert!(!without_namespace.contains("url(http"));
}

#[test]
fn hex_colors_are_recognized_strictly() {
    for good in ["#000000", "#ffffff", "#0b1026", "#ABCDEF"] {
        assert!(is_hex_color(good), "{good}");
    }
    for bad in [
        "",
        "#fff",
        "000000",
        "#gggggg",
        "red",
        "#12345678",
        "rgb(0,0,0)",
        " #000000",
    ] {
        assert!(!is_hex_color(bad), "{bad}");
    }
}

#[test]
fn gradients_write_stops_with_color_and_separate_opacity() {
    let mut b = SvgBuilder::new(100.0, 50.0, "t", "d");
    b.radial_gradient(
        "g1",
        (0.5, 0.5),
        0.5,
        Units::ObjectBoundingBox,
        &[
            Stop::new(0.0, "#6d4aa8", 1.0),
            Stop::new(1.0, "#6d4aa8", 0.0),
        ],
    );
    let svg = b.finish();
    assert!(svg.contains("gradientUnits=\"objectBoundingBox\""));
    assert!(svg.contains("<stop offset=\"0\" stop-color=\"#6d4aa8\" stop-opacity=\"1\"/>"));
    assert!(svg.contains("<stop offset=\"1\" stop-color=\"#6d4aa8\" stop-opacity=\"0\"/>"));
}

#[test]
fn groups_nest_and_close() {
    let mut b = SvgBuilder::new(10.0, 10.0, "t", "d");
    b.group(
        &[("id", "outer".to_string()), ("opacity", fmt(0.5))],
        |outer| {
            outer.group(&[("id", "inner".to_string())], |inner| {
                inner.add(&svg::circle(1.0, 1.0, 1.0, &[]))
            });
        },
    );
    let svg = b.finish();
    assert!(svg.contains("<g id=\"outer\" opacity=\"0.5\">"));
    assert_eq!(svg.matches("<g ").count(), svg.matches("</g>").count());
    usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("valid");
}

#[test]
fn clip_paths_are_defined_in_defs() {
    let mut b = SvgBuilder::new(10.0, 10.0, "t", "d");
    b.clip_path("c1", &svg::rect(0.0, 0.0, 5.0, 5.0, &[]));
    let svg = b.finish();
    let defs_end = svg.find("</defs>").expect("defs end");
    assert!(svg.find("<clipPath id=\"c1\">").expect("clip") < defs_end);
}

#[test]
fn smooth_paths_pass_through_their_points() {
    let pts = [(0.0, 0.0), (100.0, 50.0), (200.0, 0.0), (300.0, 50.0)];
    let open = smooth_path(&pts, false);
    assert!(open.starts_with("M0 0C"));
    assert_eq!(open.matches('C').count(), 3);
    assert!(!open.ends_with('Z'));
    for (x, y) in &pts[1..] {
        assert!(open.contains(&format!("{} {}", fmt(*x), fmt(*y))), "{open}");
    }
    let closed = smooth_path(&pts, true);
    assert_eq!(closed.matches('C').count(), 4);
    assert!(closed.ends_with('Z'));
    assert_eq!(smooth_path(&[], false), "");
    assert_eq!(smooth_path(&[(5.0, 6.0)], false), "M5 6");
    assert_eq!(smooth_path(&pts[..2], false).matches('C').count(), 1);
    usvg::Tree::from_str(&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"400\" height=\"100\"><path d=\"{closed}\"/></svg>"), &usvg::Options::default()).expect("valid path data");
}

#[test]
fn a_straight_line_of_points_gives_a_straight_curve() {
    // Catmull-Rom through collinear points must not wobble off the line.
    let pts = [(0.0, 0.0), (10.0, 10.0), (20.0, 20.0), (30.0, 30.0)];
    let path = smooth_path(&pts, false);
    for number_pair in path.split('C').skip(1) {
        let nums: Vec<f64> = number_pair
            .split(' ')
            .filter_map(|n| n.parse().ok())
            .collect();
        for xy in nums.chunks(2) {
            assert!((xy[0] - xy[1]).abs() < 0.02, "{xy:?} left the line");
        }
    }
}
