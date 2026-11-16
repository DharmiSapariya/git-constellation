//! The layout on real and synthetic skies, checked by independent brute force.

mod common;

use common::{FIXTURES, activity_with, load_fixture, repo_in};
use git_constellation::layout::labels::LabelKind;
use git_constellation::lint::{hard_failures, lint};
use git_constellation::model::{LabelMode, SkyModel};
use git_constellation::{Layout, Options, build_sky, plan_sky};

type P = (f64, f64);

fn d(a: P, b: P) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn radius(model: &SkyModel, i: usize) -> f64 {
    [2.2, 3.2, 4.6, 6.4, 8.6][usize::from(model.stars[i].tier.number()) - 1]
}

fn side(a: P, b: P, c: P) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Proper crossing of two segments with distinct endpoints (written fresh for this test).
fn properly_cross(a: P, b: P, c: P, e: P) -> bool {
    side(a, b, c) * side(a, b, e) < 0.0 && side(c, e, a) * side(c, e, b) < 0.0
}

fn in_hud(p: P, pad: f64) -> bool {
    p.0 >= 24.0 - pad && p.0 <= 330.0 + pad && p.1 >= 24.0 - pad && p.1 <= 82.0 + pad
}

fn on_segment_samples(a: P, b: P) -> Vec<P> {
    let n = (d(a, b) / 0.5).ceil().max(1.0) as usize;
    (0..=n)
        .map(|k| {
            (
                a.0 + (b.0 - a.0) * k as f64 / n as f64,
                a.1 + (b.1 - a.1) * k as f64 / n as f64,
            )
        })
        .collect()
}

/// Kruskal minimum-spanning-tree weight of a set of points.
fn mst_weight(points: &[P]) -> f64 {
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            pairs.push((d(points[i], points[j]), i, j));
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut group: Vec<usize> = (0..points.len()).collect();
    let mut total = 0.0;
    for (w, i, j) in pairs {
        let (gi, gj) = (group[i], group[j]);
        if gi != gj {
            total += w;
            for g in group.iter_mut().filter(|g| **g == gj) {
                *g = gi;
            }
        }
    }
    total
}

