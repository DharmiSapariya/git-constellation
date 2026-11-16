//! The linter must catch each kind of mistake, and must pass a good sky.

mod common;

use common::load_fixture;
use git_constellation::lint::{Check, LintReport, geometry_failures, lint};
use git_constellation::model::{LabelMode, LanguageKey, SkyModel};
use git_constellation::{Layout, Options, plan_sky};

fn good() -> (SkyModel, Layout) {
    let options = Options {
        labels: LabelMode::Major,
        ..Options::default()
    };
    plan_sky(&load_fixture("medium"), &options).expect("medium lays out")
}

fn failed<'a>(report: &'a LintReport, name_part: &str) -> Option<&'a Check> {
    report
        .hard
        .iter()
        .find(|c| !c.passed && c.name.contains(name_part))
}

/// Break the layout with `change`, then expect exactly this check to complain.
fn expect_caught(name_part: &str, change: impl FnOnce(&SkyModel, &mut Layout)) {
    let (model, mut layout) = good();
    assert!(
        lint(&model, &layout).hard_passed(),
        "the starting layout must be clean"
    );
    change(&model, &mut layout);
    let report = lint(&model, &layout);
    assert!(
        !report.hard_passed(),
        "{name_part}: the mistake was not noticed"
    );
    assert!(
        failed(&report, name_part).is_some(),
        "{name_part}: wrong check fired:\n{}",
        report.to_text()
    );
}

#[test]
fn a_good_layout_passes_every_hard_check() {
    let (model, layout) = good();
    let report = lint(&model, &layout);
    assert!(report.hard_passed(), "{}", report.to_text());
    assert_eq!(report.hard.len(), 10);
    assert!(geometry_failures(&model, &layout).is_empty());
}

#[test]
fn two_stars_on_top_of_each_other_are_caught() {
    expect_caught("star spacing", |_, l| l.positions[1] = l.positions[0]);
}

#[test]
fn two_stars_just_inside_the_minimum_distance_are_caught() {
    expect_caught("star spacing", |_, l| {
        let p = l.positions[0];
        l.positions[1] = (p.0 + 20.0, p.1);
    });
}

#[test]
fn a_star_in_the_margin_is_caught() {
    expect_caught("margin", |_, l| l.positions[0] = (10.0, 300.0));
    expect_caught("margin", |_, l| l.positions[2] = (600.0, 575.0));
}

#[test]
fn a_star_in_the_title_or_moon_area_is_caught() {
    expect_caught("reserved", |_, l| l.positions[0] = (100.0, 50.0));
    expect_caught("reserved", |_, l| l.positions[0] = (1090.0, 112.0));
}

#[test]
fn crossing_lines_are_caught() {
    expect_caught("line crossings", |model, l| {
        // Find a constellation with at least 4 stars, then force one of its lines
        // to be crossed by a brand new line between two other stars.
        let ci = l
            .constellations
            .iter()
            .position(|c| c.stars.len() >= 4)
            .expect("a constellation with 4 stars");
        let (a, b) = l.constellations[ci].edges[0];
        let others: Vec<usize> = l.constellations[ci]
            .stars
            .iter()
            .copied()
            .filter(|s| *s != a && *s != b)
            .collect();
        let (c, d) = (others[0], others[1]);
        let mid = (
            (l.positions[a].0 + l.positions[b].0) / 2.0,
            (l.positions[a].1 + l.positions[b].1) / 2.0,
        );
        l.positions[c] = (mid.0 - 12.0, mid.1 - 40.0);
        l.positions[d] = (mid.0 + 12.0, mid.1 + 40.0);
        l.constellations[ci].edges.push((c, d));
        let _ = model;
    });
}

#[test]
fn lines_lying_on_each_other_are_caught() {
    expect_caught("line crossings", |_, l| {
        let ci = l
            .constellations
            .iter()
            .position(|c| c.stars.len() >= 3)
            .expect("a constellation");
        let (a, b) = l.constellations[ci].edges[0];
        let third = l.constellations[ci]
            .stars
            .iter()
            .copied()
            .find(|s| *s != a && *s != b)
            .expect("a third star");
        // Put the third star exactly on the line a-b, past b, and join it to a.
        let (pa, pb) = (l.positions[a], l.positions[b]);
        l.positions[third] = (pb.0 + (pb.0 - pa.0), pb.1 + (pb.1 - pa.1));
        l.constellations[ci].edges.push((a, third));
    });
}

#[test]
fn a_loose_star_next_to_a_line_is_caught() {
    expect_caught("loose stars", |_, l| {
        let (a, b) = l.constellations[0].edges[0];
        let w = l.wanderers[0];
        l.positions[w] = (
            (l.positions[a].0 + l.positions[b].0) / 2.0,
            (l.positions[a].1 + l.positions[b].1) / 2.0 + 5.0,
        );
    });
}

#[test]
fn a_split_constellation_is_caught() {
    expect_caught("connect every star", |_, l| {
        l.constellations[0].edges.clear();
    });
}

#[test]
fn two_labels_on_top_of_each_other_are_caught() {
    expect_caught("labels clear", |_, l| {
        let copy = l.labels[0].rect;
        l.labels[1].rect = copy;
    });
}

#[test]
fn a_label_outside_the_margin_is_caught() {
    expect_caught("labels clear", |_, l| {
        let r = &mut l.labels[0].rect;
        let width = r.width();
        r.x0 = -30.0;
        r.x1 = -30.0 + width;
    });
}

