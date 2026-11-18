//! The Milky Way band: geometry, density, dot counts and keep-out areas.

mod common;

use common::load_fixture;
use git_constellation::art::milkyway::*;
use git_constellation::art::{ArtOptions, background_svg};
use git_constellation::model::{ThemeName, WeekDensity};
use git_constellation::theme::Theme;
use git_constellation::{Options, build_sky};

fn d(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn week(i: u32, total: u32, density: f64) -> WeekDensity {
    WeekDensity {
        week_start: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
            + chrono::Days::new(u64::from(i) * 7),
        total,
        density,
    }
}

fn weeks_with(f: impl Fn(u32) -> f64) -> Vec<WeekDensity> {
    (0..53).map(|i| week(i, 0, f(i))).collect()
}

#[test]
fn the_default_control_points_are_the_specified_ones() {
    assert_eq!(
        CONTROL_POINTS,
        [
            (-60.0, 545.0),
            (330.0, 520.0),
            (800.0, 300.0),
            (1260.0, 215.0)
        ]
    );
    let line = centerline(1200.0, 600.0, 200);
    assert_eq!(line.len(), 200);
    assert!(d(line[0], (-60.0, 545.0)) < 1e-9 && d(line[199], (1260.0, 215.0)) < 1e-9);
}

#[test]
fn the_band_scales_with_the_canvas() {
    let big = centerline(2400.0, 1200.0, 50);
    let small = centerline(1200.0, 600.0, 50);
    for (b, s) in big.iter().zip(&small) {
        assert!(d(*b, (s.0 * 2.0, s.1 * 2.0)) < 1e-9);
    }
    assert!((thickness(0.5, 1200.0) - 2.0 * thickness(0.5, 600.0)).abs() < 1e-9);
}

#[test]
fn the_band_runs_from_lower_left_to_upper_right() {
    let line = centerline(1200.0, 600.0, 200);
    assert!(
        line[0].1 > 500.0 && line[199].1 < 250.0,
        "starts low, ends high"
    );
    assert!(line[199].0 > line[0].0);
    // It never goes backward in x: a gentle S, not a loop.
    assert!(line.windows(2).all(|w| w[1].0 >= w[0].0));
}

#[test]
fn the_centerline_stays_clear_of_the_title_and_the_moon() {
    let line = centerline(1200.0, 600.0, 200);
    for p in &line {
        let dx = (24.0 - p.0).max(0.0).max(p.0 - 330.0);
        let dy = (24.0 - p.1).max(0.0).max(p.1 - 82.0);
        assert!(dx.hypot(dy) > 0.0, "{p:?} is inside the title rectangle");
        assert!(
            d(*p, (1090.0, 112.0)) >= 90.0 + 20.0,
            "{p:?} is within 20 px of the moon circle"
        );
    }
}

#[test]
fn thickness_follows_the_formula() {
    assert!((thickness(0.0, 600.0) - 40.0).abs() < 1e-6);
    assert!((thickness(1.0, 600.0) - 40.0).abs() < 1e-6);
    assert!((thickness(0.5, 600.0) - 170.0).abs() < 1e-6);
    let expected = 40.0 + 130.0 * (std::f64::consts::PI * 0.25).sin().powf(0.7);
    assert!((thickness(0.25, 600.0) - expected).abs() < 1e-9);
    assert!(
        thickness(0.5, 600.0) > thickness(0.2, 600.0)
            && thickness(0.2, 600.0) > thickness(0.05, 600.0)
    );
}

#[test]
fn bezier_points_and_tangents_are_consistent() {
    let p = CONTROL_POINTS;
    assert_eq!(point_at(&p, 0.0), p[0]);
    assert_eq!(point_at(&p, 1.0), p[3]);
    // The tangent points along the curve: a short step moves the same way.
    for t in [0.1, 0.4, 0.8] {
        let (tx, ty) = tangent_at(&p, t);
        let (a, b) = (point_at(&p, t), point_at(&p, t + 1e-5));
        assert!(((b.0 - a.0) * tx + (b.1 - a.1) * ty) > 0.0);
    }
}

#[test]
fn densities_follow_weekly_totals_with_a_floor() {
    let weeks = vec![
        week(0, 100, 1.0),
        week(1, 50, 0.5),
        week(2, 5, 0.05),
        week(3, 0, 0.0),
    ];
    let dens = densities(&weeks);
    assert_eq!(dens, vec![1.0, 0.5, 0.15, 0.15]);
    assert!(
        densities(&[week(0, 1, 7.0)])[0] <= 1.0,
        "out-of-range values are clamped"
    );
}

#[test]
fn a_year_with_no_contributions_is_a_faint_band_not_nothing() {
    let quiet = densities(&weeks_with(|_| 0.0));
    assert_eq!(quiet.len(), 53);
    assert!(quiet.iter().all(|x| (x - 0.15).abs() < 1e-12));
    assert_eq!(
        densities(&[]).len(),
        53,
        "even no weeks at all gives a band"
    );
    assert!(densities(&[]).iter().all(|x| (x - 0.15).abs() < 1e-12));
    let m = build_sky(&load_fixture("empty"), &Options::default()).unwrap();
    let svg = background_svg(
        &m,
        &Theme::for_name(ThemeName::Midnight),
        &ArtOptions::default(),
        &[],
    );
    assert!(
        svg.contains("<g id=\"milkyway\">") && svg.matches("<ellipse").count() >= 106,
        "the empty fixture still draws the band"
    );
}

#[test]
fn busy_weeks_get_more_stars_than_quiet_ones() {
    let dens = densities(&weeks_with(|i| if i == 20 { 1.0 } else { 0.0 }));
    let counts = dot_counts(&dens);
    assert_eq!(counts.len(), 53);
    assert_eq!(counts[20], 16, "3 + 13 * 1.0");
    assert_eq!(counts[0], 5, "round(3 + 13 * 0.15)");
    assert!(counts[20] > counts[19] * 2);
    let half = dot_counts(&[0.5]);
    assert_eq!(half[0], 10, "round(3 + 13 * 0.5)");
}

#[test]
fn the_total_never_passes_700_even_for_the_busiest_year() {
    let busy = dot_counts(&densities(&weeks_with(|_| 1.0)));
    let total: usize = busy.iter().sum();
    assert!(total <= 700, "{total}");
    assert!(total > 600, "scaled down together, not wiped out: {total}");
    assert!(busy.iter().all(|n| *n >= 3));
    assert!(
        busy.windows(2).all(|w| w[0] == w[1]),
        "all-busy weeks stay equal after scaling"
    );
    let quiet: usize = dot_counts(&densities(&weeks_with(|_| 0.0))).iter().sum();
    assert_eq!(quiet, 53 * 5);
    // An uneven year is scaled proportionally, so the busy weeks stay busier.
    let mixed = dot_counts(&densities(&weeks_with(
        |i| if i % 2 == 0 { 1.0 } else { 0.4 },
    )));
    assert!(mixed.iter().sum::<usize>() <= 700 && mixed[0] > mixed[1]);
}

#[test]
fn the_clouds_are_two_per_segment_and_follow_the_opacity_formula() {
    let dens = densities(&weeks_with(|i| f64::from(i) / 52.0));
    let clouds = band_clouds(5, 1200.0, 600.0, &dens, 1.0);
    assert_eq!(clouds.len(), 106);
    for (i, pair) in clouds.chunks(2).enumerate() {
        assert!(
            pair[0].core && !pair[1].core,
            "a core cloud then a fringe cloud"
        );
        for c in pair {
            let want = 0.04 + 0.06 * dens[i];
            assert!((c.opacity - want).abs() < 1e-9, "segment {i}");
            assert!(
                c.rx > 0.0
                    && c.ry > 0.0
                    && c.cx.is_finite()
                    && c.cy.is_finite()
                    && c.angle_deg.is_finite()
            );
        }
    }
    let stronger = band_clouds(5, 1200.0, 600.0, &dens, 2.0);
    assert!(
        (stronger[10].opacity - 2.0 * clouds[10].opacity).abs() < 1e-9,
        "theme strength multiplies opacity"
    );
    let huge = band_clouds(5, 1200.0, 600.0, &dens, 1000.0);
    assert!(huge.iter().all(|c| c.opacity <= 1.0), "opacity is clamped");
}

#[test]
fn the_clouds_follow_the_band_and_the_tangent() {
    let dens = densities(&[]);
    let clouds = band_clouds(9, 1200.0, 600.0, &dens, 1.0);
    let line = centerline(1200.0, 600.0, 400);
    for c in &clouds {
        let nearest = line
            .iter()
            .map(|p| d(*p, (c.cx, c.cy)))
            .fold(f64::INFINITY, f64::min);
        assert!(
            nearest < 0.25 * 170.0 + 8.0,
            "a cloud is {nearest:.0}px off the band"
        );
        assert!(
            (-90.0..=90.0).contains(&c.angle_deg),
            "rotated to the direction of travel"
        );
    }
    assert_eq!(clouds, band_clouds(9, 1200.0, 600.0, &dens, 1.0));
    assert_ne!(clouds, band_clouds(10, 1200.0, 600.0, &dens, 1.0));
}

#[test]
fn the_tiny_stars_are_few_small_and_in_the_band() {
    let busy = densities(&weeks_with(|_| 1.0));
    let dots = band_dots(3, 1200.0, 600.0, &busy);
    assert!(dots.len() <= 700 && dots.len() > 500, "{}", dots.len());
    let line = centerline(1200.0, 600.0, 600);
    for dot in &dots {
        assert!((0.4..=1.0).contains(&dot.r) && dot.bucket < 4);
        assert!(dot.x.is_finite() && dot.y.is_finite());
        let nearest = line
            .iter()
            .map(|p| d(*p, (dot.x, dot.y)))
            .fold(f64::INFINITY, f64::min);
        assert!(
            nearest <= 90.0,
            "a star is {nearest:.0}px from the centerline"
        );
    }
    assert_eq!(dots, band_dots(3, 1200.0, 600.0, &busy));
    // Most stars sit near the middle of the band (roughly Gaussian).
    let core = dots.iter().filter(|d| d.core).count();
    assert!(core * 2 > dots.len(), "core {core} of {}", dots.len());
}

#[test]
fn a_busy_week_really_has_more_stars_drawn_than_a_quiet_one() {
    let dens = densities(&weeks_with(|i| if i == 26 { 1.0 } else { 0.0 }));
    let dots = band_dots(4, 1200.0, 600.0, &dens);
    let p = [
        (-60.0, 545.0),
        (330.0, 520.0),
        (800.0, 300.0),
        (1260.0, 215.0),
    ];
    let near_week = |index: usize| {
        let (t0, t1) = (index as f64 / 53.0, (index + 1) as f64 / 53.0);
        let (a, b) = (point_at(&p, t0), point_at(&p, t1));
        dots.iter()
            .filter(|dot| dot.x >= a.0.min(b.0) - 60.0 && dot.x <= a.0.max(b.0) + 60.0)
            .count()
    };
    assert!(
        near_week(26) > near_week(10) + 5,
        "{} vs {}",
        near_week(26),
        near_week(10)
    );
}

#[test]
fn the_drawn_layer_uses_shared_gradients_and_grouped_dots() {
    let m = build_sky(&load_fixture("medium"), &Options::default()).unwrap();
    let svg = background_svg(
        &m,
        &Theme::for_name(ThemeName::Midnight),
        &ArtOptions::default(),
        &[],
    );
    let band = &svg[svg.find("<g id=\"milkyway\">").unwrap()..];
    assert_eq!(band.matches("<ellipse").count(), 106);
    let gradients: std::collections::BTreeSet<&str> = band
        .split("fill=\"url(#")
        .skip(1)
        .map(|r| r.split(')').next().unwrap_or(""))
        .collect();
    assert_eq!(
        gradients.len(),
        2,
        "one shared gradient each for core and fringe"
    );
    assert!(band.matches("<circle").count() <= 700);
    assert!(
        band.lines()
            .filter(|l| l.starts_with("<circle"))
            .all(|l| l.matches('=').count() == 3)
    );
}