/// Check every promise of Section 5 and 10.3 without using the linter.
fn verify(model: &SkyModel, layout: &Layout, what: &str) {
    let n = model.stars.len();
    assert_eq!(layout.positions.len(), n, "{what}: one position per star");

    // Every star is in exactly one place: a constellation or the loose list.
    let mut seen = vec![0; n];
    for c in &layout.constellations {
        for &i in &c.stars {
            seen[i] += 1;
        }
    }
    for &i in &layout.wanderers {
        seen[i] += 1;
    }
    assert!(seen.iter().all(|&c| c == 1), "{what}: membership {seen:?}");

    // Star spacing, margins and reserved areas.
    for i in 0..n {
        let p = layout.positions[i];
        assert!(p.0.is_finite() && p.1.is_finite(), "{what}: finite");
        assert!(
            (40.0..=1160.0).contains(&p.0) && (40.0..=560.0).contains(&p.1),
            "{what}: star {i} in the margin"
        );
        assert!(!in_hud(p, radius(model, i)), "{what}: star {i} in the HUD");
        assert!(
            d(p, (1090.0, 112.0)) >= 90.0 + radius(model, i),
            "{what}: star {i} in the moon area"
        );
        for j in (i + 1)..n {
            let (ri, rj) = (radius(model, i), radius(model, j));
            let need = (4.0 * ri.max(rj) + 14.0).max(ri + rj + 28.0);
            assert!(
                d(p, layout.positions[j]) >= need - 1e-6,
                "{what}: stars {i} and {j} are too close"
            );
        }
    }

    // Constellation anchors keep their gaps.
    for (a, ca) in layout.constellations.iter().enumerate() {
        for cb in layout.constellations.iter().skip(a + 1) {
            let gap = ca.base_radius + cb.base_radius + 30.0 * layout.scale;
            assert!(
                d(ca.anchor, cb.anchor) >= gap - 1e-6,
                "{what}: anchors overlap"
            );
        }
    }

    // Lines: connected, MST-based, and never crossing anything.
    let mut all_edges = Vec::new();
    for (ci, c) in layout.constellations.iter().enumerate() {
        let pts: Vec<P> = c.stars.iter().map(|&i| layout.positions[i]).collect();
        if pts.len() >= 2 {
            assert!(
                c.edges.len() >= pts.len() - 1,
                "{what}: constellation {ci} has too few lines"
            );
            assert!(
                c.edges.len() <= pts.len() - 1 + pts.len() / 4,
                "{what}: constellation {ci} has too many lines"
            );
            // Kruskal on just our edges must reach the true MST weight, so our edges contain an MST.
            let local = |s: usize| {
                c.stars
                    .iter()
                    .position(|&x| x == s)
                    .expect("edge star is a member")
            };
            let mut weighted: Vec<(f64, usize, usize)> = c
                .edges
                .iter()
                .map(|&(a, b)| {
                    (
                        d(layout.positions[a], layout.positions[b]),
                        local(a),
                        local(b),
                    )
                })
                .collect();
            weighted.sort_by(|x, y| x.0.total_cmp(&y.0));
            let mut group: Vec<usize> = (0..pts.len()).collect();
            let (mut total, mut joins) = (0.0, 0);
            for (w, i, j) in weighted {
                let (gi, gj) = (group[i], group[j]);
                if gi != gj {
                    total += w;
                    joins += 1;
                    for g in group.iter_mut().filter(|g| **g == gj) {
                        *g = gi;
                    }
                }
            }
            assert_eq!(
                joins,
                pts.len() - 1,
                "{what}: constellation {ci} is not connected"
            );
            assert!(
                (total - mst_weight(&pts)).abs() < 1e-6,
                "{what}: constellation {ci} lost its MST"
            );
        }
        all_edges.extend(c.edges.iter().copied());
    }
    for (x, &(a, b)) in all_edges.iter().enumerate() {
        for &(c, e) in &all_edges[x + 1..] {
            if a == c || a == e || b == c || b == e {
                let (s, p, q) = if a == c || a == e {
                    (a, b, if c == a { e } else { c })
                } else {
                    (b, a, if c == b { e } else { c })
                };
                let (u, v) = (
                    (
                        layout.positions[p].0 - layout.positions[s].0,
                        layout.positions[p].1 - layout.positions[s].1,
                    ),
                    (
                        layout.positions[q].0 - layout.positions[s].0,
                        layout.positions[q].1 - layout.positions[s].1,
                    ),
                );
                let cross = (u.0 * v.1 - u.1 * v.0).abs();
                let dot = u.0 * v.0 + u.1 * v.1;
                assert!(
                    cross > 1e-6 || dot < 0.0,
                    "{what}: two lines lie on top of each other"
                );
            } else {
                let (pa, pb, pc, pe) = (
                    layout.positions[a],
                    layout.positions[b],
                    layout.positions[c],
                    layout.positions[e],
                );
                assert!(
                    !properly_cross(pa, pb, pc, pe),
                    "{what}: lines {a}-{b} and {c}-{e} cross"
                );
            }
        }
    }

    // Loose stars stay 30 px from every line.
    for &w in &layout.wanderers {
        for &(a, b) in &all_edges {
            let nearest = on_segment_samples(layout.positions[a], layout.positions[b])
                .into_iter()
                .map(|q| d(layout.positions[w], q))
                .fold(f64::INFINITY, f64::min);
            assert!(
                nearest >= 29.0,
                "{what}: loose star {w} is {nearest:.1}px from a line"
            );
        }
    }

    // Balance.
    if n > 0 {
        let cx = layout.positions.iter().map(|p| p.0).sum::<f64>() / n as f64;
        let cy = layout.positions.iter().map(|p| p.1).sum::<f64>() / n as f64;
        assert!(
            (cx - 600.0).abs() <= 0.12 * 1200.0 + 1e-6,
            "{what}: centroid x {cx}"
        );
        assert!(
            (cy - 300.0).abs() <= 0.15 * 600.0 + 1e-6,
            "{what}: centroid y {cy}"
        );
    }

    // Labels: inside the margin, clear of everything, and not on top of each other.
    for (k, label) in layout.labels.iter().enumerate() {
        let r = &label.rect;
        assert!(
            r.x0 >= 40.0 && r.y0 >= 40.0 && r.x1 <= 1160.0 && r.y1 <= 560.0,
            "{what}: label {k} outside the margin"
        );
        assert!(r.x1 > r.x0 && r.y1 > r.y0, "{what}: label {k} has no size");
        assert!(
            !(r.x0 < 330.0 && r.x1 > 24.0 && r.y0 < 82.0 && r.y1 > 24.0),
            "{what}: label {k} in the HUD"
        );
        let nearest_x = (1090.0f64).clamp(r.x0, r.x1);
        let nearest_y = (112.0f64).clamp(r.y0, r.y1);
        assert!(
            d((nearest_x, nearest_y), (1090.0, 112.0)) >= 90.0,
            "{what}: label {k} in the moon area"
        );
        for (i, &p) in layout.positions.iter().enumerate() {
            let (cx, cy) = (p.0.clamp(r.x0, r.x1), p.1.clamp(r.y0, r.y1));
            assert!(
                d((cx, cy), p) >= 1.8 * radius(model, i) - 1e-6,
                "{what}: label {k} sits on star {i}"
            );
        }
        for &(a, b) in &all_edges {
            let hit = on_segment_samples(layout.positions[a], layout.positions[b])
                .into_iter()
                .any(|q| q.0 > r.x0 && q.0 < r.x1 && q.1 > r.y0 && q.1 < r.y1);
            assert!(!hit, "{what}: label {k} sits on line {a}-{b}");
        }
        if let LabelKind::Constellation(ci) = label.kind {
            let to = |i: usize| {
                let p = layout.positions[i];
                d((p.0.clamp(r.x0, r.x1), p.1.clamp(r.y0, r.y1)), p)
            };
            let own = layout.constellations[ci]
                .stars
                .iter()
                .map(|&i| to(i))
                .fold(f64::INFINITY, f64::min);
            let foreign = layout
                .constellations
                .iter()
                .enumerate()
                .filter(|(o, _)| *o != ci)
                .flat_map(|(_, c)| c.stars.iter().map(|&i| to(i)))
                .fold(f64::INFINITY, f64::min);
            assert!(
                own < foreign,
                "{what}: the name of constellation {ci} is {own:.1}px from its own stars but {foreign:.1}px from another group"
            );
        }
        for other in &layout.labels[k + 1..] {
            let o = &other.rect;
            assert!(
                !(r.x0 < o.x1 && o.x0 < r.x1 && r.y0 < o.y1 && o.y0 < r.y1),
                "{what}: two labels overlap"
            );
        }
    }
}

