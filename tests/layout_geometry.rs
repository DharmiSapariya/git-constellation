//! Layout building blocks: randomness, Halton, rectangles, segments, MST, text sizes.

use git_constellation::layout::anchors::{SeededRng, halton, radius_estimate};
use git_constellation::layout::labels::{
    MAX_NAME_CHARS, NAME_OWNER_RATIO, name_belongs, text_width, truncate_name,
};
use git_constellation::layout::lines::{
    angle_at, build_edges, edges_cross, point_segment_distance, segments_cross,
};
use git_constellation::layout::{
    HUD_RECT, MOON_CENTER, Rect, Region, min_dist, reserved_regions, safe_rect, segment_hits_rect,
    star_area, star_radius,
};
use git_constellation::model::Tier;
use git_constellation::svg::fmt as num;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn halton_matches_the_textbook_values() {
    let base2 = [0.5, 0.25, 0.75, 0.125, 0.625];
    for (i, want) in base2.iter().enumerate() {
        assert!(
            close(halton(i as u64 + 1, 2), *want),
            "base 2 index {}",
            i + 1
        );
    }
    let base3 = [1.0 / 3.0, 2.0 / 3.0, 1.0 / 9.0, 4.0 / 9.0];
    for (i, want) in base3.iter().enumerate() {
        assert!(
            close(halton(i as u64 + 1, 3), *want),
            "base 3 index {}",
            i + 1
        );
    }
    assert_eq!(halton(0, 2), 0.0);
}

#[test]
fn halton_stays_in_the_unit_interval_and_spreads_out() {
    let values: Vec<f64> = (1..=200).map(|i| halton(i, 2)).collect();
    assert!(values.iter().all(|v| (0.0..1.0).contains(v)));
    let below_half = values.iter().filter(|v| **v < 0.5).count();
    assert!((90..=110).contains(&below_half), "{below_half}");
}

#[test]
fn the_same_layer_gives_the_same_stream_and_other_layers_differ() {
    let draw = |seed, layer: &str, index, attempt| {
        let mut rng = SeededRng::for_layer(seed, layer, index, attempt);
        (rng.unit(), rng.unit(), rng.unit())
    };
    assert_eq!(draw(42, "shape", 0, 0), draw(42, "shape", 0, 0));
    assert_ne!(draw(42, "shape", 0, 0), draw(43, "shape", 0, 0));
    assert_ne!(draw(42, "shape", 0, 0), draw(42, "wanderers", 0, 0));
    assert_ne!(draw(42, "shape", 0, 0), draw(42, "shape", 1, 0));
    assert_ne!(draw(42, "shape", 0, 0), draw(42, "shape", 0, 1));
}

#[test]
fn the_random_stream_is_pinned() {
    // Regression guard: if this changes, every layout in the world changes.
    let mut rng = SeededRng::for_layer(42, "anchors", 0, 0);
    let first = rng.unit();
    assert!((first - PINNED_FIRST_UNIT).abs() < 1e-15, "got {first:.17}");
}
const PINNED_FIRST_UNIT: f64 = 0.894_590_247_854_781_7;

#[test]
fn random_numbers_have_the_right_shape() {
    let mut rng = SeededRng::for_layer(7, "stats", 0, 0);
    let units: Vec<f64> = (0..20_000).map(|_| rng.unit()).collect();
    assert!(units.iter().all(|u| (0.0..1.0).contains(u)));
    let mean = units.iter().sum::<f64>() / units.len() as f64;
    assert!((mean - 0.5).abs() < 0.01, "mean {mean}");

    let normals: Vec<f64> = (0..20_000).map(|_| rng.normal()).collect();
    let mean = normals.iter().sum::<f64>() / normals.len() as f64;
    let var = normals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / normals.len() as f64;
    assert!(
        mean.abs() < 0.03 && (var - 1.0).abs() < 0.05,
        "mean {mean} var {var}"
    );
    assert!(normals.iter().all(|v| v.is_finite()));
}

