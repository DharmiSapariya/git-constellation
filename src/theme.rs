//! Palettes, color helpers and language colors.

use crate::model::{LanguageKey, ThemeName};

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

/// A color is not `#rrggbb`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ColorError {
    #[error("`{0}` is not a color like #rrggbb")]
    Bad(String),
}

/// An 8-bit sRGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// Parse `#rrggbb` (any case).
pub fn parse_hex(text: &str) -> Result<Rgb, ColorError> {
    let bad = || ColorError::Bad(text.to_string());
    let digits = text.strip_prefix('#').ok_or_else(bad)?;
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(bad());
    }
    let channel = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).map_err(|_| bad());
    Ok(Rgb {
        r: channel(0)?,
        g: channel(2)?,
        b: channel(4)?,
    })
}

/// Blend `a` toward `b` by `t` (0 gives `a`, 1 gives `b`; `t` is clamped).
pub fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let blend = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
    Rgb {
        r: blend(a.r, b.r),
        g: blend(a.g, b.g),
        b: blend(a.b, b.b),
    }
}

/// Luma on the gamma-encoded 0..255 values, scaled to 0..1. This is what the
/// "how bright is the picture" check uses. It is NOT linearized: a night sky
/// measured in linear light is about 0.01 and would falsely fail.
pub fn luma(c: Rgb) -> f64 {
    (0.2126 * f64::from(c.r) + 0.7152 * f64::from(c.g) + 0.0722 * f64::from(c.b)) / 255.0
}

/// WCAG relative luminance (linearized), used for contrast ratios.
pub fn relative_luminance(c: Rgb) -> f64 {
    let linear = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}

/// Where the sky's middle color sits on the vertical gradient (a share of the height).
pub const SKY_MID_STOP: f64 = 0.55;

/// The sky's color at height `y` on a canvas `height` tall: the vertical
/// gradient on dark themes, the paper itself on light ones. Text halos use it
/// so they blend into whatever is behind them.
pub fn sky_color_at(theme: &Theme, y: f64, height: f64) -> Rgb {
    let parse = |hex: &str| parse_hex(hex).unwrap_or(Rgb { r: 0, g: 0, b: 0 });
    if !theme.is_dark {
        return parse(theme.background());
    }
    let share = if height > 0.0 {
        (y / height).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if share <= SKY_MID_STOP {
        mix(
            parse(theme.sky_top),
            parse(theme.sky_mid),
            share / SKY_MID_STOP,
        )
    } else {
        mix(
            parse(theme.sky_mid),
            parse(theme.sky_bottom),
            (share - SKY_MID_STOP) / (1.0 - SKY_MID_STOP),
        )
    }
}

/// WCAG contrast ratio, 1.0 to 21.0, whichever color is lighter.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

// ---------------------------------------------------------------------------
// Themes
// ---------------------------------------------------------------------------

/// How the sky's soft color clouds are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backdrop {
    /// Five glowing radial nebulae (dark themes).
    Nebula,
    /// Three soft watercolor blobs (paper).
    Watercolor,
    /// Nothing but the gradient (mono).
    Plain,
}

/// The two monochrome looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonoVariant {
    Dark,
    Light,
}