fn plan(name: &str, mode: LabelMode) -> (SkyModel, Layout) {
    let options = Options {
        labels: mode,
        ..Options::default()
    };
    plan_sky(&load_fixture(name), &options).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn every_fixture_lays_out_cleanly_in_every_label_mode() {
    for name in FIXTURES {
        for mode in [LabelMode::Off, LabelMode::Major, LabelMode::All] {
            let (model, layout) = plan(name, mode);
            let what = format!("{name}/{mode:?}");
            verify(&model, &layout, &what);
            let report = lint(&model, &layout);
            assert!(report.hard_passed(), "{what}:\n{}", report.to_text());
            assert!(hard_failures(&model, &layout).is_empty(), "{what}");
        }
    }
}

#[test]
fn the_layout_is_deterministic() {
    for name in FIXTURES {
        assert_eq!(
            plan(name, LabelMode::Major),
            plan(name, LabelMode::Major),
            "{name}"
        );
    }
}

#[test]
fn repo_order_in_the_input_does_not_change_the_layout() {
    for name in ["small", "medium", "edge"] {
        let mut activity = load_fixture(name);
        let a = plan_sky(&activity, &Options::default()).unwrap();
        activity.repos.reverse();
        let b = plan_sky(&activity, &Options::default()).unwrap();
        assert_eq!(a, b, "{name}");
    }
}

#[test]
fn a_golden_position_pins_the_whole_algorithm() {
    // If any layout step changes, this moves. Update it only on purpose.
    let (_, layout) = plan("medium", LabelMode::Major);
    let first = layout.positions[0];
    let rounded = (
        (first.0 * 100.0).round() / 100.0,
        (first.1 * 100.0).round() / 100.0,
    );
    assert_eq!(rounded, GOLDEN_MEDIUM_STAR_0, "got {rounded:?}");
}
const GOLDEN_MEDIUM_STAR_0: (f64, f64) = (353.39, 190.65);

#[test]
fn a_different_seed_gives_a_different_but_still_clean_layout() {
    let activity = load_fixture("medium");
    let base = plan_sky(&activity, &Options::default()).unwrap().1;
    let mut differing = 0;
    for seed in 1..=12 {
        let options = Options {
            seed: Some(seed),
            ..Options::default()
        };
        let (model, layout) = plan_sky(&activity, &options).unwrap();
        verify(&model, &layout, &format!("medium seed {seed}"));
        if layout.positions != base.positions {
            differing += 1;
        }
    }
    assert!(
        differing >= 10,
        "seeds should change the picture ({differing} of 12 did)"
    );
}

#[test]
fn many_seeds_stay_clean_on_the_big_fixtures() {
    for (name, seeds) in [("large", 6), ("edge", 10), ("small", 10)] {
        let activity = load_fixture(name);
        for seed in 100..100 + seeds {
            let options = Options {
                seed: Some(seed),
                labels: LabelMode::Major,
                ..Options::default()
            };
            let (model, layout) = plan_sky(&activity, &options).unwrap();
            verify(&model, &layout, &format!("{name} seed {seed}"));
        }
    }
}

/// A made-up sky: `count` repos over `languages` languages with varied scores.
fn synthetic(count: usize, languages: usize) -> git_constellation::Activity {
    let names = ["Rust", "Python", "Go", "TypeScript", "Java", "C", "HTML"];
    let mut state = 12345u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u32
    };
    let repos = (0..count)
        .map(|i| {
            let mut r = repo_in(
                &format!("repo-{i:03}"),
                Some(names[i % languages]),
                next() % 40,
            );
            r.commits_last_year = next() % 200;
            r.last_pushed = common::days_ago(u64::from(next() % 300));
            r
        })
        .collect();
    activity_with(repos)
}