#[test]
fn below_stays_in_range_and_reaches_every_value() {
    let mut rng = SeededRng::for_layer(1, "below", 0, 0);
    let mut seen = [false; 5];
    for _ in 0..500 {
        seen[rng.below(5)] = true;
    }
    assert!(seen.iter().all(|s| *s));
    assert_eq!(rng.below(0), 0);
    assert_eq!(rng.below(1), 0);
}

#[test]
fn radius_estimate_follows_the_formula_and_clamps() {
    assert!(close(radius_estimate(1), 92.0));
    assert!(close(radius_estimate(3), 70.0 + 22.0 * 3f64.sqrt()));
    assert!(close(radius_estimate(16), 158.0));
    assert!(close(radius_estimate(0), 90.0));
    assert!(close(radius_estimate(1000), 200.0));
}

#[test]
fn star_sizes_and_spacing_follow_the_spec() {
    let radii: Vec<f64> = [Tier::One, Tier::Two, Tier::Three, Tier::Four, Tier::Five]
        .map(star_radius)
        .to_vec();
    assert_eq!(radii, [2.2, 3.2, 4.6, 6.4, 8.6]);
    // Small stars are held to the 28 px breathing room, big ones to 4*r + 14.
    assert!(close(min_dist(2.2, 2.2), 2.2 + 2.2 + 28.0));
    assert!(close(min_dist(8.6, 8.6), 4.0 * 8.6 + 14.0));
    assert!(close(min_dist(8.6, 2.2), 4.0 * 8.6 + 14.0));
    assert!(close(min_dist(3.2, 6.4), min_dist(6.4, 3.2)));
}

#[test]
fn rectangles_behave() {
    let r = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 10.0,
        y1: 5.0,
    };
    assert!(close(r.width(), 10.0) && close(r.height(), 5.0));
    assert!(r.contains((10.0, 5.0)) && !r.contains((10.1, 5.0)));
    assert!(close(r.distance_to_point((13.0, 9.0)), 5.0));
    assert!(close(r.distance_to_point((4.0, 2.0)), 0.0));
    let touching = Rect {
        x0: 10.0,
        y0: 0.0,
        x1: 20.0,
        y1: 5.0,
    };
    assert!(!r.overlaps(&touching), "touching edges are not an overlap");
    assert!(r.overlaps(&Rect {
        x0: 9.9,
        ..touching
    }));
    assert!(r.inflate(1.0).contains((-1.0, -1.0)));
    assert!(r.contains_rect(&Rect {
        x0: 1.0,
        y0: 1.0,
        x1: 2.0,
        y1: 2.0
    }));
    assert!(!r.contains_rect(&Rect {
        x0: 1.0,
        y0: 1.0,
        x1: 12.0,
        y1: 2.0
    }));
}

#[test]
fn the_safe_areas_are_what_the_spec_says() {
    let safe = safe_rect();
    assert_eq!(
        (safe.x0, safe.y0, safe.x1, safe.y1),
        (40.0, 40.0, 1160.0, 560.0)
    );
    let stars = star_area();
    assert!(
        stars.x0 > safe.x0 && stars.y1 < safe.y1,
        "star centers are kept a little farther in"
    );
}