/// Everything about how a theme looks. Colors are `#rrggbb` text plus separate
/// opacities, so they can go straight into SVG.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: ThemeName,
    pub is_dark: bool,
    pub sky_top: &'static str,
    pub sky_mid: &'static str,
    pub sky_bottom: &'static str,
    /// Color and opacity of the glow along the bottom edge.
    pub horizon_glow: (&'static str, f64),
    pub backdrop: Backdrop,
    /// Three cloud colors with their opacities (nebula or watercolor blobs).
    pub nebula: [(&'static str, f64); 3],
    /// Edge color of the paper vignette, if any.
    pub vignette: Option<&'static str>,
    pub milkyway_core: &'static str,
    pub milkyway_fringe: &'static str,
    /// Colors of the band's tiny stars (core, fringe). Dark themes reuse the cloud
    /// colors; light themes need darker ones to show up on paper.
    pub milkyway_dots: (&'static str, &'static str),
    /// Scales the Milky Way's cloud opacity (1.0 is the standard strength).
    pub milkyway_strength: f64,
    pub line: (&'static str, f64),
    pub text: &'static str,
    pub text_dim: &'static str,
    pub ink: &'static str,
    /// The warm accent for small highlights (the HUD sparkle). Ink on light themes.
    pub accent: &'static str,
    pub dust_color: &'static str,
    pub dust_tint: &'static str,
    /// The four dust opacity buckets, faintest first.
    pub dust_opacities: [f64; 4],
}

const NIGHT_DUST: [f64; 4] = [0.25, 0.45, 0.65, 0.85];
const INK_DUST: [f64; 4] = [0.15, 0.22, 0.31, 0.40];

impl Theme {
    pub fn for_name(name: ThemeName) -> Theme {
        match name {
            ThemeName::Midnight => Theme {
                name,
                is_dark: true,
                sky_top: "#0b1026",
                sky_mid: "#151b3d",
                sky_bottom: "#2a2250",
                horizon_glow: ("#4b315f", 0.55),
                backdrop: Backdrop::Nebula,
                nebula: [("#6d4aa8", 0.22), ("#3a6ea5", 0.18), ("#c76fa8", 0.12)],
                vignette: None,
                milkyway_core: "#bfa8ff",
                milkyway_fringe: "#7fb6ff",
                milkyway_dots: ("#bfa8ff", "#7fb6ff"),
                milkyway_strength: 0.7,
                line: ("#cfd6ff", 0.38),
                text: "#e6e9ff",
                text_dim: "#aab2e0",
                ink: "#1a1633",
                accent: "#ffe08a",
                dust_color: "#e6e9ff",
                dust_tint: "#bfa8ff",
                dust_opacities: NIGHT_DUST,
            },
            ThemeName::Dusk => Theme {
                name,
                is_dark: true,
                sky_top: "#1b1233",
                sky_mid: "#4a2b5f",
                // Nudged twice from the spec's #b25f7a and #f0a070 at 0.35: light text must reach
                // 4.5:1 on what is really behind it (sky, glow, nebula and band together), and
                // on the spec's rose it only reached 2.5 to 3.9. Same hues, deeper.
                sky_bottom: "#7d4066",
                horizon_glow: ("#e8807f", 0.22),
                backdrop: Backdrop::Nebula,
                nebula: [("#d9608c", 0.20), ("#7a4fb5", 0.20), ("#f0a070", 0.10)],
                vignette: None,
                milkyway_core: "#ffc9dd",
                milkyway_fringe: "#c9a0ff",
                milkyway_dots: ("#ffc9dd", "#c9a0ff"),
                milkyway_strength: 1.5,
                line: ("#ffe3ee", 0.38),
                text: "#fff1f6",
                text_dim: "#f9dfeb",
                ink: "#2a1238",
                accent: "#ffe08a",
                dust_color: "#fff1f6",
                dust_tint: "#ffc9dd",
                dust_opacities: NIGHT_DUST,
            },
            ThemeName::Aurora => Theme {
                name,
                is_dark: true,
                sky_top: "#07141f",
                sky_mid: "#0e2a3a",
                sky_bottom: "#1b3d4f",
                horizon_glow: ("#2fd6a5", 0.25),
                backdrop: Backdrop::Nebula,
                nebula: [("#2fd6a5", 0.16), ("#7a5cff", 0.14), ("#3fa7ff", 0.10)],
                vignette: None,
                milkyway_core: "#a8ffe6",
                milkyway_fringe: "#8fb0ff",
                milkyway_dots: ("#a8ffe6", "#8fb0ff"),
                milkyway_strength: 0.75,
                line: ("#d8fff3", 0.38),
                text: "#e9fff9",
                text_dim: "#a9d9cc",
                ink: "#0a1a22",
                accent: "#ffe08a",
                dust_color: "#e9fff9",
                dust_tint: "#a8ffe6",
                dust_opacities: NIGHT_DUST,
            },
            ThemeName::Sakura => Theme {
                name,
                is_dark: true,
                sky_top: "#1a1030",
                sky_mid: "#3a1d4d",
                sky_bottom: "#6b2f63",
                horizon_glow: ("#ff8fc0", 0.30),
                backdrop: Backdrop::Nebula,
                nebula: [("#ff8fc0", 0.16), ("#b48aff", 0.14), ("#ffb48a", 0.08)],
                vignette: None,
                milkyway_core: "#ffd0e6",
                milkyway_fringe: "#c8a8ff",
                milkyway_dots: ("#ffd0e6", "#c8a8ff"),
                milkyway_strength: 1.0,
                line: ("#ffe0f0", 0.38),
                text: "#fff0f7",
                text_dim: "#f8d8e8",
                ink: "#2a1038",
                accent: "#ffe08a",
                dust_color: "#fff0f7",
                dust_tint: "#ffd0e6",
                dust_opacities: NIGHT_DUST,
            },
            ThemeName::Paper => Theme {
                name,
                is_dark: false,
                sky_top: "#f6efe2",
                sky_mid: "#f6efe2",
                sky_bottom: "#f6efe2",
                horizon_glow: ("#f6efe2", 0.0),
                backdrop: Backdrop::Watercolor,
                nebula: [("#c9b8e8", 0.45), ("#f3c6b8", 0.45), ("#b8d8e8", 0.45)],
                vignette: Some("#eadfca"),
                milkyway_core: "#cdb9ee",
                milkyway_fringe: "#a8cbe8",
                milkyway_dots: ("#8f78c4", "#6b97c4"),
                // Pale clouds vanish on cream, so paper's band is drawn much stronger.
                milkyway_strength: 3.0,
                line: ("#3a3350", 0.55),
                text: "#3a3350",
                text_dim: "#4f4968",
                ink: "#3a3350",
                accent: "#3a3350",
                dust_color: "#3a3350",
                dust_tint: "#6e6788",
                dust_opacities: INK_DUST,
            },
            ThemeName::Mono => Theme::mono(MonoVariant::Dark),
        }
    }

    /// The monochrome theme in its dark or light variant.
    pub fn mono(variant: MonoVariant) -> Theme {
        match variant {
            MonoVariant::Dark => Theme {
                name: ThemeName::Mono,
                is_dark: true,
                sky_top: "#0a0a0a",
                sky_mid: "#121212",
                // Nudged from #1c1c1c: with the softer Milky Way the empty sky averaged 0.076,
                // under the 0.08 floor for dark themes.
                sky_bottom: "#242424",
                horizon_glow: ("#242424", 0.0),
                backdrop: Backdrop::Plain,
                nebula: [("#8c8c8c", 0.0); 3],
                vignette: None,
                milkyway_core: "#d9d9d9",
                milkyway_fringe: "#8c8c8c",
                milkyway_dots: ("#d9d9d9", "#8c8c8c"),
                milkyway_strength: 0.7,
                line: ("#d9d9d9", 0.38),
                text: "#f2f2f2",
                text_dim: "#b5b5b5",
                ink: "#111111",
                accent: "#f2f2f2",
                dust_color: "#f2f2f2",
                dust_tint: "#d9d9d9",
                dust_opacities: NIGHT_DUST,
            },
            MonoVariant::Light => Theme {
                name: ThemeName::Mono,
                is_dark: false,
                sky_top: "#ffffff",
                sky_mid: "#ffffff",
                sky_bottom: "#ffffff",
                horizon_glow: ("#ffffff", 0.0),
                backdrop: Backdrop::Plain,
                nebula: [("#8c8c8c", 0.0); 3],
                vignette: None,
                milkyway_core: "#bdbdbd",
                milkyway_fringe: "#8c8c8c",
                milkyway_dots: ("#6e6e6e", "#4a4a4a"),
                milkyway_strength: 2.5,
                line: ("#111111", 0.55),
                text: "#111111",
                text_dim: "#555555",
                ink: "#111111",
                accent: "#111111",
                dust_color: "#111111",
                dust_tint: "#555555",
                dust_opacities: INK_DUST,
            },
        }
    }

    /// The color for a language group's stars.
    pub fn star_color(&self, key: LanguageKey) -> &'static str {
        language_color(key)
    }

    /// The flat background color used when contrast against the page matters:
    /// the bottom of the sky for dark themes, the paper itself for light ones.
    pub fn background(&self) -> &'static str {
        self.sky_bottom
    }
}

/// Star colors by language group (the same in every theme).
pub const LANGUAGE_COLORS: [(LanguageKey, &str); 10] = [
    (LanguageKey::Rust, "#ffb38a"),
    (LanguageKey::Python, "#8ec5ff"),
    (LanguageKey::JsTs, "#ffe08a"),
    (LanguageKey::Go, "#8ff0d0"),
    (LanguageKey::JavaKotlin, "#ff9a9a"),
    (LanguageKey::CFamily, "#c4a8ff"),
    (LanguageKey::HtmlCss, "#ffa8d8"),
    (LanguageKey::Shell, "#b8e986"),
    (LanguageKey::Jupyter, "#9ff5ff"),
    (LanguageKey::Other, "#f3f0ff"),
];

/// The color for a language group.
pub fn language_color(key: LanguageKey) -> &'static str {
    LANGUAGE_COLORS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or("#f3f0ff", |(_, color)| color)
}

/// Case-insensitive lookup by language name; unknown or missing is the "other" color.
pub fn color_for_language(language: Option<&str>) -> &'static str {
    language_color(LanguageKey::from_language(language))
}