#[test]
fn synthetic_skies_of_many_sizes_lay_out_cleanly() {
    for &(count, languages) in &[
        (3, 1),
        (7, 2),
        (12, 3),
        (20, 4),
        (30, 5),
        (45, 6),
        (60, 7),
        (90, 7),
        (150, 5),
    ] {
        let activity = synthetic(count, languages);
        for seed in [1, 2] {
            let options = Options {
                seed: Some(seed),
                ..Options::default()
            };
            let (model, layout) = plan_sky(&activity, &options).unwrap();
            verify(
                &model,
                &layout,
                &format!("{count} repos/{languages} langs/seed {seed}"),
            );
        }
    }
}

#[test]
fn apply_to_fills_the_models_own_fields() {
    let (mut model, layout) = plan("medium", LabelMode::Major);
    assert!(model.stars.iter().all(|s| s.position.is_none()));
    layout.apply_to(&mut model);
    for (i, star) in model.stars.iter().enumerate() {
        assert_eq!(star.position, Some(layout.positions[i]));
    }
    for (ci, c) in model.constellations.iter().enumerate() {
        assert_eq!(c.anchor, Some(layout.constellations[ci].anchor));
        assert_eq!(c.edges, layout.constellations[ci].edges);
        assert!(c.radius.is_some());
        for &si in &c.stars {
            assert_eq!(model.stars[si].constellation, Some(ci));
        }
    }
    assert_eq!(model.wanderers, layout.wanderers);
}