#[test]
fn reserved_regions_block_what_they_should() {
    let regions = reserved_regions(false);
    assert_eq!(regions.len(), 2);
    assert_eq!(reserved_regions(true).len(), 3);
    let hud = Region::Rect(HUD_RECT);
    assert!(hud.blocks_point((100.0, 50.0), 0.0));
    assert!(!hud.blocks_point((340.0, 50.0), 0.0));
    assert!(
        hud.blocks_point((335.0, 50.0), 6.0),
        "padding widens the area"
    );
    let moon = Region::Circle {
        center: MOON_CENTER,
        radius: 90.0,
    };
    assert!(moon.blocks_point((1090.0, 112.0), 0.0));
    assert!(moon.blocks_point((1170.0, 112.0), 0.0));
    assert!(!moon.blocks_point((1190.0, 112.0), 0.0));
    let tile = Rect {
        x0: 1000.0,
        y0: 100.0,
        x1: 1060.0,
        y1: 130.0,
    };
    assert!(moon.overlaps_rect(&tile));
    assert!(!moon.overlaps_rect(&Rect {
        x0: 100.0,
        y0: 400.0,
        x1: 160.0,
        y1: 430.0
    }));
    assert!(hud.overlaps_rect(&Rect {
        x0: 300.0,
        y0: 60.0,
        x1: 400.0,
        y1: 100.0
    }));
}

#[test]
fn segments_cross_in_every_documented_case() {
    let (a, b) = ((0.0, 0.0), (10.0, 10.0));
    assert!(segments_cross(a, b, (0.0, 10.0), (10.0, 0.0)), "an X");
    assert!(!segments_cross(a, b, (0.0, 1.0), (10.0, 11.0)), "parallel");
    assert!(!segments_cross(a, b, (20.0, 0.0), (30.0, 5.0)), "far apart");
    assert!(
        segments_cross(a, b, (5.0, 5.0), (9.0, 1.0)),
        "a T: one end on the other line"
    );
    assert!(
        segments_cross(a, b, (5.0, 5.0), (5.0, 5.0)),
        "a point on the line"
    );
    assert!(
        segments_cross(a, b, (3.0, 3.0), (7.0, 7.0)),
        "overlapping on one line"
    );
    assert!(
        !segments_cross(a, b, (11.0, 11.0), (20.0, 20.0)),
        "same line, not touching"
    );
}

#[test]
fn edges_sharing_a_star_only_cross_when_they_lie_on_each_other() {
    let positions = [
        (0.0, 0.0),
        (10.0, 0.0),
        (0.0, 10.0),
        (20.0, 0.0),
        (5.0, 0.0),
    ];
    assert!(!edges_cross(&positions, (0, 1), (0, 2)), "an L shape");
    assert!(
        edges_cross(&positions, (0, 3), (0, 1)),
        "one runs along the other"
    );
    assert!(edges_cross(&positions, (0, 3), (0, 4)));
    assert!(
        !edges_cross(&positions, (1, 2), (3, 2)),
        "meet at star 2 at a real angle"
    );
}

#[test]
fn point_to_segment_distance_covers_ends_and_middle() {
    let (a, b) = ((0.0, 0.0), (10.0, 0.0));
    assert!(close(point_segment_distance((5.0, 3.0), a, b), 3.0));
    assert!(
        close(point_segment_distance((-4.0, 3.0), a, b), 5.0),
        "past the start"
    );
    assert!(
        close(point_segment_distance((14.0, -3.0), a, b), 5.0),
        "past the end"
    );
    assert!(
        close(point_segment_distance((1.0, 1.0), a, a), 2f64.sqrt()),
        "a zero-length segment"
    );
}

#[test]
fn angles_between_edges() {
    let s = (0.0, 0.0);
    assert!(close(angle_at(s, (1.0, 0.0), (0.0, 1.0)), 90.0));
    assert!(close(angle_at(s, (1.0, 0.0), (-1.0, 0.0)), 180.0));
    assert!(angle_at(s, (1.0, 0.0), (5.0, 0.0)) < 1e-6);
    assert!((angle_at(s, (1.0, 0.0), (1.0, 0.364)) - 20.0).abs() < 0.1);
}