#[test]
fn a_label_sitting_on_a_star_is_caught() {
    expect_caught("labels clear", |_, l| {
        let p = l.positions[0];
        let r = &mut l.labels[0].rect;
        let (w, h) = (r.width(), r.height());
        *r = git_constellation::layout::Rect {
            x0: p.0 - w / 2.0,
            y0: p.1 - h / 2.0,
            x1: p.0 + w / 2.0,
            y1: p.1 + h / 2.0,
        };
    });
}

#[test]
fn a_label_on_a_line_is_caught() {
    expect_caught("labels clear", |_, l| {
        let (a, b) = l.constellations[0].edges[0];
        let mid = (
            (l.positions[a].0 + l.positions[b].0) / 2.0,
            (l.positions[a].1 + l.positions[b].1) / 2.0,
        );
        let r = &mut l.labels[0].rect;
        let (w, h) = (r.width(), r.height());
        *r = git_constellation::layout::Rect {
            x0: mid.0 - w / 2.0,
            y0: mid.1 - h / 2.0,
            x1: mid.0 + w / 2.0,
            y1: mid.1 + h / 2.0,
        };
    });
}

#[test]
fn a_lopsided_sky_is_caught() {
    expect_caught("balance", |_, l| {
        for p in &mut l.positions {
            p.0 -= 250.0;
        }
    });
    expect_caught("balance", |_, l| {
        for p in &mut l.positions {
            p.1 += 150.0;
        }
    });
}

#[test]
fn not_a_number_is_caught() {
    expect_caught("NaN", |_, l| l.positions[0].0 = f64::NAN);
    expect_caught("NaN", |_, l| l.constellations[0].anchor.1 = f64::INFINITY);
    expect_caught("NaN", |_, l| l.labels[0].rect.x1 = f64::NAN);
}

#[test]
fn a_star_without_a_position_is_caught() {
    expect_caught("every star has a position", |_, l| {
        l.positions.pop();
    });
}

#[test]
fn soft_checks_warn_but_never_fail_the_build() {
    let (mut model, layout) = good();
    // More than seven different hues is too many.
    let keys = [
        LanguageKey::Rust,
        LanguageKey::Python,
        LanguageKey::JsTs,
        LanguageKey::Go,
        LanguageKey::JavaKotlin,
        LanguageKey::CFamily,
        LanguageKey::HtmlCss,
        LanguageKey::Shell,
    ];
    for (star, key) in model.stars.iter_mut().zip(keys) {
        star.language = key;
    }
    let report = lint(&model, &layout);
    assert!(report.hard_passed());
    let hues = report
        .soft
        .iter()
        .find(|c| c.name.contains("hues"))
        .expect("hues check");
    assert!(
        !hues.passed && hues.detail.contains("at most 7"),
        "{}",
        hues.detail
    );
    assert!(report.warnings() >= 1);
}

#[test]
fn a_sparse_sky_warns_about_empty_space_and_coverage() {
    let options = Options::default();
    let (model, layout) = plan_sky(&load_fixture("tiny"), &options).unwrap();
    let report = lint(&model, &layout);
    assert!(report.hard_passed());
    let empty = report
        .soft
        .iter()
        .find(|c| c.name.contains("star-free"))
        .expect("empty-space check");
    assert!(
        !empty.passed,
        "one star leaves almost everything empty: {}",
        empty.detail
    );
}

#[test]
fn the_empty_space_measure_is_sane() {
    use git_constellation::lint::largest_empty_share;
    let (model, layout) = plan_sky(&load_fixture("empty"), &Options::default()).unwrap();
    assert!(
        (largest_empty_share(&model, &layout) - 1.0).abs() < 1e-9,
        "no stars: everything is empty"
    );
    let (model, layout) = good();
    let share = largest_empty_share(&model, &layout);
    assert!(share > 0.0 && share < 0.30, "{share}");
}

#[test]
fn the_report_lists_checks_and_what_is_not_checkable_yet() {
    let (model, layout) = good();
    let text = lint(&model, &layout).to_text();
    assert!(text.contains("HARD checks") && text.contains("SOFT checks"));
    assert!(text.contains("[pass] star spacing"));
    assert!(text.contains("Not checkable yet"));
    assert!(text.contains("contrast") && text.contains("luminance"));
    let (mut model, mut layout) = good();
    layout.positions[1] = layout.positions[0];
    model.stars[1].repo = "the-culprit".to_string();
    assert!(
        lint(&model, &layout)
            .to_text()
            .contains("[FAIL] star spacing")
    );
}

#[test]
fn a_constellation_name_next_to_the_wrong_group_is_caught() {
    expect_caught("nearer their own stars", |_, l| {
        // Park constellation 0's name right beside a star of constellation 1.
        let target = l.positions[l.constellations[1].stars[0]];
        let index = l
            .labels
            .iter()
            .position(|label| {
                label.kind == git_constellation::layout::labels::LabelKind::Constellation(0)
            })
            .expect("constellation 0 has a name");
        let r = &mut l.labels[index].rect;
        let (w, h) = (r.width(), r.height());
        *r = git_constellation::layout::Rect {
            x0: target.0 + 12.0,
            y0: target.1 - h / 2.0,
            x1: target.0 + 12.0 + w,
            y1: target.1 + h / 2.0,
        };
    });
}