#[test]
fn a_sky_with_fewer_than_three_stars_is_a_young_centered_row() {
    for name in ["empty", "tiny"] {
        let (model, layout) = plan(name, LabelMode::Major);
        assert!(model.is_young() && layout.constellations.is_empty());
        assert!(
            layout
                .labels
                .iter()
                .any(|l| l.kind == LabelKind::Caption && l.text == "a young sky"),
            "{name}"
        );
        for p in &layout.positions {
            assert!(
                (p.0 - 600.0).abs() < 100.0 && (p.1 - 320.0).abs() < 30.0,
                "{name}: {p:?}"
            );
        }
    }
    // Even with labels off, the caption stays.
    let (_, layout) = plan("tiny", LabelMode::Off);
    assert_eq!(layout.labels.len(), 1);
}

#[test]
fn labels_off_means_no_labels_but_labels_major_names_everything_important() {
    let (model, off) = plan("medium", LabelMode::Off);
    assert!(off.labels.is_empty());
    let (_, major) = plan("medium", LabelMode::Major);
    for ci in 0..model.constellations.len() {
        assert!(
            major
                .labels
                .iter()
                .any(|l| l.kind == LabelKind::Constellation(ci)),
            "constellation {ci} has a name"
        );
    }
    let big: Vec<usize> = model
        .stars
        .iter()
        .enumerate()
        .filter(|(_, s)| s.tier.number() >= 4)
        .map(|(i, _)| i)
        .collect();
    let named: Vec<usize> = major
        .labels
        .iter()
        .filter_map(|l| {
            if let LabelKind::Star(i) = l.kind {
                Some(i)
            } else {
                None
            }
        })
        .collect();
    assert!(
        big.iter().all(|i| named.contains(i)),
        "tier 4 and 5 are named: {big:?} vs {named:?}"
    );
    assert!(
        named.iter().all(|i| big.contains(i)),
        "and nothing smaller in major mode"
    );
    assert!(
        major
            .labels
            .iter()
            .any(|l| matches!(l.kind, LabelKind::FirstLight(_)))
    );
    let (_, all) = plan("medium", LabelMode::All);
    assert!(all.labels.len() > major.labels.len());
}

#[test]
fn constellation_names_are_uppercase_and_repo_names_are_cut() {
    let (model, layout) = plan("edge", LabelMode::All);
    for l in &layout.labels {
        match l.kind {
            LabelKind::Constellation(ci) => {
                assert_eq!(l.text, model.constellations[ci].name.to_uppercase())
            }
            LabelKind::Star(i) => {
                assert!(l.text.chars().count() <= 18);
                if model.stars[i].repo.chars().count() > 18 {
                    assert!(l.text.ends_with('\u{2026}'));
                }
            }
            _ => {}
        }
    }
}

#[test]
fn an_impossible_sky_reports_crowding_instead_of_a_bad_layout() {
    // 240 flagship stars need about 48 px of room each: far more than the canvas has.
    let mut model = build_sky(&synthetic(60, 6), &Options::default()).unwrap();
    let template = model.stars[0].clone();
    while model.stars.len() < 240 {
        model.stars.push(template.clone());
        model.wanderers.push(model.stars.len() - 1);
    }
    for star in &mut model.stars {
        star.tier = git_constellation::model::Tier::Five;
    }
    let error = git_constellation::layout::compute(&model, &Options::default())
        .expect_err("240 flagship stars cannot fit");
    assert!(error.to_string().contains("crowded"), "{error}");
}

#[test]
fn a_crowded_activity_still_gets_a_clean_sky_by_lowering_the_star_cap() {
    // Every repo starts at the cap of 60; plan_sky may lower it, never fail.
    let activity = synthetic(150, 7);
    let (model, layout) = plan_sky(&activity, &Options::default()).unwrap();
    assert!(model.stars.len() <= 60);
    match layout.star_cap {
        Some(cap) => assert!(model.stars.len() <= cap && cap < 60),
        None => assert_eq!(model.stars.len(), 60),
    }
    verify(&model, &layout, "150 repos");
}

#[test]
fn the_star_cap_steps_down_by_a_fifth_until_one() {
    use git_constellation::lower_star_cap;
    assert_eq!(lower_star_cap(60), Some(48));
    assert_eq!(lower_star_cap(48), Some(38));
    assert_eq!(lower_star_cap(5), Some(4));
    assert_eq!(lower_star_cap(2), Some(1));
    assert_eq!(lower_star_cap(1), None);
    assert_eq!(lower_star_cap(0), None);
    // From any cap it strictly decreases and always ends.
    for start in 1..=60 {
        let mut cap = start;
        let mut steps = 0;
        while let Some(next) = lower_star_cap(cap) {
            assert!(next < cap && next >= 1);
            cap = next;
            steps += 1;
            assert!(steps < 100);
        }
        assert_eq!(cap, 1);
    }
}