#[test]
fn segment_rect_clipping() {
    let r = Rect {
        x0: 10.0,
        y0: 10.0,
        x1: 20.0,
        y1: 20.0,
    };
    assert!(
        segment_hits_rect((0.0, 15.0), (30.0, 15.0), &r),
        "straight through"
    );
    assert!(
        segment_hits_rect((12.0, 12.0), (14.0, 14.0), &r),
        "fully inside"
    );
    assert!(
        segment_hits_rect((0.0, 0.0), (15.0, 15.0), &r),
        "ends inside"
    );
    assert!(
        !segment_hits_rect((0.0, 0.0), (30.0, 5.0), &r),
        "passes below"
    );
    assert!(
        !segment_hits_rect((0.0, 25.0), (9.0, 25.0), &r),
        "stops short"
    );
    assert!(
        segment_hits_rect((15.0, 0.0), (15.0, 30.0), &r),
        "vertical through"
    );
}

fn spans_everything(n: usize, edges: &[(usize, usize)]) -> bool {
    let mut reached = vec![0usize];
    loop {
        let before = reached.len();
        for &(a, b) in edges {
            for (from, to) in [(a, b), (b, a)] {
                if reached.contains(&from) && !reached.contains(&to) {
                    reached.push(to);
                }
            }
        }
        if reached.len() == before {
            return reached.len() == n;
        }
    }
}

#[test]
fn edges_for_tiny_inputs() {
    assert!(build_edges(&[], &[], &[]).is_empty());
    assert!(build_edges(&[(5.0, 5.0)], &[2.2], &[]).is_empty());
    assert_eq!(
        build_edges(&[(0.0, 0.0), (50.0, 0.0)], &[2.2, 2.2], &[]),
        vec![(0, 1)]
    );
}

#[test]
fn the_tree_picks_the_shortest_connections() {
    // Four stars on a 40 x 70 rectangle: the tree is the two short sides plus
    // one long side, never the diagonal.
    let points = [(0.0, 0.0), (40.0, 0.0), (0.0, 70.0), (40.0, 70.0)];
    let radii = [2.2; 4];
    let edges = build_edges(&points, &radii, &[]);
    assert!(spans_everything(4, &edges));
    assert!(edges.contains(&(0, 1)) && edges.contains(&(2, 3)));
    assert!(!edges.contains(&(0, 3)) && !edges.contains(&(1, 2)));
    // 4 stars -> floor(4/4) = 1 extra edge, so 4 edges in total (a square).
    assert_eq!(edges.len(), 4);
}

#[test]
fn the_extra_edge_count_is_floor_n_over_four_at_most() {
    // A wobbly ring of 9 stars: the tree has 8 edges, and 2 more may be added.
    let ring: Vec<(f64, f64)> = (0..9)
        .map(|k| {
            let a = k as f64 * std::f64::consts::TAU / 9.0;
            (200.0 + 80.0 * a.cos(), 200.0 + 80.0 * a.sin())
        })
        .collect();
    let edges = build_edges(&ring, &[3.2; 9], &[]);
    assert!(spans_everything(9, &edges));
    assert!((8..=10).contains(&edges.len()), "{}", edges.len());
    for (i, a) in edges.iter().enumerate() {
        for b in &edges[i + 1..] {
            assert!(!edges_cross(&ring, *a, *b), "{a:?} crosses {b:?}");
        }
    }
}

#[test]
fn extra_edges_avoid_other_constellations_lines() {
    // A square: the tree is A-B, A-D, B-C and the best extra edge would be D-C.
    // Another constellation's short line crosses D-C (and only D-C).
    let points = [(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)];
    let wall = [((20.0, 35.0), (20.0, 45.0))];
    let edges = build_edges(&points, &[2.2; 4], &wall);
    assert!(spans_everything(4, &edges));
    assert!(
        !edges.contains(&(2, 3)),
        "D-C would cross the wall: {edges:?}"
    );
    for &(a, b) in &edges {
        assert!(
            !segments_cross(points[a], points[b], wall[0].0, wall[0].1),
            "{a}-{b} crosses the wall"
        );
    }
    // Without the wall, D-C is accepted as the extra edge.
    assert!(build_edges(&points, &[2.2; 4], &[]).contains(&(2, 3)));
}

