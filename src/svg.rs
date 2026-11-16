//! SvgBuilder: gradients, paths, escaping and number formatting.
//! (Only the shared number helper exists so far; the builder arrives in Milestone 4.)

/// Format a number with at most two decimals and no trailing zeros. This is
/// the one helper every SVG number goes through (4.9). A non-finite value
/// prints as `NaN` or `inf` on purpose, so the linter and tests can spot it.
pub fn num(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "inf" } else { "-inf" }.to_string();
    }
    let text = format!("{value:.2}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" || trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}
