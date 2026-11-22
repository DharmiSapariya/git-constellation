//! Star visibility on the paper theme (FIX 5): inked stars must stand out from the cream page.

mod common;

use common::{options_for, planned};
use git_constellation::art::{ArtOptions, sky_svg};
use git_constellation::export::svg_to_png;
use git_constellation::lint::star_visibility;
use git_constellation::model::{LabelMode, ThemeName};
use git_constellation::theme::Theme;

/// On paper at least this share of all stars, and of the tier 3 to 5 stars, must stand out.
const MIN_OVERALL: f64 = 0.85;
const MIN_BIG: f64 = 0.95;

/// Prints the shares too. Run: cargo test --test visibility -- --nocapture
#[test]
fn paper_star_visibility_shares() {
    for fixture in ["small", "medium", "large"] {
        let options = options_for(ThemeName::Paper);
        let (model, layout) = planned(fixture, &options);
        let theme = Theme::for_name(ThemeName::Paper);
        let svg = sky_svg(
            &model,
            &layout,
            &theme,
            &ArtOptions::default(),
            LabelMode::Major,
        );
        let png = svg_to_png(&svg, 1.0).expect("renders");
        let seen = star_visibility(&png, &model, &layout, false).expect("measures");
        println!(
            "paper {fixture:<6} overall {:5.1}% of {:3}   tiers 3-5 {:5.1}% of {:3}",
            seen.share * 100.0,
            seen.measured,
            seen.big_share * 100.0,
            seen.big_measured
        );
        assert!(
            seen.share >= MIN_OVERALL,
            "{fixture}: {:.1}% overall",
            seen.share * 100.0
        );
        assert!(
            seen.big_share >= MIN_BIG,
            "{fixture}: {:.1}% at tier 3+",
            seen.big_share * 100.0
        );
    }
}
