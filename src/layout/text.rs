//! Text measuring: font sizes, conservative width estimates, repo-name cutting.

/// Font sizes in pixels (2.5).
pub const CONSTELLATION_FONT: f64 = 11.0;
pub const REPO_FONT: f64 = 10.5;
/// The "first light" note: as large as repo names (9 px was under the 10 px readability floor).
pub const SMALL_FONT: f64 = 10.5;
/// Average glyph width as a share of font size, deliberately a little high.
pub const WIDTH_NORMAL: f64 = 0.56;
pub const WIDTH_UPPERCASE: f64 = 0.62;
/// Extra spacing between letters in constellation names, in font sizes.
pub const LETTER_SPACING: f64 = 0.16;
/// Line height as a share of font size, and padding around every box.
pub const LINE_HEIGHT: f64 = 1.3;
pub const BOX_PAD: f64 = 2.0;
/// Repo names longer than this are cut with an ellipsis.
pub const MAX_NAME_CHARS: usize = 18;
/// Cut `text` to at most `max_chars` characters (never bytes), ending in a
/// single ellipsis if it was shortened.
pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut short: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    short.push('\u{2026}');
    short
}

/// Cut a repo name to 18 characters, ending in a single ellipsis if shortened.
pub fn truncate_name(name: &str) -> String {
    truncate_chars(name, MAX_NAME_CHARS)
}

/// Conservative text width. Characters outside ASCII (wide scripts, accents)
/// count as a full font size each.
pub fn text_width(text: &str, font_size: f64, uppercase_with_spacing: bool) -> f64 {
    text.chars()
        .map(|c| {
            let glyph = if !c.is_ascii() {
                1.0
            } else if uppercase_with_spacing {
                WIDTH_UPPERCASE
            } else {
                WIDTH_NORMAL
            };
            (glyph
                + if uppercase_with_spacing {
                    LETTER_SPACING
                } else {
                    0.0
                })
                * font_size
        })
        .sum()
}
