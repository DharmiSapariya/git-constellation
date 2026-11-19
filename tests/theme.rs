//! Themes and color helpers.

use git_constellation::model::{LanguageKey, ThemeName};
use git_constellation::svg::is_hex_color;
use git_constellation::theme::*;

const ALL: [ThemeName; 6] = [
    ThemeName::Midnight,
    ThemeName::Dusk,
    ThemeName::Aurora,
    ThemeName::Sakura,
    ThemeName::Paper,
    ThemeName::Mono,
];

fn every_theme() -> Vec<Theme> {
    let mut themes: Vec<Theme> = ALL.iter().map(|n| Theme::for_name(*n)).collect();
    themes.push(Theme::mono(MonoVariant::Light));
    themes
}

fn hexes(t: &Theme) -> Vec<&'static str> {
    let mut all = vec![
        t.sky_top,
        t.sky_mid,
        t.sky_bottom,
        t.horizon_glow.0,
        t.milkyway_core,
        t.milkyway_fringe,
        t.line.0,
        t.text,
        t.text_dim,
        t.ink,
        t.dust_color,
        t.dust_tint,
    ];
    all.extend(t.nebula.iter().map(|(c, _)| *c));
    all.extend(t.vignette);
    all
}

#[test]
fn parse_hex_accepts_colors_and_rejects_everything_else() {
    assert_eq!(
        parse_hex("#0b1026"),
        Ok(Rgb {
            r: 11,
            g: 16,
            b: 38
        })
    );
    assert_eq!(
        parse_hex("#FFFFFF"),
        Ok(Rgb {
            r: 255,
            g: 255,
            b: 255
        })
    );
    for bad in [
        "",
        "#",
        "#fff",
        "0b1026",
        "#0b102",
        "#0b10266",
        "#gg0000",
        "rgb(1,2,3)",
        " #000000",
        "#00000é",
    ] {
        assert_eq!(
            parse_hex(bad),
            Err(ColorError::Bad(bad.to_string())),
            "{bad:?}"
        );
    }
    assert!(ColorError::Bad("x".into()).to_string().contains("#rrggbb"));
    assert_eq!(
        Rgb {
            r: 11,
            g: 16,
            b: 38
        }
        .to_hex(),
        "#0b1026"
    );
}

#[test]
fn mixing_blends_and_clamps() {
    let (black, white) = (
        Rgb { r: 0, g: 0, b: 0 },
        Rgb {
            r: 255,
            g: 255,
            b: 255,
        },
    );
    assert_eq!(mix(black, white, 0.0), black);
    assert_eq!(mix(black, white, 1.0), white);
    assert_eq!(
        mix(black, white, 0.5),
        Rgb {
            r: 128,
            g: 128,
            b: 128
        }
    );
    assert_eq!(mix(black, white, -3.0), black);
    assert_eq!(mix(black, white, 9.0), white);
    assert_eq!(mix(black, white, f64::NAN), black);
    assert_eq!(
        mix(
            Rgb {
                r: 10,
                g: 200,
                b: 50
            },
            Rgb {
                r: 30,
                g: 100,
                b: 250
            },
            0.5
        ),
        Rgb {
            r: 20,
            g: 150,
            b: 150
        }
    );
}

#[test]
fn luma_is_gamma_encoded_and_contrast_is_wcag() {
    let (black, white) = (
        Rgb { r: 0, g: 0, b: 0 },
        Rgb {
            r: 255,
            g: 255,
            b: 255,
        },
    );
    assert_eq!(luma(black), 0.0);
    assert!((luma(white) - 1.0).abs() < 1e-12);
    // Mid-gray: encoded luma is 0.5, linear light is much lower.
    let gray = Rgb {
        r: 128,
        g: 128,
        b: 128,
    };
    assert!((luma(gray) - 128.0 / 255.0).abs() < 1e-12);
    assert!(relative_luminance(gray) < 0.25);
    assert!((contrast_ratio(black, white) - 21.0).abs() < 1e-9);
    assert!(
        (contrast_ratio(white, black) - 21.0).abs() < 1e-9,
        "order does not matter"
    );
    assert!((contrast_ratio(gray, gray) - 1.0).abs() < 1e-12);
    // A well-known value: #767676 on white is just over 4.5:1.
    let ratio = contrast_ratio(
        Rgb {
            r: 0x76,
            g: 0x76,
            b: 0x76,
        },
        white,
    );
    assert!((ratio - 4.54).abs() < 0.01, "{ratio}");
}