#[test]
fn the_tree_is_identical_every_time_even_with_ties() {
    let points = [
        (0.0, 0.0),
        (50.0, 0.0),
        (0.0, 50.0),
        (50.0, 50.0),
        (25.0, 25.0),
    ];
    let first = build_edges(&points, &[3.2; 5], &[]);
    for _ in 0..5 {
        assert_eq!(build_edges(&points, &[3.2; 5], &[]), first);
    }
}

#[test]
fn repo_names_are_cut_to_eighteen_characters_with_one_ellipsis() {
    assert_eq!(truncate_name("short"), "short");
    assert_eq!(
        truncate_name("exactly-eighteen-ch"),
        "exactly-eighteen-\u{2026}"
    );
    let name18 = "a".repeat(MAX_NAME_CHARS);
    assert_eq!(truncate_name(&name18), name18);
    let long = truncate_name(&"b".repeat(40));
    assert_eq!(long.chars().count(), MAX_NAME_CHARS);
    assert!(long.ends_with('\u{2026}') && long.matches('\u{2026}').count() == 1);
    // Multi-byte characters are cut on character boundaries, never in the middle.
    let wide = truncate_name(&"日本語".repeat(10));
    assert_eq!(wide.chars().count(), MAX_NAME_CHARS);
    assert_eq!(truncate_name(""), "");
}

#[test]
fn text_width_is_proportional_and_conservative() {
    assert!(close(text_width("", 10.5, false), 0.0));
    assert!(close(text_width("abcd", 10.0, false), 4.0 * 5.6));
    assert!(text_width("abcd", 10.0, false) < text_width("abcdabcd", 10.0, false));
    // Uppercase with letter spacing is wider than plain text of the same length.
    assert!(text_width("ABCD", 11.0, true) > text_width("abcd", 11.0, false));
    assert!(close(
        text_width("ABCD", 11.0, true),
        4.0 * (0.62 + 0.16) * 11.0
    ));
    // Wide scripts count a full em per character.
    assert!(close(text_width("日", 10.0, false), 10.0));
    assert!(text_width("日本", 10.0, false) > text_width("ab", 10.0, false));
}

#[test]
fn numbers_print_with_at_most_two_decimals() {
    assert_eq!(num(1.0), "1");
    assert_eq!(num(1.5), "1.5");
    assert_eq!(num(1.239), "1.24");
    assert_eq!(num(-0.001), "0");
    assert_eq!(num(0.0), "0");
    assert_eq!(num(1234.5), "1234.5");
    assert_eq!(num(100.0), "100");
    assert_eq!(num(-3.456), "-3.46");
    assert_eq!(num(f64::NAN), "0");
    assert_eq!(num(f64::INFINITY), "0");
    assert_eq!(num(f64::NEG_INFINITY), "0");
}

#[test]
fn a_name_must_be_clearly_nearer_its_own_stars() {
    let label = Rect {
        x0: 100.0,
        y0: 100.0,
        x1: 160.0,
        y1: 112.0,
    };
    let own = [(130.0, 130.0)]; // 18 px below the box
    assert!(
        name_belongs(&label, &own, &[(130.0, 300.0)]),
        "own is much nearer"
    );
    assert!(
        !name_belongs(&label, &own, &[(130.0, 125.0)]),
        "a foreign star is nearer"
    );
    assert!(
        !name_belongs(&label, &own, &[(130.0, 134.0)]),
        "nearly equal is ambiguous (needs the ratio)"
    );
    // Exactly at the ratio passes; just past it does not.
    let limit = 18.0 / NAME_OWNER_RATIO;
    assert!(name_belongs(&label, &own, &[(130.0, 112.0 + limit + 0.01)]));
    assert!(!name_belongs(&label, &own, &[(130.0, 112.0 + limit - 0.5)]));
    // With no other constellation there is nothing to confuse it with.
    assert!(name_belongs(&label, &own, &[]));
}