#[test]
fn the_brightest_constellation_sits_center_left_and_mid_height() {
    // The anchor starts in 25-45% of the width and 30-70% of the height; the fit
    // step may then scale (up to 1.25x) and nudge it a little, so allow for that.
    for name in ["small", "medium", "large", "edge"] {
        let (_, layout) = plan(name, LabelMode::Major);
        let anchor = layout.constellations[0].anchor;
        assert!(
            anchor.0 > 0.15 * 1200.0 && anchor.0 < 0.55 * 1200.0,
            "{name}: x = {}",
            anchor.0
        );
        assert!(
            anchor.1 > 0.2 * 600.0 && anchor.1 < 0.8 * 600.0,
            "{name}: y = {}",
            anchor.1
        );
    }
}

#[test]
fn composition_quality_is_pinned_per_fixture() {
    // Soft warnings per fixture with the default labels. If the layout gets worse
    // (or better), this changes; update it on purpose after looking at the pictures.
    let expected = [
        ("empty", 1),
        ("tiny", 1),
        ("small", 1),
        ("medium", 0),
        ("large", 0),
        ("edge", 0),
    ];
    for (name, warnings) in expected {
        let (model, layout) = plan(name, LabelMode::Major);
        assert_eq!(
            lint(&model, &layout).warnings(),
            warnings,
            "{name}:\n{}",
            lint(&model, &layout).to_text()
        );
    }
}

#[test]
fn picking_the_best_clean_layout_never_picks_a_worse_one_than_the_first() {
    // The edge fixture's first clean layout has more warnings than the one chosen.
    let (model, layout) = plan("edge", LabelMode::Major);
    assert!(layout.attempt > 0, "a later attempt was chosen");
    assert!(lint(&model, &layout).warnings() == 0);
}

#[test]
fn small_skies_use_staggered_slots_for_good_coverage() {
    use git_constellation::layout::anchors::SMALL_SKY_MAX_STARS;
    for name in ["small", "edge"] {
        let (model, layout) = plan(name, LabelMode::Major);
        assert!(
            model.stars.len() <= SMALL_SKY_MAX_STARS,
            "{name} counts as a small sky"
        );
        let ys: Vec<f64> = layout.constellations.iter().map(|c| c.anchor.1).collect();
        let spread = ys.iter().cloned().fold(f64::MIN, f64::max)
            - ys.iter().cloned().fold(f64::MAX, f64::min);
        assert!(
            spread > 100.0,
            "{name}: groups are staggered high and low (spread {spread:.0}px)"
        );
        let report = lint(&model, &layout);
        let coverage = report
            .soft
            .iter()
            .find(|c| c.name.contains("coverage"))
            .expect("coverage check");
        assert!(coverage.passed, "{name}: {}", coverage.detail);
    }
}

#[test]
fn bigger_skies_do_not_use_the_slots() {
    // The medium sky (22 stars) uses seeded random anchors: its groups are not
    // pinned to the small-sky slot positions.
    let (_, layout) = plan("medium", LabelMode::Major);
    assert!(layout.constellations.len() >= 5);
}

#[test]
fn small_sky_slots_exist_for_one_to_four_groups_and_stay_on_the_canvas() {
    use git_constellation::layout::anchors::SMALL_SKY_SLOTS;
    for (k, slots) in SMALL_SKY_SLOTS.iter().enumerate() {
        assert_eq!(slots.len(), k + 1);
        assert!(
            (0.25..=0.45).contains(&slots[0].0) && (0.30..=0.70).contains(&slots[0].1),
            "the brightest slot is center-left"
        );
        for &(x, y) in *slots {
            assert!((0.1..=0.9).contains(&x) && (0.2..=0.8).contains(&y));
        }
    }
}