#[test]
fn every_theme_builds_and_every_color_is_valid() {
    for t in every_theme() {
        for h in hexes(&t) {
            assert!(is_hex_color(h) && parse_hex(h).is_ok(), "{:?}: {h}", t.name);
        }
        for opacity in [t.horizon_glow.1, t.line.1]
            .into_iter()
            .chain(t.nebula.iter().map(|(_, o)| *o))
            .chain(t.dust_opacities)
        {
            assert!(
                (0.0..=1.0).contains(&opacity),
                "{:?}: opacity {opacity}",
                t.name
            );
        }
        assert!(
            t.milkyway_strength > 0.0 && t.milkyway_strength <= 4.0,
            "{:?}: band strength",
            t.name
        );
        assert!(
            t.dust_opacities.windows(2).all(|w| w[0] < w[1]),
            "{:?}: dust buckets rise",
            t.name
        );
    }
}

#[test]
fn the_exact_palette_values_are_pinned() {
    let m = Theme::for_name(ThemeName::Midnight);
    assert_eq!(
        (m.sky_top, m.sky_mid, m.sky_bottom),
        ("#0b1026", "#151b3d", "#2a2250")
    );
    assert_eq!(m.horizon_glow, ("#4b315f", 0.55));
    assert_eq!(
        m.nebula,
        [("#6d4aa8", 0.22), ("#3a6ea5", 0.18), ("#c76fa8", 0.12)]
    );
    assert_eq!(
        (m.milkyway_core, m.milkyway_fringe, m.line),
        ("#bfa8ff", "#7fb6ff", ("#cfd6ff", 0.38))
    );
    assert_eq!(
        (m.text, m.text_dim, m.ink, m.dust_color, m.dust_tint),
        ("#e6e9ff", "#aab2e0", "#1a1633", "#e6e9ff", "#bfa8ff")
    );

    let d = Theme::for_name(ThemeName::Dusk);
    assert_eq!((d.sky_top, d.sky_mid), ("#1b1233", "#4a2b5f"));
    // Nudged from the spec's #b25f7a and #f0a070 at 0.35: light text must reach 4.5:1 on the
    // real picture (sky, glow, nebula and band together), and on the spec's rose it did not.
    assert_eq!(d.sky_bottom, "#7d4066");
    assert_eq!(d.horizon_glow, ("#e8807f", 0.22));
    assert_eq!(
        d.nebula,
        [("#d9608c", 0.20), ("#7a4fb5", 0.20), ("#f0a070", 0.10)]
    );

    let a = Theme::for_name(ThemeName::Aurora);
    assert_eq!(
        (a.sky_top, a.sky_mid, a.sky_bottom),
        ("#07141f", "#0e2a3a", "#1b3d4f")
    );
    assert_eq!(a.horizon_glow, ("#2fd6a5", 0.25));

    let s = Theme::for_name(ThemeName::Sakura);
    assert_eq!(
        (s.sky_top, s.sky_mid, s.sky_bottom),
        ("#1a1030", "#3a1d4d", "#6b2f63")
    );

    let p = Theme::for_name(ThemeName::Paper);
    assert!(!p.is_dark);
    assert_eq!(p.sky_top, "#f6efe2");
    assert_eq!(p.vignette, Some("#eadfca"));
    assert_eq!(
        p.nebula.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        ["#c9b8e8", "#f3c6b8", "#b8d8e8"]
    );
    assert!(
        p.nebula.iter().all(|(_, o)| (0.35..=0.50).contains(o)),
        "watercolor blobs are strong enough to register on cream"
    );
    assert_eq!(
        (p.milkyway_core, p.milkyway_fringe, p.line),
        ("#cdb9ee", "#a8cbe8", ("#3a3350", 0.55))
    );
    assert_eq!(p.dust_opacities[0], 0.15);
    assert_eq!(p.dust_opacities[3], 0.40);

    let dark = Theme::mono(MonoVariant::Dark);
    assert_eq!(
        (dark.sky_top, dark.sky_mid, dark.sky_bottom),
        ("#0a0a0a", "#121212", "#242424")
    );
    assert_eq!(
        (dark.milkyway_core, dark.milkyway_fringe, dark.text),
        ("#d9d9d9", "#8c8c8c", "#f2f2f2")
    );
    let light = Theme::mono(MonoVariant::Light);
    assert_eq!(
        (
            light.sky_top,
            light.ink,
            light.milkyway_core,
            light.milkyway_fringe,
            light.text
        ),
        ("#ffffff", "#111111", "#bdbdbd", "#8c8c8c", "#111111")
    );
    assert!(!light.is_dark && dark.is_dark);
    assert_eq!(
        Theme::for_name(ThemeName::Mono),
        dark,
        "plain `mono` means the dark variant"
    );
}

#[test]
fn text_contrast_is_at_least_4_5_on_every_theme() {
    for t in every_theme() {
        let text = parse_hex(t.text).unwrap();
        let ground = parse_hex(t.background()).unwrap();
        let ratio = contrast_ratio(text, ground);
        assert!(
            ratio >= 4.5,
            "{:?} ({}): text {} on {} is only {ratio:.2}:1",
            t.name,
            if t.is_dark { "dark" } else { "light" },
            t.text,
            t.background()
        );
    }
}

#[test]
fn dim_text_is_still_readable_on_the_themes_flat_color() {
    for t in every_theme() {
        let ratio = contrast_ratio(
            parse_hex(t.text_dim).unwrap(),
            parse_hex(t.sky_top).unwrap(),
        );
        assert!(
            ratio >= 4.5,
            "{:?}: dim text on the top of the sky is {ratio:.2}:1",
            t.name
        );
    }
}

#[test]
fn language_colors_match_the_table() {
    let cases = [
        ("Rust", "#ffb38a"),
        ("Python", "#8ec5ff"),
        ("JavaScript", "#ffe08a"),
        ("TypeScript", "#ffe08a"),
        ("Go", "#8ff0d0"),
        ("Java", "#ff9a9a"),
        ("Kotlin", "#ff9a9a"),
        ("C", "#c4a8ff"),
        ("C++", "#c4a8ff"),
        ("C#", "#c4a8ff"),
        ("HTML", "#ffa8d8"),
        ("CSS", "#ffa8d8"),
        ("Shell", "#b8e986"),
        ("Makefile", "#b8e986"),
        ("Jupyter Notebook", "#9ff5ff"),
        ("R", "#9ff5ff"),
        ("Brainfuck", "#f3f0ff"),
        ("", "#f3f0ff"),
    ];
    for (language, color) in cases {
        assert_eq!(color_for_language(Some(language)), color, "{language}");
        assert_eq!(
            color_for_language(Some(&language.to_uppercase())),
            color,
            "{language} in capitals"
        );
        assert_eq!(
            color_for_language(Some(&language.to_lowercase())),
            color,
            "{language} in lowercase"
        );
    }
    assert_eq!(color_for_language(None), "#f3f0ff");
    assert_eq!(LANGUAGE_COLORS.len(), 10);
    for key in [LanguageKey::Rust, LanguageKey::Other] {
        assert_eq!(
            Theme::for_name(ThemeName::Midnight).star_color(key),
            language_color(key)
        );
    }
    assert!(LANGUAGE_COLORS.iter().all(|(_, c)| is_hex_color(c)));
}

#[test]
fn night_themes_are_darker_than_the_paper_theme() {
    let sky = |name: ThemeName| luma(parse_hex(Theme::for_name(name).sky_mid).unwrap());
    for dark in [
        ThemeName::Midnight,
        ThemeName::Dusk,
        ThemeName::Aurora,
        ThemeName::Sakura,
    ] {
        assert!(sky(dark) < 0.4 && sky(dark) < sky(ThemeName::Paper));
    }
}
